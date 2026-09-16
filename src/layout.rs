use crate::{GridPosition, Inset, insets};
use android_activity::Rect as AndroidRect;
use android_activity::ndk::native_window::NativeWindow;
use log::{info, warn};
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
        if (0.0 <= row && row <= 8.0) && (0.0 <= col && col <= 8.0) {
            Some(GridPosition::new(row as u8, col as u8))
        } else {
            None
        }
    }
}

#[derive(Default)]
struct ButtonLayout {
    num_square_top: f32,
    num_square_left: f32,
    num_square_size: f32,
}

#[derive(Default)]
pub struct Layout {
    grid: GridLayout,
    button: ButtonLayout,
}

impl Layout {
    pub fn new_portrait(window: Rect, insets: &[Inset]) -> Self {
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
        let margin = window.width() * MARGIN_FACTOR;
        let size = window.width() - 2.0 * margin;
        let grid = GridLayout {
            left: window.left() + margin,
            top: window.top() + margin,
            size: size,
            bold_stroke: 3.0,
        };
        Self {
            grid,
            button: ButtonLayout {
                ..Default::default()
            },
        }
    }

    pub fn new_landscape(window: Rect, insets: &[Inset]) -> Self {
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
            button: ButtonLayout {
                ..Default::default()
            },
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
    Solid,
    Center,
    Corner,
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
