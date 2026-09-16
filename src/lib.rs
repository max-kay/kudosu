use std::ffi::CString;
use std::io::Read;
use std::time::Duration;

use android_activity::{
    AndroidApp, InputStatus, MainEvent, PollEvent,
    input::{InputEvent, MotionAction},
    ndk::{hardware_buffer_format::HardwareBufferFormat, native_window::NativeWindow},
};
use log::{LevelFilter, info, warn};

const NAME: &str = "Kudosu";

mod insets;
pub use insets::{Inset, InsetKind, get_insets};

mod layout;
use layout::{Layout, Palette};
use tiny_skia::{FillRule, PathBuilder, Rect, Transform};

mod sudoku_types;
pub use sudoku_types::{GridPosition, NineGrid, Number, NumberBucket, PositionBucket};
use ttf_parser::{Face, OutlineBuilder};

use crate::sudoku_types::Sudoku;

struct SkiaOutlineBuilder(PathBuilder);

impl OutlineBuilder for SkiaOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.quad_to(cx, cy, x, y);
    }
    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        self.0.cubic_to(cx1, cy1, cx2, cy2, x, y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

#[derive(Copy, Clone)]
pub enum SelectionMode {
    New,
    WithOld,
    Add,
    Clear,
}

pub enum DrawState {
    Ok,
    NeedsRelayout,
    NeedsRedraw,
}

impl DrawState {
    pub fn clear(&mut self) {
        *self = DrawState::Ok;
    }

    pub fn relayout(&mut self) {
        *self = DrawState::NeedsRelayout;
    }

    pub fn redraw(&mut self) {
        if let Self::NeedsRelayout = self {
            return;
        } else {
            *self = Self::NeedsRedraw;
        }
    }
}

pub struct State {
    running: bool,
    app: AndroidApp,
    layout: Layout,
    palette: Palette,
    sudoku: Box<Sudoku>,
    sel_mode: SelectionMode,
    selection: PositionBucket,
    draw_state: DrawState,
    face: Face<'static>,
}

impl State {
    pub fn running(&self) -> bool {
        self.running
    }

    pub fn handle_grid_input(&mut self, pos: GridPosition, action: MotionAction) {
        match action {
            MotionAction::Down | MotionAction::Move => match self.sel_mode {
                SelectionMode::New => {
                    if self.selection.count() == 1 && self.selection.contains(pos) {
                        self.selection = PositionBucket::new();
                        return;
                    }
                    self.selection = pos.into();
                    self.sel_mode = SelectionMode::Add;
                }
                SelectionMode::WithOld => {
                    if self.selection.contains(pos) {
                        self.selection.remove(pos);
                        self.sel_mode = SelectionMode::Clear;
                    } else {
                        self.selection.insert(pos);
                        self.sel_mode = SelectionMode::Add;
                    }
                }
                SelectionMode::Add => self.selection.insert(pos),
                SelectionMode::Clear => self.selection.remove(pos),
            },
            MotionAction::Up => self.sel_mode = SelectionMode::New,
            _ => info!(
                "marked motion action: `{:?}` as handled without change",
                action
            ),
        }
    }

    pub fn handle_input(&mut self, input: &InputEvent) -> InputStatus {
        if let InputEvent::MotionEvent(motion_event) = input {
            let pointer = motion_event.pointer_at_index(0);
            let hit = self.layout.hit(pointer.x(), pointer.y());
            match hit {
                layout::Hit::Cell(pos) => self.handle_grid_input(pos, motion_event.action()),
                layout::Hit::Button(button) => todo!(),
                layout::Hit::None => match self.sel_mode {
                    SelectionMode::New | SelectionMode::WithOld => {
                        self.selection = PositionBucket::new()
                    }
                    SelectionMode::Add | SelectionMode::Clear => (),
                },
            }
            self.draw_state.redraw();
            InputStatus::Handled
        } else {
            InputStatus::Unhandled
        }
    }

    pub fn new(app: AndroidApp, face: Face<'static>) -> Self {
        Self {
            running: true,
            app,
            layout: Default::default(),
            palette: Default::default(),
            sudoku: Default::default(),
            sel_mode: SelectionMode::New,
            selection: PositionBucket::box_(5),
            face,
            draw_state: DrawState::NeedsRelayout,
        }
    }

    fn event_callback(&mut self, event: PollEvent) {
        let main_event = match event {
            PollEvent::Wake => {
                info!("Wake");
                return;
            }
            PollEvent::Timeout => return,
            PollEvent::Main(main_event) => main_event,
            _ => return, // TODO
        };

        match main_event {
            MainEvent::InitWindow { .. } => {
                info!("Window initialized");
                if let Some(window) = self.app.native_window() {
                    window
                        .set_buffers_geometry(0, 0, Some(HardwareBufferFormat::R8G8B8A8_UNORM))
                        .ok();
                }
                self.draw_state.relayout();
            }
            MainEvent::TerminateWindow { .. } => {
                info!("Window terminated");
            }
            MainEvent::WindowResized { .. } => {
                info!("WindowResized");
                self.draw_state.relayout();
            }
            MainEvent::RedrawNeeded { .. } => {
                self.draw_state.redraw();
            }
            MainEvent::InputAvailable => {
                info!("InputAvailable");
                if let Ok(mut iter) = self.app.clone().input_events_iter() {
                    while iter.next(|i| self.handle_input(i)) {}
                }
            }
            MainEvent::Destroy => {
                info!("App shutting down");
                self.running = false;
            }
            MainEvent::ContentRectChanged { .. } => {
                info!("ContentRectChanged");
                self.draw_state.relayout();
            }
            MainEvent::GainedFocus => {
                info!("GainedFocus");
            }
            MainEvent::LostFocus => {
                info!("LostFocus");
            }
            MainEvent::ConfigChanged { .. } => {
                info!("ConfigChanged");
            }
            MainEvent::LowMemory => warn!("running low on memory"),
            MainEvent::Start => info!("Start"),
            MainEvent::Resume { loader, .. } => {
                info!("Resume")
            }
            MainEvent::SaveState { saver, .. } => {
                info!("SaveState")
            }
            MainEvent::Pause => {
                info!("Pause")
            }
            MainEvent::Stop => {
                info!("Stop")
            }
            MainEvent::InsetsChanged { .. } => {
                info!("InsetsChanged");
                self.draw_state.relayout();
            }
            _ => {
                info!("Unknown MainEvent")
            }
        }

        if let Some(window) = self.app.native_window() {
            match self.draw_state {
                DrawState::Ok => (),
                DrawState::NeedsRelayout => {
                    let window_rect =
                        Rect::from_xywh(0.0, 0.0, window.width() as f32, window.height() as f32)
                            .expect("valid window");
                    self.layout = Layout::new(window_rect, &get_insets(&self.app));
                    self.render_frame(&window);
                }
                DrawState::NeedsRedraw => {
                    self.render_frame(&window);
                }
            }
            self.draw_state.clear();
        } else {
            warn!("expected to have window but had none")
        }
    }

    pub fn render_frame(&mut self, window: &NativeWindow) {
        // TODO dont spin wait
        let mut lock = loop {
            // TODO make redraw region
            if let Ok(lock) = window.lock(None) {
                break lock;
            }
        };

        let bytes = lock.bytes().unwrap();
        let mut pixmap = tiny_skia::PixmapMut::from_bytes(
            unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr() as _, bytes.len()) },
            lock.stride() as u32,
            lock.height() as u32,
        )
        .unwrap();

        pixmap.fill(self.palette.background);
        pixmap.fill_rect(
            self.layout.get_grid_rect(),
            &self.palette.grid_background_paint(),
            Transform::identity(),
            None,
        );

        let highlight = if self.selection.empty() {
            PositionBucket::new()
        } else {
            let mut hl = PositionBucket::all();
            for pos in self.selection.into_iter() {
                hl = hl & pos.sees_by_sudoku();
            }
            hl
        };

        for pos in PositionBucket::all().into_iter() {
            let number = self.sudoku.get(pos).solution.as_char();

            const CELL_MARGIN: f32 = 0.1;
            let rect = self.layout.get_rect(pos);

            if self.selection.contains(pos) {
                pixmap.fill_rect(
                    rect,
                    &self.palette.selection_paint(),
                    Transform::identity(),
                    None,
                );
            } else if highlight.contains(pos) {
                pixmap.fill_rect(
                    rect,
                    &self.palette.highlight_paint(),
                    Transform::identity(),
                    None,
                );
            }

            // TODO is this worth caching?
            let glyph_id = self.face.glyph_index(number).expect("every digit is here");
            let mut builder = SkiaOutlineBuilder(PathBuilder::new());
            self.face.outline_glyph(glyph_id, &mut builder);
            let path = builder.0.finish().expect("path should always be valid");

            let font_height = rect.height() * (1.0 - 2.0 * CELL_MARGIN);
            let scale = font_height / self.face.height() as f32;
            let center_y = rect.top() + rect.height() / 2.0;
            let center_x = rect.left() + rect.width() / 2.0;
            let cap_height = self
                .face
                .capital_height()
                .map(|h| h as f32)
                .unwrap_or_else(|| self.face.ascender() as f32);

            let font_center_y = cap_height / 2.0;
            let font_center_x = self.face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 / 2.0;

            let transform = Transform::from_scale(scale, -scale).post_translate(
                center_x - font_center_x * scale,
                center_y + font_center_y * scale,
            );
            let paint = self.palette.text_paint();
            pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
        }

        self.layout.draw_grid(&self.palette, &mut pixmap);
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    // TODO remove after testing
    sudoku_types::test::number_bucket();
    sudoku_types::test::pos_bucket();

    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(LevelFilter::Info)
            .with_tag(NAME),
    );

    // app.set_window_flags(
    //     WindowManagerFlags::LAYOUT_IN_SCREEN | WindowManagerFlags::FULLSCREEN, // TODO not done yet
    //     WindowManagerFlags::empty(),
    // );
    let asset_mgr = app.asset_manager();
    let path = CString::new("Libre_Baskerville/static/LibreBaskerville-Medium.ttf").unwrap();

    let mut asset = asset_mgr.open(&path).expect("font was not available");

    let mut font_bytes = Vec::new();
    asset
        .read_to_end(&mut font_bytes)
        .expect("could not get asset buffer");
    let font_bytes = font_bytes.leak(); // TODO doesn't seem rusty
    let face = ttf_parser::Face::parse(font_bytes, 0).expect("could not parse font");
    info!("Kudosu Started!");
    let mut state = State::new(app.clone(), face);

    while state.running() {
        // Redraw on events or poll interval
        app.poll_events(Some(Duration::from_millis(16)), |event| {
            state.event_callback(event)
        });
    }
}
