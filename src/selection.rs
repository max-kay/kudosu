use std::str::FromStr;

use android_activity::input::MotionAction;
use android_activity::{InputStatus, input::InputEvent};
use log::warn;

use crate::SUDOKUS;
use crate::canvas::Swatch;
use crate::{
    Component, DrawState, Navigation, PositionBucket, Rect, Sudoku, canvas::Canvas,
    sudoku_types::GridLayout,
};

pub struct SelectionScreen {
    sudoku_index: usize,
    sudoku_cache: Sudoku,
    layout: Option<Layout>,
}

impl SelectionScreen {
    pub fn new(mut i: usize) -> Self {
        i %= SUDOKUS.len();
        Self {
            sudoku_cache: Sudoku::new(
                FromStr::from_str(SUDOKUS[i].0).unwrap(),
                FromStr::from_str(SUDOKUS[i].1).unwrap(),
            ),
            sudoku_index: i,
            layout: None,
        }
    }
}

impl Component for SelectionScreen {
    fn handle_input(
        &mut self,
        input: &InputEvent,
        draw_state_handle: &mut DrawState,
    ) -> (InputStatus, Navigation) {
        let layout = if let Some(l) = self.layout.as_ref() {
            l
        } else {
            return (InputStatus::Unhandled, Navigation::None);
        };
        if let InputEvent::MotionEvent(motion_event) = input {
            if matches!(motion_event.action(), MotionAction::Up) {
                let pointer = motion_event.pointer_at_index(0);
                let x = pointer.x();
                let y = pointer.y();
                if layout.left_button.contains(x, y) {
                    self.sudoku_index += SUDOKUS.len() - 1;
                    self.sudoku_index %= SUDOKUS.len();
                    draw_state_handle.redraw();
                    self.sudoku_cache = Sudoku::new(
                        FromStr::from_str(SUDOKUS[self.sudoku_index].0).unwrap(),
                        FromStr::from_str(SUDOKUS[self.sudoku_index].1).unwrap(),
                    )
                } else if layout.right_button.contains(x, y) {
                    self.sudoku_index += 1;
                    self.sudoku_index %= SUDOKUS.len();
                    draw_state_handle.redraw();
                    self.sudoku_cache = Sudoku::new(
                        FromStr::from_str(SUDOKUS[self.sudoku_index].0).unwrap(),
                        FromStr::from_str(SUDOKUS[self.sudoku_index].1).unwrap(),
                    )
                } else {
                    return (
                        InputStatus::Handled,
                        Navigation::ToSolving(self.sudoku_index),
                    );
                }
            }
        }
        return (InputStatus::Handled, Navigation::None);
    }

    fn render_frame(&self, canvas: &mut Canvas<'_>) {
        let layout = if let Some(l) = &self.layout {
            l
        } else {
            warn!("had no layout");
            return;
        };
        canvas.fill(Swatch::Background);
        self.sudoku_cache.render(canvas, &[], &layout.grid);
    }

    fn relayout(&mut self, bounds: Rect) {
        self.layout = Some(Layout::new(bounds));
    }
}

pub struct Layout {
    grid: GridLayout,
    left_button: Rect,
    right_button: Rect,
}

const MARGIN: f32 = 0.1;

impl Layout {
    fn new(bounds: Rect) -> Self {
        let size = bounds.width().min(bounds.height()) * (1.0 - 2.0 * MARGIN);
        let x_margin = bounds.left() + bounds.width() / 2.0 - size / 2.0;
        Layout {
            grid: GridLayout {
                left: x_margin,
                top: bounds.top() + bounds.height() / 2.0 - size / 2.0,
                size,
            },
            left_button: Rect::from_ltrb(bounds.left(), bounds.top(), x_margin, bounds.bottom()),
            right_button: Rect::from_ltrb(
                bounds.right() - x_margin,
                bounds.top(),
                bounds.right(),
                bounds.bottom(),
            ),
        }
    }
}
