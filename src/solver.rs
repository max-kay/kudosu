use std::str::FromStr;

use android_activity::{
    InputStatus,
    input::{InputEvent, MotionAction},
};
use log::{info, warn};

use crate::{
    Canvas, Component, DrawState, GridPosition, Navigation, Number, NumberBucket, PositionBucket,
    Rect, SUDOKUS, Sudoku, canvas::Swatch, sudoku_types::GridLayout,
};

#[derive(Copy, Clone)]
pub enum SelectionMode {
    New,
    WithOld,
    Add,
    Clear,
}

#[derive(PartialEq, Eq)]
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
    selected_num: Option<Number>,
}

impl ButtonState {
    pub fn is_active(&self, button: Button) -> bool {
        match button {
            Button::Undo | Button::Redo | Button::Delete => false,
            Button::Number(num) => {
                if let Some(n) = self.selected_num
                    && n == num
                {
                    true
                } else {
                    false
                }
            }
            Button::SelectionMode => self.clear_on_new_selection,
            Button::Solve => self.input_mode == InputMode::Solve,
            Button::Center => self.input_mode == InputMode::Center,
            Button::Corner => self.input_mode == InputMode::Corner,
            Button::Color => self.input_mode == InputMode::Color,
            // TODO generic button
            Button::Generic1 => false,
            Button::Generic2 => false,
            Button::Generic3 => false,
        }
    }

    pub fn get_sel_mode(&self) -> SelectionMode {
        if self.clear_on_new_selection {
            SelectionMode::New
        } else {
            SelectionMode::WithOld
        }
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        Self {
            clear_on_new_selection: true,
            input_mode: InputMode::Solve,
            secondary_input: InputMode::Center,
            selected_num: None,
        }
    }
}

pub struct Solver {
    sudoku: Box<Sudoku>,
    sel_mode: SelectionMode,
    selection: PositionBucket,
    button_state: ButtonState,
    layout: Option<Layout>,
}

impl Solver {
    pub fn new(i: usize) -> Self {
        Self {
            sudoku: Box::new(Sudoku::new(
                FromStr::from_str(SUDOKUS[i].0).unwrap(),
                FromStr::from_str(SUDOKUS[i].1).unwrap(),
            )),
            sel_mode: SelectionMode::New,
            selection: PositionBucket::new(),
            button_state: Default::default(),
            layout: None,
        }
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
            MotionAction::Up => self.sel_mode = self.button_state.get_sel_mode(),
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
        let all_center = self
            .sudoku
            .iter_cells(self.selection)
            .fold(NumberBucket::new(), |acc, cell| acc | *cell.center_notes);
        let all_corner = self
            .sudoku
            .iter_cells(self.selection)
            .fold(NumberBucket::new(), |acc, cell| acc | *cell.corner_notes);
        match self.button_state.input_mode {
            InputMode::Center => {
                if !all_center.empty() {
                    self.sudoku
                        .iter_cells_mut(self.selection)
                        .for_each(|c| c.center_notes.clear());
                    return;
                };
                if !all_corner.empty() {
                    self.sudoku
                        .iter_cells_mut(self.selection)
                        .for_each(|c| c.corner_notes.clear());
                    return;
                };
                self.sudoku
                    .iter_cells_mut(self.selection)
                    .for_each(|c| *c.solved_number = None);
            }
            InputMode::Solve | InputMode::Corner => {
                if !all_corner.empty() {
                    self.sudoku
                        .iter_cells_mut(self.selection)
                        .for_each(|c| c.corner_notes.clear());
                    return;
                };
                if !all_center.empty() {
                    self.sudoku
                        .iter_cells_mut(self.selection)
                        .for_each(|c| c.center_notes.clear());
                    return;
                };
                self.sudoku
                    .iter_cells_mut(self.selection)
                    .for_each(|c| *c.solved_number = None);
            }
            InputMode::Color => (),
        }
    }

