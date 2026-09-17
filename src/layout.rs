use crate::{GridPosition, Inset, Number};
use log::{info, warn};
use tiny_skia::{Color, LineJoin, Paint, PathBuilder, PixmapMut, Rect, Stroke};

pub trait IntoPaint {
    fn into_paint(&self) -> Paint<'static>;
}

impl IntoPaint for Color {
    fn into_paint(&self) -> Paint<'static> {
        let mut p = Paint::default();
        p.shader = tiny_skia::Shader::SolidColor(*self);
        p
    }
}

pub struct Palette {
    pub background: Color,

    pub grid_background: Color,
    pub grid_line: Color,
    pub grid_numbers: Color,

    pub selection: Color,
    pub highlight: Color,
    pub num_highlight: Color,

    pub button_background: Color,
    pub button_foreground: Color,
    pub button_background_active: Color,
    pub button_foreground_active: Color,

    pub wrong_number: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: unsafe { Color::from_rgba_unchecked(0.3, 0.3, 0.3, 1.0) },

            grid_background: Color::WHITE,
            grid_line: Color::BLACK,
            grid_numbers: Color::BLACK,

            selection: unsafe { Color::from_rgba_unchecked(0.8, 0.1, 0.9, 1.0) },
            highlight: unsafe { Color::from_rgba_unchecked(0.8, 0.8, 0.1, 1.0) },
            num_highlight: unsafe { Color::from_rgba_unchecked(0.2, 0.8, 0.7, 1.0) },

            button_background: Color::WHITE,
            button_foreground: unsafe { Color::from_rgba_unchecked(0.0, 0.0, 0.0, 1.0) },
            button_background_active: unsafe { Color::from_rgba_unchecked(0.5, 0.5, 0.5, 1.0) },
            button_foreground_active: Color::WHITE,

            wrong_number: unsafe { Color::from_rgba_unchecked(0.9, 0.0, 0.0, 1.0) },
        }
    }
}

const MARGIN_FACTOR: f32 = 0.05;

#[derive(Clone, Copy)]
pub struct GridLayout {
    left: f32,
    top: f32,
    size: f32,
    bold_stroke: f32,
}

impl GridLayout {
    pub fn bold_stroke(&self) -> Stroke {
        Stroke {
            width: self.bold_stroke,
            miter_limit: 4.0,
            line_cap: tiny_skia::LineCap::Square,
            line_join: LineJoin::Bevel,
            dash: None,
        }
    }
    pub fn light_stroke(&self) -> Stroke {
        Stroke {
            width: self.bold_stroke / 3.0,
            miter_limit: 4.0,
            line_cap: tiny_skia::LineCap::Square,
            line_join: LineJoin::Bevel,
            dash: None,
        }
    }
}

impl GridLayout {
    pub fn get_rect(&self, pos: GridPosition) -> Rect {
        Rect::from_xywh(
            self.left + pos.col() as f32 * self.size / 9.0,
            self.top + pos.row() as f32 * self.size / 9.0,
            self.size / 9.0,
            self.size / 9.0,
        )
        .unwrap()
    }

