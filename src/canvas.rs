use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Mutex,
};

use enum_map::{Enum, EnumMap, enum_map};
use log::info;
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

impl From<tiny_skia::Rect> for Rect {
    fn from(value: tiny_skia::Rect) -> Self {
        Self::from_ltrb(value.left(), value.top(), value.right(), value.bottom())
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

    pub fn square_from_center_side(c_x: f32, c_y: f32, side: f32) -> Self {
        Self {
            left: c_x - side / 2.0,
            top: c_y - side / 2.0,
            right: c_x + side / 2.0,
            bottom: c_y + side / 2.0,
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

    pub fn center(&self) -> (f32, f32) {
        (
            (self.left + self.right) / 2.0,
            (self.top + self.bottom) / 2.0,
        )
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

pub struct FaceBook<'a> {
    faces: Vec<(String, Face<'a>)>,
    number_paths: [CharPath; 9],
    char_paths: Mutex<HashMap<char, CharPath>>,
}

type GlyphId = (usize, ttf_parser::GlyphId);

impl<'a> FaceBook<'a> {
    pub fn new(faces: Vec<(String, Face<'a>)>) -> Self {
        let number_paths = std::array::from_fn(|i| {
            let c = (b'1' + i as u8) as char;
            Self::build_char_path(&faces, c)
        });
        Self {
            faces,
            number_paths,
            char_paths: Mutex::new(HashMap::new()),
        }
    }

    pub fn number_path(&self, num: Number) -> &CharPath {
        &self.number_paths[(num.as_u8() - 1) as usize]
    }

    fn glyph_index_in(faces: &[(String, Face<'a>)], c: char) -> Option<GlyphId> {
        for (i, (_name, face)) in faces.iter().enumerate() {
            if let Some(id) = face.glyph_index(c) {
                return Some((i, id));
            }
        }
        None
    }

    fn build_char_path(faces: &[(String, Face<'a>)], c: char) -> CharPath {
        let glyph_id = Self::glyph_index_in(faces, c)
            .unwrap_or_else(|| panic!("glyph for character `{}` not found in any font", c));
        let face = &faces[glyph_id.0].1;
        let mut builder = SkiaOutlineBuilder(PathBuilder::new());
        face.outline_glyph(glyph_id.1, &mut builder);
        let path = builder.0.finish().expect("path should always be valid");
        let cap_height = face
            .capital_height()
            .or_else(|| face.ascender().into())
            .unwrap_or(face.height() as i16) as f32;
        let advance = face
            .glyph_hor_advance(glyph_id.1)
            .expect("every glyph has advance") as f32;

        CharPath {
            path,
            advance,
            cap_height,
        }
    }

    pub fn make_char_path(&self, c: char) -> CharPath {
        if ('1'..='9').contains(&c) {
            let idx = (c as u8 - b'1') as usize;
            return self.number_paths[idx].clone();
        }
        let mut char_map = self.char_paths.lock().expect("never poisoned");
        match char_map.entry(c) {
            Entry::Occupied(occupied_entry) => occupied_entry.get().clone(),
            Entry::Vacant(vacant_entry) => {
                let path = Self::build_char_path(&self.faces, c);
                vacant_entry.insert(path.clone());
                path
            }
        }
    }
}

pub struct Canvas<'a> {
    pixmap: PixmapMut<'a>,
    pub palette: Palette, // TODO remove pub after done
    face_book: &'a FaceBook<'a>,
}

impl<'a> Canvas<'a> {
    pub fn new(face_book: &'a FaceBook<'a>, pixmap: PixmapMut<'a>, palette: Palette) -> Self {
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

    pub fn outline_rect_rounded(
        &mut self,
        rect: Rect,
        radius: f32,
        color: Swatch,
        stroke: &Stroke,
    ) {
        let path = rect.make_rounded_path(radius);
        self.pixmap.stroke_path(
            &path,
            &self.palette.0[color].into_paint(),
            stroke,
            Transform::identity(),
            None,
        );
    }
}

impl Canvas<'_> {
    pub fn draw_center_notes(&mut self, rect: Rect, notes: NumberBucket, color: Swatch) {
        let paths = notes
            .into_iter()
            .map(|n| self.face_book.number_path(n))
            .collect::<Vec<_>>();
        let total_advance: f32 = paths.iter().map(|p| p.advance).sum();
        let cap_height = paths.first().unwrap().cap_height;
        let margin = rect.height() / 3.0 * CELL_MARGIN;
        let scale = ((rect.height() / 3.0 - 2.0 * margin) / cap_height)
            .min((rect.width() - 2.0 * margin) / total_advance);
        let mut start_x = rect.left() + rect.width() / 2.0 - total_advance * scale / 2.0;
        let ground_line = rect.top() + rect.height() / 2.0 + cap_height * scale / 2.0;
        for p in paths {
            self.pixmap.fill_path(
                &p.path,
                &self.palette.0[color].into_paint(),
                FillRule::Winding,
                Transform::from_scale(scale, -scale).post_translate(start_x, ground_line),
                None,
            );
            start_x += p.advance * scale;
        }
    }

    pub fn draw_corner_notes(&mut self, rect: Rect, notes: NumberBucket, color: Swatch) {
        let paths = notes
            .into_iter()
            .map(|n| self.face_book.number_path(n))
            .collect::<Vec<_>>();

        let (top, bottom): (&[&CharPath], &[&CharPath]) = if paths.len() <= 2 {
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
        for p in top {
            self.pixmap.fill_path(
                &p.path,
                &self.palette.0[color].into_paint(),
                FillRule::Winding,
                Transform::from_scale(scale, -scale).post_translate(start_x, ground_line),
                None,
            );
            start_x += p.advance * scale + top_spacing;
        }

        let bottom_spacing = (rect.width() - bottom_advance * scale - 2.0 * margin)
            / (bottom.len().max(1) - 1).max(1) as f32;
        let mut start_x = rect.left() + margin;

        let ground_line = rect.bottom() - margin;
        for p in bottom {
            self.pixmap.fill_path(
                &p.path,
                &self.palette.0[color].into_paint(),
                FillRule::Winding,
                Transform::from_scale(scale, -scale).post_translate(start_x, ground_line),
                None,
            );
            start_x += p.advance * scale + bottom_spacing;
        }
    }

    pub fn draw_num(&mut self, num: Number, rect: Rect, color: Swatch) {
        let l = self.face_book.number_path(num);
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

    pub fn draw_char_centered(&mut self, c: char, rect: Rect, color: Swatch) {
        let l = self.make_char_path(c);
        let center_y = rect.top() + rect.height() / 2.0;
        let center_x = rect.left() + rect.width() / 2.0;

        let bounds: Rect = l.path.bounds().into();
        let (sym_center_x, sym_center_y) = bounds.center();
        let scale = (rect.width() / bounds.width()).min(rect.height() / bounds.height());

        let transform = Transform::from_scale(scale, -scale).post_translate(
            center_x - sym_center_x * scale,
            center_y + sym_center_y * scale,
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
        self.face_book.make_char_path(c)
    }
}

#[derive(Clone, Debug)]
pub struct CharPath {
    pub path: Path,
    pub advance: f32,
    // TODO remove fields below
    pub cap_height: f32,
}