    fn handle_number_input_corner(&mut self, num: Number) {
        let every_corner = self
            .sudoku
            .iter_cells(self.selection)
            .fold(NumberBucket::all(), |acc, c| acc & *c.corner_notes);
        if every_corner.contains(num) {
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
        let every_center = self
            .sudoku
            .iter_cells(self.selection)
            .fold(NumberBucket::all(), |acc, c| acc & *c.center_notes);
        if every_center.contains(num) {
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
        if self.selection.is_empty() {
            match self.button_state.selected_num.take() {
                Some(sel_num) if num == sel_num => {}
                _ => self.button_state.selected_num = Some(num),
            }
            return;
        }
        match self.button_state.input_mode {
            InputMode::Solve => {
                if self.selection.count() == 1 {
                    let pos = self.selection.into_iter().next().expect("checked above");
                    *self.sudoku.get_mut(pos).solved_number = Some(num)
                } else {
                    match self.button_state.secondary_input {
                        InputMode::Solve => self
                            .sudoku
                            .iter_cells_mut(self.selection)
                            .for_each(|c| *c.solved_number = Some(num)),
                        InputMode::Center => self.handle_number_input_center(num),
                        InputMode::Corner => self.handle_number_input_corner(num),
                        InputMode::Color => warn!("this input state is invalid"),
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
}

impl Solver {
    pub fn draw_buttons(&self, canvas: &mut Canvas<'_>) {
        let layout = if let Some(l) = self.layout.as_ref() {
            l
        } else {
            unreachable!()
        };
        for (rect, button) in layout.get_button_rects() {
            let is_active = self.button_state.is_active(button);
            let foreground_color = if is_active {
                canvas.fill_rect_rounded(rect, rect.width() / 20.0, Swatch::ButtonBackgroundActive);
                Swatch::ButtonForegroundActive
            } else {
                canvas.fill_rect_rounded(rect, rect.width() / 20.0, Swatch::ButtonBackground);
                Swatch::ButtonForeground
            };

            match button {
                Button::Number(num) => {
                    canvas.draw_num(num, rect, foreground_color);
                }
                Button::Undo => {}
                Button::Redo => {}
                Button::SelectionMode => {}
                Button::Delete => {}
                Button::Solve => {
                    canvas.draw_num(Number(1), rect, foreground_color);
                }
                Button::Center => {
                    let bucket = NumberBucket::example();
                    canvas.draw_center_notes(rect, bucket, foreground_color);
                }
                Button::Corner => {
                    let bucket = NumberBucket::example();
                    canvas.draw_corner_notes(rect, bucket, foreground_color);
                }
                Button::Color => {}
                Button::Generic1 | Button::Generic2 | Button::Generic3 => {}
            }
        }
    }
}

impl Component for Solver {
    fn handle_input(
        &mut self,
        input: &InputEvent,
        draw_state_handle: &mut DrawState,
    ) -> (InputStatus, Navigation) {
        if let InputEvent::MotionEvent(motion_event) = input {
            let pointer = motion_event.pointer_at_index(0);
            let hit = if let Some(layout) = self.layout.as_ref() {
                layout.hit(pointer.x(), pointer.y())
            } else {
                return (InputStatus::Unhandled, Navigation::None);
            };
            match hit {
                Hit::Cell(pos) => self.handle_grid_input(pos, motion_event.action()),
                Hit::Button(button) if motion_event.action() == MotionAction::Up => match button {
                    Button::Number(num) => self.handle_number_input(num),

                    Button::Undo => self.undo(),
                    Button::Redo => self.redo(),

                    Button::SelectionMode => {
                        self.button_state.clear_on_new_selection =
                            !self.button_state.clear_on_new_selection;
                        self.sel_mode = self.button_state.get_sel_mode();
                    }
                    Button::Delete => self.handle_delete(),

                    Button::Solve => self.button_state.input_mode = InputMode::Solve,
                    Button::Center => self.button_state.input_mode = InputMode::Center,
                    Button::Corner => self.button_state.input_mode = InputMode::Corner,
                    Button::Color => self.button_state.input_mode = InputMode::Color,

                    Button::Generic1 | Button::Generic2 | Button::Generic3 => {
                        info!("received generic button input")
                    }
                },
                Hit::Button(_) => (),
                Hit::None => match self.sel_mode {
                    SelectionMode::New | SelectionMode::WithOld => {
                        self.selection = PositionBucket::new()
                    }
                    SelectionMode::Add | SelectionMode::Clear => (),
                },
            }
            draw_state_handle.redraw();
            (InputStatus::Handled, Navigation::None)
        } else {
            (InputStatus::Unhandled, Navigation::None)
        }
    }

    fn render_frame(&self, canvas: &mut Canvas<'_>) {
        let layout = if let Some(l) = self.layout {
            l
        } else {
            warn!("could not render frame because layout was not available");
            return;
        };

        canvas.fill(Swatch::Background);
        canvas.fill_rect(layout.get_grid_rect(), Swatch::GridBackground);

        let highlight = if self.selection.is_empty() {
            PositionBucket::new()
        } else {
            let mut hl = PositionBucket::all();
            for pos in self.selection.into_iter() {
                hl = hl & pos.sees_by_sudoku();
            }
            hl
        };
        self.sudoku.render(
            canvas,
            &[
                (Swatch::Highlight, highlight),
                (Swatch::Selection, self.selection),
            ],
            &layout.grid,
        );

        self.draw_buttons(canvas);
    }

    fn relayout(&mut self, bounds: Rect) {
        self.layout = Some(Layout::new(bounds));
    }
}

const MARGIN_FACTOR: f32 = 0.05;

#[derive(Clone, Copy)]
struct ButtonLayout {
    all: Rect,
    margin: f32,
}

impl ButtonLayout {
    const N_COLS: usize = 5;
    const N_ROWS: usize = 4;
    const BUTTONS: [Button; 4 * 5] = [
        Button::Undo,              // (0, 0)
        Button::Number(Number(1)), // (1, 0)
        Button::Number(Number(2)), // (2, 0)
        Button::Number(Number(3)), // (3, 0)
        Button::Solve,             // (4, 0)
        Button::Redo,              // (0, 1)
        Button::Number(Number(4)), // (1, 1)
        Button::Number(Number(5)), // (2, 1)
        Button::Number(Number(6)), // (3, 1)
        Button::Corner,            // (4, 1)
        Button::SelectionMode,     // (0, 2)
        Button::Number(Number(7)), // (1, 2)
        Button::Number(Number(8)), // (2, 2)
        Button::Number(Number(9)), // (3, 2)
        Button::Center,            // (4, 2)
        Button::Delete,            // (0, 3)
        Button::Generic1,          // (1, 3)
        Button::Generic2,          // (2, 3)
        Button::Generic3,          // (3, 3)
        Button::Color,             // (4, 3)
    ];

    pub fn hit(&self, x: f32, y: f32) -> Option<Button> {
        let norm_x = ((x - self.all.left()) / self.all.width() * Self::N_COLS as f32).floor();
        let norm_y = ((y - self.all.top()) / self.all.height() * Self::N_ROWS as f32).floor();
        if !(0.0 <= norm_x && norm_x <= Self::N_COLS as f32) {
            return None;
        }
        if !(0.0 <= norm_y && norm_y <= Self::N_ROWS as f32) {
            return None;
        }
        Some(Self::BUTTONS[norm_x as usize + norm_y as usize * Self::N_COLS])
    }

    pub fn get_button_rects(&self) -> Vec<(Rect, Button)> {
        let mut out = Vec::new();
        for i in 0..Self::N_ROWS {
            for j in 0..Self::N_COLS {
                out.push((
                    Rect::from_xywh(
                        self.all.left()
                            + j as f32 * self.all.width() / Self::N_COLS as f32
                            + self.margin / 2.0,
                        self.all.top()
                            + i as f32 * self.all.height() / Self::N_ROWS as f32
                            + self.margin / 2.0,
                        self.all.width() / Self::N_COLS as f32 - self.margin,
                        self.all.height() / Self::N_ROWS as f32 - self.margin,
                    ),
                    Self::BUTTONS[i * Self::N_COLS + j],
                ))
            }
        }
        out
    }
}

struct MenuLayout {
    pause: Rect,
    settings: Rect,
    hint: Rect,
}

#[derive(Clone, Copy)]
pub struct Layout {
    grid: GridLayout,
    button: ButtonLayout,
}

impl Layout {
    pub fn new_portrait(window: Rect) -> Self {
        let margin = window.width() * MARGIN_FACTOR;
        let size = window.width() - 2.0 * margin;
        let ui_button_size = size / (ButtonLayout::N_COLS as f32 + 1.0);
        let button_area = Rect::from_ltrb(
            margin + ui_button_size,
            window.bottom() - ButtonLayout::N_ROWS as f32 * ui_button_size - margin,
            window.right() - margin,
            window.bottom() - margin,
        );
        let button_margin = ui_button_size / 20.0;
        let grid = GridLayout {
            left: window.left() + margin,
            top: button_area.top() - margin - size,
            size: size,
        };
        Self {
            grid,
            button: ButtonLayout {
                all: button_area,
                margin: button_margin,
            },
        }
    }

    pub fn new_landscape(window: Rect) -> Self {
        let margin = window.height() * MARGIN_FACTOR;
        let grid_size = window.height() - 2.0 * margin;
        let ui_button_size = (window.width() - 3.0 * margin) / (ButtonLayout::N_COLS as f32 + 1.0);
        let button_area = Rect::from_ltrb(
            2.0 * margin + grid_size + ui_button_size,
            window.bottom() - margin - ui_button_size * ButtonLayout::N_ROWS as f32,
            window.right() - margin,
            window.bottom() - margin,
        );
        let button_margin = ui_button_size / 20.0;
        let grid = GridLayout {
            left: window.left() + margin,
            top: button_area.top() + margin,
            size: grid_size,
        };
        Self {
            grid,
            button: ButtonLayout {
                all: button_area,
                margin: button_margin,
            },
        }
    }

    pub fn new(window: Rect) -> Self {
        if window.width() < window.height() {
            Self::new_portrait(window)
        } else {
            Self::new_landscape(window)
        }
    }
}

#[derive(Clone, Copy)]
pub enum Button {
    Number(Number),
    Undo,
    Redo,
    SelectionMode,
    Delete,
    Solve,
    Center,
    Corner,
    Color,
    Generic1,
    Generic2,
    Generic3,
}

pub enum Hit {
    Cell(GridPosition),
    Button(Button),
    None,
}

impl Layout {
    pub fn hit(&self, x: f32, y: f32) -> Hit {
        if let Some(pos) = self.grid.hit(x, y) {
            return Hit::Cell(pos);
        }
        if let Some(but) = self.button.hit(x, y) {
            return Hit::Button(but);
        }
        return Hit::None;
    }

    pub fn get_grid_rect(&self) -> Rect {
        Rect::from_xywh(
            self.grid.left,
            self.grid.top,
            self.grid.size,
            self.grid.size,
        )
    }

    pub fn get_button_rects(&self) -> Vec<(Rect, Button)> {
        self.button.get_button_rects()
    }
}
