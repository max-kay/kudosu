use std::ffi::CString;
use std::io::Read;
use std::time::Duration;

use android_activity::{
    AndroidApp, InputStatus, MainEvent, PollEvent,
    input::{InputEvent, MotionAction},
    ndk::{hardware_buffer_format::HardwareBufferFormat, native_window::NativeWindow},
};
use log::{LevelFilter, info, warn};

mod canvas;
mod insets;
mod sudoku_types;

mod selection;
mod solver;

pub use canvas::Rect;

pub use selection::SelectionScreen;
pub use sudoku_types::{GridPosition, NineGrid, Number, NumberBucket, PositionBucket, Sudoku};

use crate::{
    canvas::{Canvas, FaceBook, Palette, Swatch},
    solver::Solver,
};

enum Navigation {
    ToSelection(usize),
    ToSolving(usize),
    None,
}

trait Component {
    fn handle_input(
        &mut self,
        input: &InputEvent,
        draw_state_handle: &mut DrawState,
    ) -> (InputStatus, Navigation);

    fn render_frame(&self, canvas: &mut Canvas<'_>);

    fn relayout(&mut self, bounds: Rect);
}

const NAME: &str = "Kudosu";

struct Welcome {}

impl Welcome {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for Welcome {
    fn handle_input(
        &mut self,
        input: &InputEvent,
        draw_state_handle: &mut DrawState,
    ) -> (InputStatus, Navigation) {
        if let InputEvent::MotionEvent(motion_event) = input {
            if matches!(motion_event.action(), MotionAction::Up) {
                draw_state_handle.relayout();
                return (InputStatus::Handled, Navigation::ToSelection(0));
            }
            return (InputStatus::Handled, Navigation::None);
        }
        return (InputStatus::Unhandled, Navigation::None);
    }

    fn render_frame(&self, canvas: &mut Canvas<'_>) {
        canvas.fill(Swatch::Background);
    }

    fn relayout(&mut self, _bounds: Rect) {}
}

enum AppState {
    Welcome(Welcome),
    Selection(SelectionScreen),
    Solving(Solver),
}

impl AppState {
    fn handle_input(
        &mut self,
        input: &InputEvent,
        draw_state_handle: &mut DrawState,
    ) -> InputStatus {
        let (status, navigation) = match self {
            AppState::Welcome(welcome) => welcome.handle_input(input, draw_state_handle),
            AppState::Selection(screen) => screen.handle_input(input, draw_state_handle),
            AppState::Solving(game_state) => game_state.handle_input(input, draw_state_handle),
        };
        match navigation {
            Navigation::ToSelection(i) => {
                *self = Self::Selection(SelectionScreen::new(i));
                draw_state_handle.relayout();
            }
            Navigation::ToSolving(i) => {
                *self = Self::Solving(Solver::new(i));
                draw_state_handle.relayout();
            }

            Navigation::None => (),
        }
        status
    }

    fn render_frame(&self, canvas: &mut Canvas<'_>) {
        match self {
            AppState::Welcome(welcome) => welcome.render_frame(canvas),
            AppState::Selection(screen) => screen.render_frame(canvas),
            AppState::Solving(game_state) => game_state.render_frame(canvas),
        }
    }

    fn relayout(&mut self, bounds: Rect) {
        match self {
            AppState::Welcome(welcome) => welcome.relayout(bounds),
            AppState::Selection(selection_screen) => selection_screen.relayout(bounds),
            AppState::Solving(game_state) => game_state.relayout(bounds),
        }
    }
}

pub struct MyApp {
    state: AppState,
    running: bool,
    app: AndroidApp,

    palette: Palette,
    draw_state: DrawState,
    face_book: FaceBook,
}

impl MyApp {
    pub fn running(&self) -> bool {
        self.running
    }

    pub fn handle_input(&mut self, input: &InputEvent) -> InputStatus {
        self.state.handle_input(input, &mut self.draw_state)
    }
}

impl MyApp {
    pub fn new(app: AndroidApp, face_book: FaceBook) -> Self {
        Self {
            running: true,
            app,
            palette: Default::default(),
            face_book,
            draw_state: DrawState::NeedsRelayout,
            state: AppState::Welcome(Welcome::new()),
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
            _ => return,
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
                if let Ok(mut iter) = self.app.clone().input_events_iter() {
                    while iter.next(|i| self.handle_input(i)) {}
                }
            }
            MainEvent::Destroy => {
                self.running = false;
            }
            MainEvent::ContentRectChanged { .. } => {
                self.draw_state.relayout();
            }
            MainEvent::GainedFocus => {
                info!("GainedFocus");
            }
            MainEvent::LostFocus => {
                info!("LostFocus");
            }
            MainEvent::ConfigChanged { .. } => {
                self.draw_state.relayout();
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
                    if let Some(rect) = insets::get_bounds(&self.app) {
                        self.state.relayout(rect);
                        self.render_frame(&window);
                    }
                }
                DrawState::NeedsRedraw => {
                    self.render_frame(&window);
                }
            }
            self.draw_state.clear();
        }
    }

