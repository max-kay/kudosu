use crate::{GridPosition, Number, Rect, sudoku_types::GridLayout};

const MARGIN_FACTOR: f32 = 0.05;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ButtonLayout {
    all: Rect,
    margin: f32,
}

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    Generic(u8),
}

impl ButtonLayout {
    const N_COLS: usize = 5;
    const N_ROWS: usize = 4;
    const BUTTONS: [Button; 4 * 5] = [
        Button::Redo,               // (0, 0)
        Button::Number(Number::N1), // (1, 0)
        Button::Number(Number::N2), // (2, 0)
        Button::Number(Number::N3), // (3, 0)
        Button::Solve,              // (4, 0)
        Button::Undo,               // (0, 1)
        Button::Number(Number::N4), // (1, 1)
        Button::Number(Number::N5), // (2, 1)
        Button::Number(Number::N6), // (3, 1)
        Button::Corner,             // (4, 1)
        Button::SelectionMode,      // (0, 2)
        Button::Number(Number::N7), // (1, 2)
        Button::Number(Number::N8), // (2, 2)
        Button::Number(Number::N9), // (3, 2)
        Button::Center,             // (4, 2)
        Button::Delete,             // (0, 3)
        Button::Generic(1),         // (1, 3)
        Button::Generic(2),         // (2, 3)
        Button::Generic(3),         // (3, 3)
        Button::Color,              // (4, 3)
    ];

    pub fn hit(&self, x: f32, y: f32) -> Option<Button> {
        let norm_x = ((x - self.all.left()) / self.all.width() * Self::N_COLS as f32).floor();
        let norm_y = ((y - self.all.top()) / self.all.height() * Self::N_ROWS as f32).floor();
        if !(0.0 <= norm_x && norm_x < Self::N_COLS as f32) {
            return None;
        }
        if !(0.0 <= norm_y && norm_y < Self::N_ROWS as f32) {
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

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct MenuLayout {
    pub pause: Rect,
    pub settings: Rect,
    pub hint: Rect,
}

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MenuItem {
    Pause,
    Settings,
    Hint,
}

impl MenuLayout {
    fn hit(&self, x: f32, y: f32) -> Option<MenuItem> {
        if self.pause.contains(x, y) {
            return Some(MenuItem::Pause);
        }
        if self.settings.contains(x, y) {
            return Some(MenuItem::Settings);
        }
        if self.hint.contains(x, y) {
            return Some(MenuItem::Hint);
        }
        return None;
    }
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    pub window: Rect,
    pub grid: GridLayout,
    pub button: ButtonLayout,
    pub menu: MenuLayout,
}

impl Layout {
    pub fn new_portrait(window: Rect) -> Self {
        let margin = window.width() * MARGIN_FACTOR;
        let size = window.width() - 2.0 * margin;
        let ui_button_size = size / (ButtonLayout::N_COLS as f32 + 1.0);

        let menu_top = window.bottom() - ButtonLayout::N_ROWS as f32 * ui_button_size - margin;
        let menu_bottom = window.bottom() - margin;
        let menu_left = window.left() + margin;
        let menu_right = menu_left + ui_button_size;

        let button_area =
            Rect::from_ltrb(menu_right, menu_top, window.right() - margin, menu_bottom);
        let button_margin = ui_button_size / 20.0;

        let grid = GridLayout {
            left: window.left() + margin,
            top: button_area.top() - margin - size,
            size,
        };

        // Corrected Menu Calculations
        let menu_width = menu_right - menu_left;
        let menu_height = menu_bottom - menu_top;
        let menu_size = menu_width.min(menu_height / 3.0);

        let menu_center_x = (menu_left + menu_right) / 2.0;
        // Base center Y for the top menu item (pause)
        let start_y = menu_top + menu_size / 2.0;
        let menu_shrink = menu_size * super::MENU_MARGIN;

        Self {
            window,
            grid,
            button: ButtonLayout {
                all: button_area,
                margin: button_margin,
            },
            menu: MenuLayout {
                pause: Rect::square_from_center_side(menu_center_x, start_y, menu_size)
                    .shrink(menu_shrink),

                settings: Rect::square_from_center_side(
                    menu_center_x,
                    start_y + menu_size,
                    menu_size,
                )
                .shrink(menu_shrink),

                hint: Rect::square_from_center_side(
                    menu_center_x,
                    start_y + 2.0 * menu_size,
                    menu_size,
                )
                .shrink(menu_shrink),
            },
        }
    }

    pub fn new_landscape(window: Rect) -> Self {
        let margin = window.height() * MARGIN_FACTOR;
        let grid_size = window.height() - 2.0 * margin;
        let ui_button_size =
            (window.width() - 4.0 * margin - grid_size) / (ButtonLayout::N_COLS as f32 + 1.0);

        let menu_left = window.left() + 2.0 * margin + grid_size;
        let menu_right = menu_left + ui_button_size;
        let menu_top = window.bottom() - margin - ui_button_size * ButtonLayout::N_ROWS as f32;
        let menu_bottom = window.bottom() - margin;

        let button_area =
            Rect::from_ltrb(menu_right, menu_top, window.right() - margin, menu_bottom);
        let button_margin = ui_button_size / 20.0;

        let grid = GridLayout {
            left: window.left() + margin,
            top: window.top() + margin,
            size: grid_size,
        };

        let menu_width = menu_right - menu_left;
        let menu_height = menu_bottom - menu_top;
        let menu_size = menu_width.min(menu_height / 3.0);

        let menu_center_x = (menu_left + menu_right) / 2.0;
        let start_y = menu_top + menu_size / 2.0;
        let menu_shrink = menu_size * super::MENU_MARGIN;

        Self {
            window,
            grid,
            button: ButtonLayout {
                all: button_area,
                margin: button_margin,
            },
            menu: MenuLayout {
                pause: Rect::square_from_center_side(menu_center_x, start_y, menu_size)
                    .shrink(menu_shrink),

                settings: Rect::square_from_center_side(
                    menu_center_x,
                    start_y + menu_size,
                    menu_size,
                )
                .shrink(menu_shrink),

                hint: Rect::square_from_center_side(
                    menu_center_x,
                    start_y + 2.0 * menu_size,
                    menu_size,
                )
                .shrink(menu_shrink),
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

#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Hit {
    Cell(GridPosition),
    Button(Button),
    Menu(MenuItem),
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
        if let Some(men) = self.menu.hit(x, y) {
            return Hit::Menu(men);
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
