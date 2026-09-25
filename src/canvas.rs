use enum_map::{Enum, EnumMap, enum_map};
use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, PixmapMut, Stroke, Transform};
use ttf_parser::{Face, OutlineBuilder};

use crate::{Number, NumberBucket};

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

#[repr(u8)]
#[derive(Debug, Clone, Copy, Enum)]
pub enum Swatch {
    Background,
    UiColor,
    UiColorInActive,

    GridBackground,
    GridLine,
    GridNumbers,

    Selection,
    Highlight,
    NumHighlight,

    ButtonBackground,
    ButtonForeground,
    ButtonBackgroundActive,
    ButtonForegroundActive,

    WrongNumber,
}

#[derive(Clone)] // TODO remove Clone implementation
pub struct Palette(EnumMap<Swatch, Color>);

impl Default for Palette {
    fn default() -> Self {
        // SAFETY: All color components below are compile-time constants within [0.0, 1.0]
        // and alpha is 1.0 (so color channels are trivially valid pre-multiplied values).
        Self(enum_map! {
            Swatch::Background=> unsafe { Color::from_rgba_unchecked(0.3, 0.3, 0.3, 1.0) },
            Swatch::UiColor => Color::WHITE,
            Swatch::UiColorInActive => unsafe { Color::from_rgba_unchecked(0.5, 0.5, 0.5, 1.0) },

            Swatch::GridBackground=> Color::WHITE,
            Swatch::GridLine=> Color::BLACK,
            Swatch::GridNumbers=> Color::BLACK,

            Swatch::Selection=> unsafe { Color::from_rgba_unchecked(0.8, 0.1, 0.9, 1.0) },
            Swatch::Highlight=> unsafe { Color::from_rgba_unchecked(0.8, 0.8, 0.1, 1.0) },
            Swatch::NumHighlight=> unsafe { Color::from_rgba_unchecked(0.2, 0.8, 0.7, 1.0) },

            Swatch::ButtonBackground=> Color::WHITE,
            Swatch::ButtonForeground=> unsafe { Color::from_rgba_unchecked(0.0, 0.0, 0.0, 1.0) },
            Swatch::ButtonBackgroundActive=> unsafe { Color::from_rgba_unchecked(0.5, 0.5, 0.5, 1.0) },
            Swatch::ButtonForegroundActive=> Color::WHITE,

            Swatch::WrongNumber=> unsafe { Color::from_rgba_unchecked(0.9, 0.0, 0.0, 1.0) },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Into<tiny_skia::Rect> for Rect {
    fn into(self) -> tiny_skia::Rect {
        let Self {
            left,
            top,
            right,
            bottom,
        } = self;
        tiny_skia::Rect::from_ltrb(left, top, right, bottom).expect("expected valid rect")
    }
}

impl Rect {
    pub fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    pub fn left(&self) -> f32 {
        self.left
    }

    pub fn top(&self) -> f32 {
        self.top
    }

    pub fn right(&self) -> f32 {
        self.right
    }

    pub fn bottom(&self) -> f32 {
        self.bottom
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        (self.left < x && x < self.right) && (self.top < y && y < self.bottom)
    }

    pub fn shrink(&self, rad: f32) -> Self {
        Self {
            left: self.left + rad,
            top: self.top + rad,
            right: self.right - rad,
            bottom: self.bottom - rad,
        }
    }

    fn make_rounded_path(&self, rad: f32) -> Path {
        let mut pb = PathBuilder::new();
        pb.move_to(self.left() + rad, self.top());

        pb.line_to(self.right() - rad, self.top());
        pb.quad_to(self.right(), self.top(), self.right(), self.top() + rad);

        pb.line_to(self.right(), self.bottom() - rad);
        pb.quad_to(
            self.right(),
            self.bottom(),
            self.right() - rad,
            self.bottom(),
        );

        pb.line_to(self.left() + rad, self.bottom());
        pb.quad_to(self.left(), self.bottom(), self.left(), self.bottom() - rad);

        pb.line_to(self.left(), self.top() + rad);
        pb.quad_to(self.left(), self.top(), self.left() + rad, self.top());

        pb.close();
        pb.finish().expect("always valid path")
    }
}

const CELL_MARGIN: f32 = 0.2; // TODO remove

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

#[derive(Clone)]
pub struct FaceBook(pub Vec<(String, Face<'static>)>);

type GlyphId = (usize, ttf_parser::GlyphId);

impl FaceBook {
    pub fn glyph_index(&self, c: char) -> Option<GlyphId> {
        for (i, (_name, face)) in self.0.iter().enumerate() {
            if let Some(id) = face.glyph_index(c) {
                return Some((i, id));
            }
        }
        None
    }

    pub fn outline_glyph(&self, id: GlyphId, builder: &mut SkiaOutlineBuilder) {
        self.0[id.0].1.outline_glyph(id.1, builder);
    }

    pub fn capital_height(&self, id: GlyphId) -> Option<i16> {
        self.0[id.0].1.capital_height()
    }

    pub fn glyph_hor_advance(&self, id: GlyphId) -> Option<u16> {
        self.0[id.0].1.glyph_hor_advance(id.1)
    }
}

pub struct Canvas<'a> {
    pixmap: PixmapMut<'a>,
    pub palette: Palette, // TODO remove pub after done
    face_book: FaceBook,
}

impl<'a> Canvas<'a> {
    pub fn new(face_book: FaceBook, pixmap: PixmapMut<'a>, palette: Palette) -> Self {
        Self {
            pixmap,
            face_book,
            palette,
        }
    }
}

impl Canvas<'_> {
    pub fn fill(&mut self, color: Swatch) {
        self.pixmap.fill(self.palette.0[color]);
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Swatch) {
        self.pixmap.fill_rect(
            rect.into(),
            &self.palette.0[color].into_paint(),
            Transform::identity(),
            None,
        );
    }

    pub fn fill_rect_rounded(&mut self, rect: Rect, radius: f32, color: Swatch) {
        self.pixmap.fill_path(
            &rect.make_rounded_path(radius),
            &self.palette.0[color].into_paint(),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    pub fn stroke_path(&mut self, path: &Path, color: Swatch, stroke: &Stroke) {
        self.pixmap.stroke_path(
            &path,
            &self.palette.0[color].into_paint(),
            &stroke,
            Transform::identity(),
            None,
        );
    }
}

impl Canvas<'_> {
    pub fn draw_center_notes(&mut self, rect: Rect, notes: NumberBucket, color: Swatch) {
        let paths = notes
            .into_iter()
            .map(|n| self.make_char_path(n.as_char()))
            .collect::<Vec<_>>();
        let total_advance: f32 = paths.iter().map(|p| p.advance).sum();
        let cap_height = paths.first().unwrap().cap_height;
        let margin = rect.height() / 3.0 * CELL_MARGIN;
        let scale = ((rect.height() / 3.0 - 2.0 * margin) / cap_height)
            .min((rect.width() - 2.0 * margin) / total_advance);
        let mut start_x = rect.left() + rect.width() / 2.0 - total_advance * scale / 2.0;
        let ground_line = rect.top() + rect.height() / 2.0 + cap_height * scale / 2.0;
        for CharPath { path, advance, .. } in paths {
            self.pixmap.fill_path(
                &path,
                &self.palette.0[color].into_paint(),
                FillRule::Winding,
                Transform::from_scale(scale, -scale).post_translate(start_x, ground_line),
                None,
            );
            start_x += advance * scale;
        }
    }

    pub fn draw_corner_notes(&mut self, rect: Rect, notes: NumberBucket, color: Swatch) {
        let paths = notes
            .into_iter()
            .map(|n| self.make_char_path(n.as_char()))
            .collect::<Vec<_>>();

        let (top, bottom): (&[CharPath], &[CharPath]) = if paths.len() <= 2 {
            (&paths[..], &[])
        } else {
            let len = paths.len();
            let bottom_len = len / 2;
            (&paths[..len - bottom_len], &paths[len - bottom_len..])
        };
        let margin = rect.height() / 3.0 * CELL_MARGIN;

        let top_advance: f32 = top.iter().map(|p| p.advance).sum();
        let bottom_advance: f32 = bottom.iter().map(|p| p.advance).sum();

        let cap_height = paths.first().unwrap().cap_height;

        let scale = if bottom.is_empty() {
            ((rect.height() / 3.0 - 2.0 * margin) / cap_height)
                .min((rect.width() - 2.0 * margin) / top_advance)
        } else {
            ((rect.height() / 3.0 - 2.0 * margin) / cap_height)
                .min((rect.width() - 2.0 * margin) / top_advance)
                .min((rect.width() - 2.0 * margin) / bottom_advance)
        };

        let top_spacing = (rect.width() - top_advance * scale - 2.0 * margin)
            / (top.len().max(1) - 1).max(1) as f32;
        let mut start_x = rect.left() + margin;

        let ground_line = rect.top() + cap_height * scale + margin;
        for CharPath { path, advance, .. } in top {
            self.pixmap.fill_path(
                &path,
                &self.palette.0[color].into_paint(),
                FillRule::Winding,
                Transform::from_scale(scale, -scale).post_translate(start_x, ground_line),
                None,
            );
            start_x += advance * scale + top_spacing;
        }

        let bottom_spacing = (rect.width() - bottom_advance * scale - 2.0 * margin)
            / (bottom.len().max(1) - 1).max(1) as f32;
        let mut start_x = rect.left() + margin;

        let ground_line = rect.bottom() - margin;
        for CharPath { path, advance, .. } in bottom {
            self.pixmap.fill_path(
                &path,
                &self.palette.0[color].into_paint(),
                FillRule::Winding,
                Transform::from_scale(scale, -scale).post_translate(start_x, ground_line),
                None,
            );
            start_x += advance * scale + bottom_spacing;
        }
    }

    pub fn draw_num(&mut self, num: Number, rect: Rect, color: Swatch) {
        self.draw_char(num.as_char(), rect, color);
    }

    pub fn draw_char(&mut self, c: char, rect: Rect, color: Swatch) {
        let l = self.make_char_path(c);
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
        self.pixmap.fill_path(
            &l.path,
            &self.palette.0[color].into_paint(),
            FillRule::Winding,
            transform,
            None,
        );
    }

    pub fn make_char_path(&self, c: char) -> CharPath {
        let glyph_id = self
            .face_book
            .glyph_index(c)
            .expect(&format!("{} does not exist", c));
        let mut builder = SkiaOutlineBuilder(PathBuilder::new());
        self.face_book.outline_glyph(glyph_id, &mut builder);
        let path = builder.0.finish().expect("path should always be valid");

        let cap_height = self
            .face_book
            .capital_height(glyph_id)
            .expect("Face should have capital height") as f32;

        CharPath {
            path,
            advance: self
                .face_book
                .glyph_hor_advance(glyph_id)
                .expect("every glyph has advance") as f32,
            cap_height,
        }
    }
}

pub struct CharPath {
    path: Path,
    advance: f32,
    // TODO remove fields below
    cap_height: f32,
}
