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
const CELL_MARGIN: f32 = 0.2; // TODO remove

mod insets;
pub use insets::{Inset, InsetKind, get_insets};

mod layout;
pub use layout::{IntoPaint, Layout, Palette};
use tiny_skia::{FillRule, Path, PathBuilder, PixmapMut, Rect, Transform};

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

enum InputMode {
    Solve,
    Center,
    Corner,
    Color,
}

struct ButtonState {
    clear_on_new_selection: bool,
    input_mode: InputMode,
    secondary_input: InputMode,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self {
            clear_on_new_selection: true,
            input_mode: InputMode::Solve,
            secondary_input: InputMode::Center,
        }
    }
}

pub struct State {
    running: bool,
    app: AndroidApp,
    layout: Option<Layout>,
    palette: Palette,
    sudoku: Box<Sudoku>,
    sel_mode: SelectionMode,
    selection: PositionBucket,
    draw_state: DrawState,
    button_state: ButtonState,
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
            MotionAction::Up => {
                self.sel_mode = if self.button_state.clear_on_new_selection {
                    SelectionMode::New
                } else {
                    SelectionMode::WithOld
                }
            }
            _ => info!(
                "marked motion action: `{:?}` as handled without change",
                action
            ),
        }
    }

    pub fn undo(&mut self) {
        // TODO
    }

    pub fn redo(&mut self) {
        // TODO
    }

    pub fn handle_delete(&mut self) {
        let all_center = self.sudoku.selection_or_all_center_notes(self.selection);
        let all_corner = self.sudoku.selection_or_all_corner_notes(self.selection);
        match self.button_state.input_mode {
            InputMode::Center => {
                if !all_center.empty() {
                    for pos in self.selection.into_iter() {
                        self.sudoku.get_mut(pos).center_notes.clear();
                    }
                    return;
                };
                if !all_corner.empty() {
                    for pos in self.selection.into_iter() {
                        self.sudoku.get_mut(pos).corner_notes.clear();
                    }
                    return;
                };
                for pos in self.selection.into_iter() {
                    *self.sudoku.get_mut(pos).solved_number = None;
                }
            }
            InputMode::Solve | InputMode::Corner => {
                if !all_corner.empty() {
                    for pos in self.selection.into_iter() {
                        self.sudoku.get_mut(pos).corner_notes.clear();
                    }
                    return;
                };
                if !all_center.empty() {
                    for pos in self.selection.into_iter() {
                        self.sudoku.get_mut(pos).center_notes.clear();
                    }
                    return;
                };
                for pos in self.selection.into_iter() {
                    *self.sudoku.get_mut(pos).solved_number = None;
                }
            }
            InputMode::Color => (),
        }
    }

    fn handle_number_input_corner(&mut self, num: Number) {
        if self
            .sudoku
            .selection_and_all_corner_notes(self.selection)
            .contains(num)
        {
            for pos in self.selection.into_iter() {
                self.sudoku.get_mut(pos).corner_notes.remove(num);
            }
        } else {
            for pos in self.selection.into_iter() {
                self.sudoku.get_mut(pos).corner_notes.insert(num);
            }
        }
    }

    fn handle_number_input_center(&mut self, num: Number) {
        if self
            .sudoku
            .selection_and_all_center_notes(self.selection)
            .contains(num)
        {
            for pos in self.selection.into_iter() {
                self.sudoku.get_mut(pos).center_notes.remove(num);
            }
        } else {
            for pos in self.selection.into_iter() {
                self.sudoku.get_mut(pos).center_notes.insert(num);
            }
        }
    }

    pub fn handle_number_input(&mut self, num: Number) {
        match self.button_state.input_mode {
            InputMode::Solve => {
                if self.selection.count() == 1 {
                    let pos = self.selection.into_iter().next().expect("checked above");
                    *self.sudoku.get_mut(pos).solved_number = Some(num)
                } else {
                    match self.button_state.secondary_input {
                        InputMode::Solve | InputMode::Color => warn!("this input state is invalid"),
                        InputMode::Center => self.handle_number_input_center(num),
                        InputMode::Corner => self.handle_number_input_corner(num),
                    }
                }
            }
            InputMode::Center => self.handle_number_input_center(num),
            InputMode::Corner => self.handle_number_input_corner(num),
            InputMode::Color => {
                // TODO markup
            }
        }
    }

    pub fn handle_input(&mut self, input: &InputEvent) -> InputStatus {
        if let InputEvent::MotionEvent(motion_event) = input {
            let pointer = motion_event.pointer_at_index(0);
            let hit = if let Some(layout) = self.layout.as_ref() {
                layout.hit(pointer.x(), pointer.y())
            } else {
                return InputStatus::Unhandled;
            };
            match hit {
                layout::Hit::Cell(pos) => self.handle_grid_input(pos, motion_event.action()),
                layout::Hit::Button(button) if motion_event.action() == MotionAction::Up => {
                    match button {
                        layout::Button::Number(num) => self.handle_number_input(num),

                        layout::Button::Undo => self.undo(),
                        layout::Button::Redo => self.redo(),

                        layout::Button::SelectionMode => {
                            self.button_state.clear_on_new_selection =
                                !self.button_state.clear_on_new_selection;
                        }
                        layout::Button::Delete => self.handle_delete(),

                        layout::Button::Solve => self.button_state.input_mode = InputMode::Solve,
                        layout::Button::Center => self.button_state.input_mode = InputMode::Center,
                        layout::Button::Corner => self.button_state.input_mode = InputMode::Corner,
                        layout::Button::Color => self.button_state.input_mode = InputMode::Color,

                        layout::Button::Generic1
                        | layout::Button::Generic2
                        | layout::Button::Generic3 => {
                            info!("received generic button input")
                        }
                    }
                }
                layout::Hit::Button(_) => (),
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
}
impl State {
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
            button_state: Default::default(),
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
                    self.layout = Some(Layout::new(window_rect, &get_insets(&self.app)));
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

    pub fn draw_buttons(&mut self, pixmap: &mut PixmapMut) {
        let layout = if let Some(l) = self.layout.as_ref() {
            l
        } else {
            unreachable!()
        };
        for (rect, button) in layout.get_button_rects() {
            let path = make_rounded(rect, rect.width() / 20.0); // TODO use transform translate
            pixmap.fill_path(
                &path,
                &self.palette.button_background.into_paint(),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
            match button {
                layout::Button::Number(num) => {
                    self.draw_num_into_rect(num, rect, pixmap);
                }
                layout::Button::Undo => {}
                layout::Button::Redo => {}
                layout::Button::SelectionMode => {
                    if self.button_state.clear_on_new_selection {
                        pixmap.fill_path(
                            &path,
                            &self.palette.button_background_active.into_paint(),
                            FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                    }
                }
                layout::Button::Delete => {}
                layout::Button::Solve => {
                    if matches!(self.button_state.input_mode, InputMode::Solve) {
                        pixmap.fill_path(
                            &path,
                            &self.palette.button_background_active.into_paint(),
                            FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                    }
                }
                layout::Button::Center => {
                    if matches!(self.button_state.input_mode, InputMode::Center) {
                        pixmap.fill_path(
                            &path,
                            &self.palette.button_background_active.into_paint(),
                            FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                    }
                }
                layout::Button::Corner => {
                    if matches!(self.button_state.input_mode, InputMode::Corner) {
                        pixmap.fill_path(
                            &path,
                            &self.palette.button_background_active.into_paint(),
                            FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                    }
                }
                layout::Button::Color => {
                    if matches!(self.button_state.input_mode, InputMode::Color) {
                        pixmap.fill_path(
                            &path,
                            &self.palette.button_background_active.into_paint(),
                            FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                    }
                }
                layout::Button::Generic1 | layout::Button::Generic2 | layout::Button::Generic3 => {}
            }
        }
    }

    pub fn draw_cells(&mut self, pixmap: &mut PixmapMut) {
        let layout = if let Some(l) = self.layout.as_ref() {
            l
        } else {
            unreachable!()
        };

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
            let rect = layout.get_cell_rect(pos);
            let cell = self.sudoku.get(pos);

            if self.selection.contains(pos) {
                pixmap.fill_rect(
                    rect,
                    &self.palette.selection.into_paint(),
                    Transform::identity(),
                    None,
                );
            } else if highlight.contains(pos) {
                pixmap.fill_rect(
                    rect,
                    &self.palette.highlight.into_paint(),
                    Transform::identity(),
                    None,
                );
            }

            if let Some(num) = cell.given_number {
                self.draw_num_into_rect(*num, rect, pixmap);
                continue;
            }
            if let Some(num) = cell.solved_number {
                if num != self.sudoku.get(pos).solution {
                    pixmap.fill_rect(
                        rect,
                        &self.palette.wrong_number.into_paint(),
                        Transform::identity(),
                        None,
                    );
                }
                self.draw_num_into_rect(*num, rect, pixmap);
                continue;
            }

            if !cell.center_notes.empty() {
                let paths = cell
                    .center_notes
                    .into_iter()
                    .map(|n| self.make_num_path(n))
                    .collect::<Vec<_>>();
                let total_advance: f32 = paths.iter().map(|p| p.advance).sum();
                let cap_height = paths.first().unwrap().cap_height;
                let scale = (rect.height() / 3.0 * (1.0 - 2.0 * CELL_MARGIN) / cap_height)
                    .min(rect.width() * (1.0 - 2.0 * CELL_MARGIN / 3.0) / total_advance);
                let mut start_x = rect.left() + rect.width() / 2.0 - total_advance * scale / 2.0;
                let ground_line = rect.top() + rect.height() / 2.0 + cap_height * scale / 2.0;
                for NumPath { path, advance, .. } in paths {
                    pixmap.fill_path(
                        &path,
                        &self.palette.text.into_paint(),
                        FillRule::Winding,
                        Transform::from_scale(scale, -scale).post_translate(start_x, ground_line),
                        None,
                    );
                    start_x += advance * scale;
                }
            }
        }
    }

    pub fn render_frame(&mut self, window: &NativeWindow) {
        // TODO no cloning
        let layout = if let Some(l) = self.layout.clone() {
            l
        } else {
            return;
        };
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
            layout.get_grid_rect(),
            &self.palette.grid_background.into_paint(),
            Transform::identity(),
            None,
        );

        self.draw_buttons(&mut pixmap);

        self.draw_cells(&mut pixmap);
        layout.draw_grid(&self.palette, &mut pixmap);
    }

    fn draw_num_into_rect(&self, num: Number, rect: Rect, pixmap: &mut PixmapMut) {
        let l = self.make_num_path(num);
        let center_y = rect.top() + rect.height() / 2.0;
        let center_x = rect.left() + rect.width() / 2.0;
        let font_height = rect.height() * (1.0 - 2.0 * CELL_MARGIN);
        let scale = font_height / l.cap_height;

        let font_center_x = l.advance / 2.0;
        let font_center_y = l.cap_height / 2.0;
        let transform = Transform::from_scale(scale, -scale).post_translate(
            center_x - font_center_x * scale,
            center_y + font_center_y * scale,
        );
        let paint = self.palette.text.into_paint();
        pixmap.fill_path(&l.path, &paint, FillRule::Winding, transform, None);
    }

    fn make_num_path(&self, num: Number) -> NumPath {
        // TODO is this worth caching?
        let glyph_id = self
            .face
            .glyph_index(num.as_char())
            .expect("every digit is here");
        let mut builder = SkiaOutlineBuilder(PathBuilder::new());
        self.face.outline_glyph(glyph_id, &mut builder);
        let path = builder.0.finish().expect("path should always be valid");

        let cap_height = self
            .face
            .capital_height()
            .map(|h| h as f32)
            .unwrap_or_else(|| self.face.ascender() as f32);

        NumPath {
            path,
            advance: self.face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32,
            cap_height,
        }
    }
}

struct NumPath {
    path: Path,
    advance: f32,
    // TODO remove fields below
    cap_height: f32,
}

pub fn make_rounded(rect: Rect, rad: f32) -> Path {
    let mut pb = PathBuilder::new();
    pb.move_to(rect.left() + rad, rect.top());

    pb.line_to(rect.right() - rad, rect.top());
    pb.quad_to(rect.right(), rect.top(), rect.right(), rect.top() + rad);

    pb.line_to(rect.right(), rect.bottom() - rad);
    pb.quad_to(
        rect.right(),
        rect.bottom(),
        rect.right() - rad,
        rect.bottom(),
    );

    pb.line_to(rect.left() + rad, rect.bottom());
    pb.quad_to(rect.left(), rect.bottom(), rect.left(), rect.bottom() - rad);

    pb.line_to(rect.left(), rect.top() + rad);
    pb.quad_to(rect.left(), rect.top(), rect.left() + rad, rect.top());

    pb.close();
    pb.finish().expect("allways valid path")
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
