use std::str::FromStr;

use android_activity::{
    InputStatus,
    input::{InputEvent, MotionAction},
};
use log::{info, warn};
use tiny_skia::Stroke;

use crate::{
    Canvas, Component, DrawState, GridPosition, Navigation, Number, NumberBucket, PositionBucket,
    Rect, SUDOKUS, Sudoku, canvas::Swatch, sudoku_types::DiffStack,
};

pub mod layout;

use layout::{Button, Hit, Layout, MenuItem};

#[derive(Copy, Clone, serde::Deserialize, serde::Serialize)]
pub enum SelectionMode {
    New,
    WithOld,
    Add,
    Clear,
}

#[derive(PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum InputMode {
    Solve,
    Corner,
    Center,
    Color,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ButtonState {
    add_on_new_selection: bool,
    input_mode: InputMode,
    secondary_input: InputMode,
    selected_num: Option<Number>,
    started_on: Option<Hit>,
    active_hover: bool,
}

impl ButtonState {
    pub fn is_active(&self, button: Button) -> bool {
        match button {
            Button::Undo | Button::Redo | Button::Delete => false,
            Button::SelectionMode => self.add_on_new_selection,
            Button::Solve => self.input_mode == InputMode::Solve,
            Button::Center => self.input_mode == InputMode::Center,
            Button::Corner => self.input_mode == InputMode::Corner,
            Button::Color => self.input_mode == InputMode::Color,
            Button::Number(_) => false,  // handled in draw_num_button
            Button::Generic(_) => false, // handled in draw_generic_button
        }
    }

    pub fn get_sel_mode(&self) -> SelectionMode {
        if self.add_on_new_selection {
            SelectionMode::WithOld
        } else {
            SelectionMode::New
        }
    }

    pub fn handle_generic_input(&mut self, num: u8) {
        match self.input_mode {
            InputMode::Solve => match num {
                1 => self.secondary_input = InputMode::Solve,
                2 => self.secondary_input = InputMode::Corner,
                3 => self.secondary_input = InputMode::Center,
                _ => unreachable!(),
            },
            InputMode::Color => (),
            InputMode::Corner | InputMode::Center => (),
        }
    }
}

const COLOR_BUTTON_SHRINK: f32 = 0.2;

impl ButtonState {
    fn get_color_pair(is_active: bool) -> (Swatch, Swatch) {
        if is_active {
            (
                Swatch::ButtonBackgroundActive,
                Swatch::ButtonForegroundActive,
            )
        } else {
            (Swatch::ButtonBackground, Swatch::ButtonForeground)
        }
    }

