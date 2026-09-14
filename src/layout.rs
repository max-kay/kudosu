use crate::GridPosition;
use android_activity::ndk::native_window::NativeWindow;
use tiny_skia::{Color, LineJoin, Paint, PathBuilder, PixmapMut, Rect, Stroke};

pub struct Palette {
    pub background: Color,
    pub grid_background: Color,
    pub grid_line_color: Color,
    pub text_color: Color,
    pub highlight_color: Color,
    pub selection_color: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: unsafe { Color::from_rgba_unchecked(0.3, 0.3, 0.3, 1.0) },
            grid_background: Color::WHITE,
            grid_line_color: Color::BLACK,
            text_color: Color::BLACK,
            highlight_color: unsafe { Color::from_rgba_unchecked(0.8, 0.8, 0.1, 1.0) },
            selection_color: unsafe { Color::from_rgba_unchecked(0.8, 0.1, 0.9, 1.0) },
        }
    }
}

impl Palette {
    pub fn grid_background_paint(&self) -> Paint<'static> {
        let mut p = Paint::default();
        p.shader = tiny_skia::Shader::SolidColor(self.grid_background);
        p
    }

    pub fn grid_line_paint(&self) -> Paint<'static> {
        let mut p = Paint::default();
        p.shader = tiny_skia::Shader::SolidColor(self.grid_line_color);
        p
    }

    pub fn text_paint(&self) -> Paint<'static> {
        let mut p = Paint::default();
        p.shader = tiny_skia::Shader::SolidColor(self.text_color);
        p
    }

    pub fn highlight_paint(&self) -> Paint<'static> {
        let mut p = Paint::default();
        p.shader = tiny_skia::Shader::SolidColor(self.highlight_color);
        p
    }
    pub fn selection_paint(&self) -> Paint<'static> {
        let mut p = Paint::default();
        p.shader = tiny_skia::Shader::SolidColor(self.selection_color);
        p
    }
}

const MARGIN_FACTOR: f32 = 0.05;

#[derive(Default)]
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
        if (1.0 <= row && row <= 9.0) && (1.0 <= col && col <= 9.0) {
            Some(GridPosition::new(row as u8, col as u8))
        } else {
            None
        }
    }
}

#[derive(Default)]
struct ButtonLayout {}

#[derive(Default)]
pub struct Layout {
    grid: GridLayout,
    button: ButtonLayout,
}

impl Layout {
    pub fn new(window: &NativeWindow) -> Self {
        let width = window.width() as f32;

        let grid_size = (1.0 - 2.0 * MARGIN_FACTOR) * width;
        Self {
            grid: GridLayout {
                top: (width - grid_size) / 2.0, // TODO proper top margin
                left: (width - grid_size) / 2.0,
                size: grid_size,
                bold_stroke: 5.0, // TODO proper stroke calculation
            },
            button: ButtonLayout {},
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
            &palette.grid_line_paint(),
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
            &palette.grid_line_paint(),
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
            &palette.grid_line_paint(),
            &self.grid.light_stroke(),
            tiny_skia::Transform::identity(),
            None,
        );
    }
}

pub enum Button {
    Number(u8),
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
        return Hit::None;
    }

    pub fn get_rect(&self, pos: GridPosition) -> Rect {
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
}