    pub fn render_frame(&self, window: &NativeWindow) {
        // TODO dont spin wait
        let mut lock = loop {
            if let Ok(lock) = window.lock(None) {
                break lock;
            }
        };

        let bytes = lock.bytes().expect("Bytes should be available");
        let pixmap = tiny_skia::PixmapMut::from_bytes(
            // SAFETY: `lock` holds exclusive access to the underlying native window buffer
            // for the duration of this frame. Casting its raw pointer to `*mut u8` and creating
            // a mutable slice is sound because `lock` is the sole owner and no other references exist.
            unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr() as _, bytes.len()) },
            lock.stride() as u32,
            lock.height() as u32,
        )
        .unwrap();
        let mut canvas = Canvas::new(self.face_book.clone(), pixmap, self.palette.clone());
        self.state.render_frame(&mut canvas);
    }
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

const SUDOKUS: &[(&str, &str)] = &[
    // source https://qqwing.com/generate.html
    (
        "..7.....82.9.......38...7........6.582......1....583...91.35..2...2..96..7..8....",
        "467391258259874136138526749713942685825763491946158327691435872584217963372689514",
    ),
    (
        "4....8.......7....6....47.3....83..7..8..5.9.5....96..3.5..6..416.9......9.3.....",
        "457638219239571468681294753926183547718465392543729681375816924162947835894352176",
    ),
    (
        ".........6.4....2.2...7.....7...6..13.58.4..6........8...4...5....21..8.75.38....",
        "587642193634198725291573864872936541315824976469751238128467359943215687756389412",
    ),
    (
        ".1.2....6...8.1...9.7........97.........9..386.8..21...5....2..2.3.86......1.7...",
        "315279846462831795987465321539718462721694538648352179156943287273586914894127653",
    ),
    (
        "684.7.9...........1...2..674...8.......95...687.2.64..7..14.2.52.....7.4.........",
        "684573912527691348193428567456387129312954876879216453768149235231865794945732681",
    ),
    (
        ".5.....1...29....3......5.29........3..54...........6..8542.....1..8.6.......189.",
        "853274916142965783796813542928736451361548279574192368685429137419387625237651894",
    ),
    (
        "28...1.....9..7.6...1.....79....2.843......9..6....2.1......9......74..24..39....",
        "287631459549287163631549827915762384372418695864953271753126948196874532428395716",
    ),
    (
        ".7........9..5.....1....98583.7....4..4.29.......31.7..52.1............1..9....52",
        "573984126298156437416273985831765294764829513925431678652318749347592861189647352",
    ),
    (
        "....64.73....739.....9..8...493...5...2....9.6...8.2.......87.436.5........7.....",
        "918264573456873912723915846149327658832156497675489231591638724367542189284791365",
    ),
    (
        "..1.24..7.......1..4....8...6..83...38.7..9.....6.1............73.2....662.3.9...",
        "951824637873596214246137859167983542385742961492651378519468723738215496624379185",
    ),
];

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    // sudoku_types::test::number_bucket();
    // sudoku_types::test::pos_bucket();

    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(LevelFilter::Info)
            .with_tag(NAME),
    );

    let asset_mgr = app.asset_manager();
    let get_bytes = |s: &str| {
        let path = CString::new(s).unwrap();

        let mut asset = asset_mgr.open(&path).expect("font was not available");

        let mut font_bytes = Vec::new();
        asset
            .read_to_end(&mut font_bytes)
            .expect("could not get asset buffer");
        let font_bytes = font_bytes.leak(); // TODO doesn't seem rusty
        (
            s.into(),
            ttf_parser::Face::parse(font_bytes, 0).expect("could not parse font"),
        )
    };
    let faces = vec![
        get_bytes("font/Noto_Sans/static/NotoSans-Regular.ttf"),
        get_bytes("font/Noto_Sans_Symbols/static/NotoSansSymbols-Regular.ttf"),
        get_bytes("font/Noto_Sans_Symbols_2/NotoSansSymbols2-Regular.ttf"),
        get_bytes("font/Noto_Emoji/static/NotoEmoji-Regular.ttf"),
        get_bytes("font/Noto_Sans_JP/static/NotoSansJP-Regular.ttf"),
    ];

    let mut state = MyApp::new(app.clone(), FaceBook(faces));

    while state.running() {
        // Redraw on events or poll interval
        app.poll_events(Some(Duration::from_millis(16)), |event| {
            state.event_callback(event)
        });
    }
}