    pub fn draw_generic_button(&self, num: u8, rect: Rect, canvas: &mut Canvas<'_>) {
        match self.input_mode {
            InputMode::Solve => {
                let mut is_active = match self.secondary_input {
                    InputMode::Solve => num == 1,
                    InputMode::Corner => num == 2,
                    InputMode::Center => num == 3,
                    InputMode::Color => unreachable!(),
                };

                match self.started_on {
                    Some(Hit::Button(Button::Generic(n))) if n == num && self.active_hover => {
                        is_active = !is_active
                    }
                    _ => (),
                }
                let (background, foreground) = Self::get_color_pair(is_active);
                match num {
                    1 => {
                        canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                        canvas.draw_num(Number::new(1), rect, foreground);
                    }
                    2 => {
                        canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                        canvas.draw_corner_notes(rect, NumberBucket::example(), foreground);
                    }
                    3 => {
                        canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                        canvas.draw_center_notes(rect, NumberBucket::example(), foreground);
                    }
                    _ => unreachable!(),
                }
            }
            InputMode::Color => {
                canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, Swatch::ButtonBackground)
            }
            InputMode::Center | InputMode::Corner => {} // Generic buttons have no function in this
                                                        // state
        }
    }

    pub fn draw_num_button(&self, num: Number, rect: Rect, canvas: &mut Canvas<'_>) {
        let mut is_active = if let Some(n) = self.selected_num {
            n == num
        } else {
            false
        };

        match self.started_on {
            Some(Hit::Button(Button::Number(n))) if n == num && self.active_hover => {
                is_active = !is_active
            }
            _ => (),
        }
        let (background, foreground) = Self::get_color_pair(is_active);
        match self.input_mode {
            InputMode::Solve => {
                canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                canvas.draw_num(num, rect, foreground);
            }
            InputMode::Center => {
                canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                canvas.draw_center_notes(rect, num.into(), foreground);
            }
            InputMode::Corner => {
                canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                canvas.draw_corner_notes(rect, num.into(), foreground);
            }
            InputMode::Color => {
                canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, Swatch::ButtonBackground);
                canvas.fill_rect(
                    rect.shrink(rect.width() * COLOR_BUTTON_SHRINK),
                    Swatch::WrongNumber,
                );
            }
        }
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        Self {
            add_on_new_selection: false,
            input_mode: InputMode::Solve,
            secondary_input: InputMode::Center,
            selected_num: None,
            started_on: None,
            active_hover: false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Solver {
    sudoku: Box<Sudoku>,
    index: usize,
    sel_mode: SelectionMode,
    selection: PositionBucket,
    button_state: ButtonState,
    layout: Option<Layout>,
    needs_diff: bool,
    is_in_pause: bool,

    undo_stack: DiffStack,
}

impl Solver {
    pub fn new(i: usize) -> Self {
        Self {
            sudoku: Box::new(Sudoku::new(
                FromStr::from_str(SUDOKUS[i].0).unwrap(),
                FromStr::from_str(SUDOKUS[i].1).unwrap(),
            )),
            index: i,
            sel_mode: SelectionMode::New,
            selection: PositionBucket::new(),
            button_state: Default::default(),
            layout: None,
            needs_diff: false,
            is_in_pause: false,
            undo_stack: DiffStack::new(),
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
        if let Some(diff) = self.undo_stack.pop() {
            self.sudoku.apply_diff(diff);
        } else {
            info!("No undo left")
        }
    }

    pub fn redo(&mut self) {
        if let Some(diff) = self.undo_stack.unpop() {
            self.sudoku.apply_diff(diff);
        } else {
            info!("No redo left")
        }
    }

    pub fn enter_pause(&mut self) {
        self.is_in_pause = true;
    }

    pub fn handle_delete(&mut self) {
        self.needs_diff = true;
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
                if !all_center.is_empty() {
                    self.sudoku
                        .iter_cells_mut(self.selection)
                        .for_each(|c| c.center_notes.clear());
                    return;
                };
                if !all_corner.is_empty() {
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
                if !all_corner.is_empty() {
                    self.sudoku
                        .iter_cells_mut(self.selection)
                        .for_each(|c| c.corner_notes.clear());
                    return;
                };
                if !all_center.is_empty() {
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
            for cell in self.sudoku.iter_cells_mut(self.selection) {
                if cell.solved_number.is_none() && cell.given_number.is_none() {
                    cell.corner_notes.remove(num);
                }
            }
        } else {
            for cell in self.sudoku.iter_cells_mut(self.selection) {
                if cell.solved_number.is_none() && cell.given_number.is_none() {
                    cell.corner_notes.insert(num);
                }
            }
        }
    }

    fn handle_number_input_center(&mut self, num: Number) {
        let every_center = self
            .sudoku
            .iter_cells(self.selection)
            .fold(NumberBucket::all(), |acc, c| acc & *c.center_notes);
        if every_center.contains(num) {
            for cell in self.sudoku.iter_cells_mut(self.selection) {
                if cell.solved_number.is_none() && cell.given_number.is_none() {
                    cell.center_notes.remove(num);
                }
            }
        } else {
            for cell in self.sudoku.iter_cells_mut(self.selection) {
                if cell.solved_number.is_none() && cell.given_number.is_none() {
                    cell.center_notes.insert(num);
                }
            }
        }
    }

    fn handle_number_input_solve(&mut self, num: Number) {
        let mut affected = PositionBucket::new();
        for pos in self.selection.into_iter() {
            let cell = self.sudoku.get_mut(pos);
            if cell.given_number.is_none() {
                *cell.solved_number = Some(num);
                affected = affected | pos.sees_by_sudoku();
            }
        }
        for cell in self.sudoku.iter_cells_mut(affected) {
            cell.corner_notes.remove(num);
            cell.center_notes.remove(num);
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
        self.needs_diff = true;
        match self.button_state.input_mode {
            InputMode::Solve => {
                if self.selection.count() == 1 {
                    self.handle_number_input_solve(num);
                } else {
                    match self.button_state.secondary_input {
                        InputMode::Solve => self.handle_number_input_solve(num),
                        InputMode::Center => self.handle_number_input_center(num),
                        InputMode::Corner => self.handle_number_input_corner(num),
                        InputMode::Color => unreachable!(),
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

const BUTTON_SYMB_MARGIN: f32 = 0.1;
const MENU_MARGIN: f32 = 0.15;
const CORNER_RAD: f32 = 1.0 / 20.0;

impl Solver {
    pub fn draw_buttons(&self, canvas: &mut Canvas<'_>) {
        let layout = if let Some(l) = self.layout.as_ref() {
            l
        } else {
            unreachable!()
        };
        for (rect, button) in layout.get_button_rects() {
            let mut is_active = self.button_state.is_active(button);
            match self.button_state.started_on {
                Some(Hit::Button(b)) if b == button && self.button_state.active_hover => {
                    is_active = !is_active
                }
                _ => (),
            }
            let (background, foreground) = ButtonState::get_color_pair(is_active);

            let symb_rect = rect.shrink(rect.width().min(rect.height()) * BUTTON_SYMB_MARGIN);

            match button {
                Button::Undo => {
                    let color = if self.undo_stack.can_undo() {
                        Swatch::UiColor
                    } else {
                        Swatch::UiColorInActive
                    };
                    canvas.draw_char_centered('←', symb_rect, color)
                }
                Button::Redo => {
                    let color = if self.undo_stack.can_redo() {
                        Swatch::UiColor
                    } else {
                        Swatch::UiColorInActive
                    };
                    canvas.draw_char_centered('→', symb_rect, color)
                }
                Button::SelectionMode => {
                    canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                    canvas.draw_char_centered('▚', symb_rect, foreground);
                }
                Button::Delete => {
                    canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                    canvas.draw_char_centered('⌫', symb_rect, foreground);
                }
                Button::Solve => {
                    canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                    canvas.draw_num(Number::N1, rect, foreground);
                }
                Button::Center => {
                    canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                    let bucket = NumberBucket::example();
                    canvas.draw_center_notes(rect, bucket, foreground);
                }
                Button::Corner => {
                    canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                    let bucket = NumberBucket::example();
                    canvas.draw_corner_notes(rect, bucket, foreground);
                }
                Button::Color => {
                    canvas.fill_rect_rounded(rect, rect.width() * CORNER_RAD, background);
                    canvas.draw_char_centered('✎', symb_rect, foreground);
                }

                // These buttons are drawn depending on input mode
                Button::Generic(num) => {
                    self.button_state.draw_generic_button(num, rect, canvas);
                }
                Button::Number(num) => {
                    self.button_state.draw_num_button(num, rect, canvas);
                }
            }
        }
    }

    fn draw_menu(&self, canvas: &mut Canvas<'_>) {
        let layout = if let Some(l) = self.layout.as_ref() {
            l
        } else {
            unreachable!()
        };

        canvas.outline_rect_rounded(
            layout.menu.pause,
            layout.menu.pause.width() * CORNER_RAD,
            Swatch::UiColor,
            &Stroke::default(),
        );
        canvas.draw_char_centered(
            '⏸',
            layout
                .menu
                .pause
                .shrink(layout.menu.pause.width() * BUTTON_SYMB_MARGIN),
            Swatch::UiColor,
        );

        canvas.outline_rect_rounded(
            layout.menu.settings,
            layout.menu.settings.width() * CORNER_RAD,
            Swatch::UiColor,
            &Stroke::default(),
        );
        canvas.draw_char_centered(
            '⛭',
            layout
                .menu
                .settings
                .shrink(layout.menu.pause.width() * BUTTON_SYMB_MARGIN),
            Swatch::UiColor,
        );

        canvas.outline_rect_rounded(
            layout.menu.hint,
            layout.menu.hint.width() * CORNER_RAD,
            Swatch::UiColor,
            &Stroke::default(),
        );
        canvas.draw_char_centered(
            '?',
            layout
                .menu
                .hint
                .shrink(layout.menu.pause.width() * BUTTON_SYMB_MARGIN),
            Swatch::UiColor,
        );
    }
}

impl Component for Solver {
    fn handle_input(
        &mut self,
        input: &InputEvent,
        draw_state_handle: &mut DrawState,
    ) -> (InputStatus, Navigation) {
        let motion_event = if let InputEvent::MotionEvent(motion_event) = input {
            motion_event
        } else {
            return (InputStatus::Unhandled, Navigation::None);
        };

        let old_sudoku = self.sudoku.clone();

        let pointer = motion_event.pointer_at_index(0);
        let action = motion_event.action();
        let hit = if let Some(layout) = self.layout.as_ref() {
            layout.hit(pointer.x(), pointer.y())
        } else {
            if action == MotionAction::Up {
                self.button_state.started_on = None;
            }
            return (InputStatus::Unhandled, Navigation::None);
        };
        match action {
            MotionAction::Down => {
                if self.button_state.started_on.is_some() {
                    warn!("had start of touch when starting new touch event");
                }
                self.button_state.started_on = Some(hit);
                self.button_state.active_hover = true;
                self.button_state.selected_num = None;
            }
            MotionAction::Move | MotionAction::Up => {
                if let Some(h) = self.button_state.started_on {
                    if h != hit {
                        self.button_state.active_hover = false;
                    } else {
                        self.button_state.active_hover = true;
                    }
                } else {
                    warn!("had no start button during move or up")
                }
            }
            _ => (),
        }
        match hit {
            Hit::Cell(pos) => {
                if matches!(
                    self.button_state.started_on,
                    None | Some(Hit::Cell(_)) | Some(Hit::None)
                ) {
                    self.handle_grid_input(pos, action)
                }
            }
            Hit::Button(button) if action == MotionAction::Up => {
                if self.button_state.started_on.is_none() {
                    warn!("had no start of touch when ending on button");
                    return (InputStatus::Handled, Navigation::None);
                }
                if let Some(h) = self.button_state.started_on
                    && h != hit
                {
                    self.button_state.started_on = None;
                    return (InputStatus::Handled, Navigation::None);
                }
                match button {
                    Button::Number(num) => self.handle_number_input(num),

                    Button::Undo => self.undo(),
                    Button::Redo => self.redo(),

                    Button::SelectionMode => {
                        self.button_state.add_on_new_selection =
                            !self.button_state.add_on_new_selection;
                        self.sel_mode = self.button_state.get_sel_mode();
                    }
                    Button::Delete => self.handle_delete(),

                    Button::Solve => self.button_state.input_mode = InputMode::Solve,
                    Button::Center => self.button_state.input_mode = InputMode::Center,
                    Button::Corner => self.button_state.input_mode = InputMode::Corner,
                    Button::Color => self.button_state.input_mode = InputMode::Color,

                    Button::Generic(num) => {
                        self.button_state.handle_generic_input(num);
                    }
                }
            }
            Hit::Button(_) => {}
            Hit::Menu(menu) if action == MotionAction::Up => {
                if self.button_state.started_on.is_none() {
                    warn!("had no start of touch when ending on menu");
                    return (InputStatus::Handled, Navigation::None);
                }
                if let Some(h) = self.button_state.started_on
                    && h != hit
                {
                    warn!("had no start of touch when ending on menu button");
                    self.button_state.started_on = None;
                    return (InputStatus::Handled, Navigation::None);
                }
                match menu {
                    MenuItem::Pause => {
                        self.button_state.started_on = None;
                        return (InputStatus::Handled, Navigation::ToSelection(self.index));
                    }
                    MenuItem::Settings => (),
                    MenuItem::Hint => (),
                }
            }
            Hit::Menu(_) => {}
            Hit::None => match self.sel_mode {
                SelectionMode::New | SelectionMode::WithOld => {
                    self.selection = PositionBucket::new()
                }
                SelectionMode::Add | SelectionMode::Clear => (),
            },
        }
        if action == MotionAction::Up {
            self.button_state.started_on = None;
        }
        draw_state_handle.redraw(); // TODO: is this necessary?

        if self.needs_diff
            && let Some(diff) = self.sudoku.form_diff(&old_sudoku)
        {
            self.undo_stack.push(diff);
            self.needs_diff = false;
        }

        (InputStatus::Handled, Navigation::None)
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

        let num_to_highlight = self.button_state.selected_num.or_else(|| {
            if self.selection.count() != 1 {
                return None;
            }

            let pos = self.selection.into_iter().next()?;
            let cell = self.sudoku.get(pos);

            cell.given_number.or(cell.solved_number).copied()
        });

        let num_highlight = if let Some(num) = num_to_highlight {
            let mut bucket = PositionBucket::new();
            for pos in PositionBucket::all().into_iter() {
                let cell = self.sudoku.get(pos);
                if cell.contains_num(num) {
                    bucket.insert(pos);
                }
            }
            bucket
        } else {
            PositionBucket::new()
        };

        self.sudoku.render(
            canvas,
            &[
                (Swatch::Highlight, highlight),
                (Swatch::NumHighlight, num_highlight),
                (Swatch::Selection, self.selection),
            ],
            &layout.grid,
        );

        self.draw_buttons(canvas);
        self.draw_menu(canvas);
    }

    fn relayout(&mut self, bounds: Rect) {
        self.layout = Some(Layout::new(bounds));
    }

    fn lost_focus(&mut self) {
        self.enter_pause()
    }
}