    pub fn hit(&self, x: f32, y: f32) -> Option<GridPosition> {
        let row = ((y - self.top) * 9.0 / self.size).floor();
        let col = ((x - self.left) * 9.0 / self.size).floor();
        if (0.0 <= row && row <= 8.0) && (0.0 <= col && col <= 8.0) {
            Some(GridPosition::new(row as u8, col as u8))
        } else {
            None
        }
    }
}

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
                    )
                    .expect("always valid"),
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
    pub fn new_portrait(window: Rect, insets: &[Inset]) -> Self {
        info!(
            "new_portrait with window: {:?}, insets: {:?}",
            window, insets
        );
        let mut drawable_area = window;
        for inset in insets {
            match inset.kind {
                crate::InsetKind::StatusBar => todo!(),
                crate::InsetKind::NavigationBar => {
                    if inset.rect.bottom() == window.bottom() {
                        if let Some(rect) = Rect::from_ltrb(
                            drawable_area.left(),
                            drawable_area.top(),
                            drawable_area.right(),
                            inset.rect.bottom(),
                        ) {
                            drawable_area = rect;
                        } else {
                            warn!("invalid drawable area from cutour")
                        };
                    } else {
                        warn!("unexpected cutout region")
                    }
                }
                crate::InsetKind::Cutout => {
                    if inset.rect.top() == 0.0 {
                        if let Some(rect) = Rect::from_ltrb(
                            drawable_area.left(),
                            inset.rect.bottom(),
                            drawable_area.right(),
                            drawable_area.bottom(),
                        ) {
                            drawable_area = rect;
                        } else {
                            warn!("invalid drawable area from cutour")
                        };
                    } else {
                        warn!("unexpected cutout region")
                    }
                }
                crate::InsetKind::CaptionBar => todo!(),
                crate::InsetKind::Waterfall => todo!(),
                crate::InsetKind::SystemBars => todo!(),
            }
        }
        let margin = drawable_area.width() * MARGIN_FACTOR;
        let size = drawable_area.width() - 2.0 * margin;
        let ui_button_size = size / (ButtonLayout::N_COLS as f32 + 1.0);
        let button_area = Rect::from_ltrb(
            margin + ui_button_size,
            drawable_area.bottom() - ButtonLayout::N_ROWS as f32 * ui_button_size - margin,
            drawable_area.right() - margin,
            drawable_area.bottom() - margin,
        )
        .expect("valid button area");
        let button_margin = ui_button_size / 20.0;
        let grid = GridLayout {
            left: drawable_area.left() + margin,
            top: button_area.top() - margin - size,
            size: size,
            bold_stroke: size / 9.0 / 20.0,
        };
        Self {
            grid,
            button: ButtonLayout {
                all: button_area,
                margin: button_margin,
            },
        }
    }

    pub fn new_landscape(window: Rect, insets: &[Inset]) -> Self {
        todo!();
        let margin = window.height() * MARGIN_FACTOR;
        let size = window.height() - 2.0 * margin;
        let grid = GridLayout {
            left: window.left() + margin,
            top: window.top() + margin,
            size: size,
            bold_stroke: 3.0,
        };
        Self {
            grid,
            button: todo!(),
        }
    }

    pub fn new(window: Rect, insets: &[Inset]) -> Self {
        if window.width() < window.height() {
            Self::new_portrait(window, insets)
        } else {
            Self::new_landscape(window, insets)
        }
    }

    pub fn draw_grid(&self, palette: &Palette, pm: &mut PixmapMut) {
        // outline
        let mut pb = PathBuilder::new();
        pb.move_to(self.grid.left, self.grid.top);
        pb.line_to(self.grid.left + self.grid.size, self.grid.top);
        pb.line_to(
            self.grid.left + self.grid.size,
            self.grid.top + self.grid.size,
        );
        pb.line_to(self.grid.left, self.grid.top + self.grid.size);
        // pb.line_to(self.grid.left, self.grid.top); // TODO necessary?
        pb.close();

        pm.stroke_path(
            &pb.finish().unwrap(),
            &palette.grid_line.into_paint(),
            &self.grid.bold_stroke(),
            tiny_skia::Transform::identity(),
            None,
        );

        // fat lines
        let mut pb = PathBuilder::new();
        for i in 1..=2 {
            pb.move_to(
                self.grid.left,
                self.grid.top + self.grid.size * (i as f32 / 3.0),
            );
            pb.line_to(
                self.grid.left + self.grid.size,
                self.grid.top + self.grid.size * (i as f32 / 3.0),
            );
        }
        for i in 1..=2 {
            pb.move_to(
                self.grid.left + self.grid.size * (i as f32 / 3.0),
                self.grid.top,
            );
            pb.line_to(
                self.grid.left + self.grid.size * (i as f32 / 3.0),
                self.grid.top + self.grid.size,
            );
        }

        pm.stroke_path(
            &pb.finish().unwrap(),
            &palette.grid_line.into_paint(),
            &self.grid.bold_stroke(),
            tiny_skia::Transform::identity(),
            None,
        );

        // horizontal
        let mut pb = PathBuilder::new();
        for i in 0..3 {
            for j in 1..3 {
                pb.move_to(
                    self.grid.left,
                    self.grid.top + self.grid.size * (i as f32 / 3.0 + j as f32 / 9.0),
                );
                pb.line_to(
                    self.grid.left + self.grid.size,
                    self.grid.top + self.grid.size * (i as f32 / 3.0 + j as f32 / 9.0),
                );
            }
        }

        // vertical
        for i in 0..3 {
            for j in 1..3 {
                pb.move_to(
                    self.grid.left + self.grid.size * (i as f32 / 3.0 + j as f32 / 9.0),
                    self.grid.top,
                );
                pb.line_to(
                    self.grid.left + self.grid.size * (i as f32 / 3.0 + j as f32 / 9.0),
                    self.grid.top + self.grid.size,
                );
            }
        }
        pm.stroke_path(
            &pb.finish().unwrap(),
            &palette.grid_line.into_paint(),
            &self.grid.light_stroke(),
            tiny_skia::Transform::identity(),
            None,
        );
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

    pub fn get_cell_rect(&self, pos: GridPosition) -> Rect {
        self.grid.get_rect(pos)
    }

    pub fn get_grid_rect(&self) -> Rect {
        Rect::from_xywh(
            self.grid.left,
            self.grid.top,
            self.grid.size,
            self.grid.size,
        )
        .unwrap()
    }

    pub fn get_button_rects(&self) -> Vec<(Rect, Button)> {
        self.button.get_button_rects()
    }
}
