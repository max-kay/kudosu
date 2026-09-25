use core::panic;
use std::{
    error::Error,
    fmt::Display,
    num::NonZeroU8,
    ops::{BitAnd, BitOr, BitXor, BitXorAssign, Index, IndexMut, Not},
    str::FromStr,
};

use log::info;
use tiny_skia::{LineCap, LineJoin, PathBuilder, Stroke};

use crate::canvas::Rect;
use crate::canvas::{Canvas, Swatch};

mod iter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition(pub u8);

impl GridPosition {
    pub fn new(row: u8, col: u8) -> Self {
        assert!(row < 9, "Gridposition with invalid row: `{}`", row);
        assert!(col < 9, "Gridposition with invalid col: `{}`", col);
        Self(row * 9 + col)
    }

    pub fn row(&self) -> u8 {
        self.0 / 9
    }

    pub fn col(&self) -> u8 {
        self.0 % 9
    }

    pub fn box_(&self) -> u8 {
        (self.row() / 3) * 3 + self.col() / 3
    }

    pub fn sees_by_sudoku(&self) -> PositionBucket {
        let mut bucket = PositionBucket::col(self.col())
            | PositionBucket::row(self.row())
            | PositionBucket::box_(self.box_());
        bucket.remove(*self);
        bucket
    }

    fn as_mask(&self) -> u128 {
        1 << self.0
    }
}

impl From<GridPosition> for PositionBucket {
    fn from(value: GridPosition) -> Self {
        PositionBucket(value.as_mask())
    }
}

#[derive(Clone, Copy)]
pub struct PositionBucket(u128);

impl PositionBucket {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn all() -> Self {
        Self((1 << 81) - 1)
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn row(num: u8) -> Self {
        if num >= 9 {
            panic!("row num out of range")
        }
        let mut acc = 0;
        let mut pointer = 1 << (num * 9);
        for _ in 0..9 {
            acc |= pointer;
            pointer <<= 1;
        }
        Self(acc)
    }

    pub fn col(num: u8) -> Self {
        if num >= 9 {
            panic!("col num out of range")
        }
        let mut acc = 0;
        let mut pointer = 1 << num;
        for _ in 0..9 {
            acc |= pointer;
            pointer <<= 9;
        }
        Self(acc)
    }

    pub fn box_(num: u8) -> Self {
        if num >= 9 {
            panic!("box num out of range");
        }
        let b_row = num / 3;
        let b_col = num % 3;
        let mut pointers = 0b111 << (9 * 3 * b_row + 3 * b_col);
        let mut acc = 0;
        for _ in 0..3 {
            acc |= pointers;
            pointers <<= 9;
        }
        Self(acc)
    }

    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn insert(&mut self, pos: GridPosition) {
        self.0 = self.0 | pos.as_mask();
    }

    pub fn remove(&mut self, pos: GridPosition) {
        self.0 = self.0 & !(pos.as_mask())
    }

    pub fn contains(&self, pos: GridPosition) -> bool {
        (self.0 & pos.as_mask()) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl BitAnd for PositionBucket {
    type Output = PositionBucket;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for PositionBucket {
    type Output = PositionBucket;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitXor for PositionBucket {
    type Output = PositionBucket;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for PositionBucket {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl Not for PositionBucket {
    type Output = PositionBucket;

    fn not(self) -> Self::Output {
        PositionBucket::all() ^ self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Number(pub(super) NonZeroU8);

impl Number {
    pub fn new(val: u8) -> Self {
        if 1 <= val && val <= 9 {
            // SAFETY: `val` was verified to be in 1..=9, so it is strictly non-zero.
            unsafe { Self(NonZeroU8::new_unchecked(val)) }
        } else {
            panic!("tried to create a number out of range `{}`", val)
        }
    }

    fn as_mask(&self) -> u16 {
        1 << (self.as_u8() - 1)
    }

    pub fn as_char(&self) -> char {
        match self.as_u8() {
            1..=9 => (b'0' + self.as_u8()) as char,
            _ => unreachable!(),
        }
    }

    fn as_u8(&self) -> u8 {
        self.0.into()
    }
}

impl Number {
    // SAFETY: The literal constants 1 through 9 are all non-zero, satisfying `NonZeroU8` invariant.
    pub const N1: Self = Self(unsafe { NonZeroU8::new_unchecked(1) });
    pub const N2: Self = Self(unsafe { NonZeroU8::new_unchecked(2) });
    pub const N3: Self = Self(unsafe { NonZeroU8::new_unchecked(3) });
    pub const N4: Self = Self(unsafe { NonZeroU8::new_unchecked(4) });
    pub const N5: Self = Self(unsafe { NonZeroU8::new_unchecked(5) });
    pub const N6: Self = Self(unsafe { NonZeroU8::new_unchecked(6) });
    pub const N7: Self = Self(unsafe { NonZeroU8::new_unchecked(7) });
    pub const N8: Self = Self(unsafe { NonZeroU8::new_unchecked(8) });
    pub const N9: Self = Self(unsafe { NonZeroU8::new_unchecked(9) });
}

#[derive(Clone, Copy)]
pub struct NumberBucket(u16);

impl From<Number> for NumberBucket {
    fn from(value: Number) -> Self {
        Self(value.as_mask())
    }
}

impl std::fmt::Debug for NumberBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NumberBucket")
            .field(&self.into_iter().collect::<Vec<_>>())
            .finish()
    }
}

impl NumberBucket {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn all() -> Self {
        Self((1 << 9) - 1)
    }

    pub fn example() -> Self {
        Self((1 << 3) - 1)
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn insert(&mut self, num: Number) {
        self.0 |= num.as_mask();
    }

    pub fn remove(&mut self, num: Number) {
        self.0 &= !num.as_mask()
    }

    pub fn toggle(&mut self, num: Number) {
        self.0 ^= num.as_mask()
    }

    pub fn contains(&self, num: Number) -> bool {
        (self.0 & num.as_mask()) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl BitAnd for NumberBucket {
    type Output = NumberBucket;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for NumberBucket {
    type Output = NumberBucket;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitXor for NumberBucket {
    type Output = NumberBucket;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for NumberBucket {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl Not for NumberBucket {
    type Output = NumberBucket;

    fn not(self) -> Self::Output {
        NumberBucket::all() ^ self
    }
}

#[derive(Clone)]
pub struct NineGrid<T>([T; 9 * 9]);

impl<T: Copy> NineGrid<T> {
    pub fn filled(val: T) -> Self {
        Self([val; _])
    }
}

impl<T> IndexMut<GridPosition> for NineGrid<T> {
    fn index_mut(&mut self, index: GridPosition) -> &mut Self::Output {
        &mut self.0[index.0 as usize]
    }
}

impl<T> Index<GridPosition> for NineGrid<T> {
    type Output = T;

    fn index(&self, index: GridPosition) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ParseSudokuError {
    TooLittleCells,
    TooManyCells,
    InvalidChar(char),
}

impl Display for ParseSudokuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseSudokuError::TooLittleCells => write!(f, "too little cells"),
            ParseSudokuError::TooManyCells => write!(f, "too many cells"),
            ParseSudokuError::InvalidChar(c) => write!(f, "invalid char {c}"),
        }
    }
}
impl Error for ParseSudokuError {}

impl FromStr for NineGrid<Option<Number>> {
    type Err = ParseSudokuError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut numbers = [None; _];
        let mut chars = s.chars();
        for pos in numbers.iter_mut() {
            let c = match chars.next() {
                Some(c) => c,
                None => return Err(ParseSudokuError::TooLittleCells),
            };
            match c {
                '1'..='9' => {
                    let num = Number::new((c as u32 - '0' as u32) as u8);
                    *pos = Some(num);
                }
                '.' => (),
                _ => return Err(ParseSudokuError::InvalidChar(c)),
            }
        }
        if chars.next().is_some() {
            return Err(ParseSudokuError::TooManyCells);
        }
        Ok(Self(numbers))
    }
}

impl FromStr for NineGrid<Number> {
    type Err = ParseSudokuError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut numbers = [Number::new(1); _];
        let mut chars = s.chars();
        for num in numbers.iter_mut() {
            let c = match chars.next() {
                Some(c) => c,
                None => return Err(ParseSudokuError::TooLittleCells),
            };
            match c {
                '1'..='9' => {
                    *num = Number::new((c as u32 - '0' as u32) as u8);
                }
                _ => return Err(ParseSudokuError::InvalidChar(c)),
            }
        }
        if chars.next().is_some() {
            return Err(ParseSudokuError::TooManyCells);
        }
        Ok(Self(numbers))
    }
}

#[derive(Clone)]
pub struct Sudoku {
    // TODO: this could be a Positionbucket Mask on self.solution
    given_numbers: NineGrid<Option<Number>>,
    solution: NineGrid<Number>,

    solved_numbers: NineGrid<Option<Number>>,

    // TODO: could be inverted to [PositionBucket; 9] 144  insteadof size 162
    center_notes: NineGrid<NumberBucket>,
    corner_notes: NineGrid<NumberBucket>,
}

impl Sudoku {
    pub fn new(given_numbers: NineGrid<Option<Number>>, solution: NineGrid<Number>) -> Self {
        Self {
            given_numbers,
            solution,
            solved_numbers: NineGrid::filled(None),
            center_notes: NineGrid::filled(NumberBucket::new()),
            corner_notes: NineGrid::filled(NumberBucket::new()),
        }
    }

    pub fn get<'a>(&'a self, pos: GridPosition) -> SCellRef<'a> {
        SCellRef {
            given_number: self.given_numbers[pos].as_ref(),
            solution: &self.solution[pos],
            solved_number: self.solved_numbers[pos].as_ref(),
            center_notes: &self.center_notes[pos],
            corner_notes: &self.corner_notes[pos],
        }
    }

    pub fn get_mut<'a>(&'a mut self, pos: GridPosition) -> SCellMut<'a> {
        SCellMut {
            given_number: &mut self.given_numbers[pos],
            solution: &mut self.solution[pos],
            solved_number: &mut self.solved_numbers[pos],
            center_notes: &mut self.center_notes[pos],
            corner_notes: &mut self.corner_notes[pos],
        }
    }

    pub fn is_solved(&self) -> bool {
        for cell in self.iter_cells(PositionBucket::all()) {
            if cell.given_number.is_some() {
                continue;
            }
            if let Some(num) = cell.solved_number {
                if num != cell.solution {
                    return false;
                }
            } else {
                return false;
            }
        }
        return true;
    }
}

pub struct SCellMut<'a> {
    pub given_number: &'a mut Option<Number>,
    pub solution: &'a mut Number,

    pub solved_number: &'a mut Option<Number>,
    pub center_notes: &'a mut NumberBucket,
    pub corner_notes: &'a mut NumberBucket,
}

pub struct SCellRef<'a> {
    pub given_number: Option<&'a Number>,
    pub solution: &'a Number,

    pub solved_number: Option<&'a Number>,
    pub center_notes: &'a NumberBucket,
    pub corner_notes: &'a NumberBucket,
}

impl SCellRef<'_> {
    pub fn contains_num(self, num: Number) -> bool {
        if let Some(x) = self.given_number {
            return *x == num;
        } else if let Some(x) = self.solved_number {
            return *x == num;
        }
        self.center_notes.contains(num) || self.corner_notes.contains(num)
    }
}

impl Sudoku {
    pub fn form_diff(&self, other: &Self) -> Option<Diff> {
        let mut solved_diff = Vec::new();
        for pos in PositionBucket::all().into_iter() {
            // SAFETY: Due to niche value optimization on `Option<Number>` (`Number(NonZeroU8)`),
            // `Option<Number>` has the exact same size (1 byte) and alignment (1 byte) as `u8`,
            // where `None` is bitwise 0 and `Some(Number(n))` is `n` (1..=9).
            // Transmuting to `u8` reads initialized bytes without violating any invariants.
            let xor = unsafe {
                std::mem::transmute::<_, u8>(self.solved_numbers[pos])
                    ^ std::mem::transmute::<_, u8>(other.solved_numbers[pos])
            };
            if xor != 0 {
                solved_diff.push((pos, xor))
            }
        }
        let mut corner_diff = Vec::new();

        for pos in PositionBucket::all().into_iter() {
            let xor = self.corner_notes[pos] ^ other.corner_notes[pos];
            if !xor.is_empty() {
                corner_diff.push((pos, xor))
            }
        }

        let mut center_diff = Vec::new();
        for pos in PositionBucket::all().into_iter() {
            let xor = self.center_notes[pos] ^ other.center_notes[pos];
            if !xor.is_empty() {
                center_diff.push((pos, xor))
            }
        }

        if solved_diff.len() == 0 && corner_diff.len() == 0 && center_diff.len() == 0 {
            return None;
        }

        Some(Diff {
            solved_diff,
            corner_diff,
            center_diff,
        })
    }

    pub fn apply_diff(&mut self, diff: DiffRef) {
        for (pos, xor) in diff.solved_diff() {
            unsafe {
                // SAFETY:
                // 1. `Option<Number>` is layout-compatible with `u8` (size 1, align 1), where 0 represents `None`
                //    and 1..=9 represent `Some(Number)`.
                // 2. The pointer cast `&mut self.solved_numbers[*pos] as *mut _ as *mut u8` is properly aligned and
                //    dereferences valid, initialized memory.
                // 3. In `form_diff()`, `xor = a ^ b` where `a` and `b` are valid representations of `Option<Number>`.
                //    Since `self.solved_numbers[*pos]` currently holds either `a` or `b`, XORing with `xor` produces
                //    the other valid value (`a ^ (a ^ b) = b` or `b ^ (a ^ b) = a`), strictly preserving the
                //    discriminant and `NonZeroU8` invariant of `Option<Number>`.
                let field: &mut u8 = &mut *(&mut self.solved_numbers[*pos] as *mut _ as *mut u8);
                *field ^= xor;
            }
        }
        for (pos, xor) in diff.corner_diff() {
            self.corner_notes[*pos] ^= *xor;
        }
        for (pos, xor) in diff.center_diff() {
            self.center_notes[*pos] ^= *xor;
        }
    }
}

// every diff must be non empty
pub struct Diff {
    solved_diff: Vec<(GridPosition, u8)>,
    corner_diff: Vec<(GridPosition, NumberBucket)>,
    center_diff: Vec<(GridPosition, NumberBucket)>,
}

fn transmute_to_u8<T>(s: &[T]) -> &[u8] {
    let byte_len = std::mem::size_of::<T>() * s.len();
    // SAFETY:
    // 1. `s.as_ptr()` points to `s.len()` contiguous, properly initialized instances of `T`.
    // 2. Reading them as bytes covers exactly `byte_len` initialized bytes.
    // 3. Alignment of `u8` is 1, which is trivially satisfied by any pointer.
    // 4. The returned lifetime is tied to `s`, upholding borrow checker guarantees.
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, byte_len) }
}

/// # Safety
/// - `s.as_ptr()` must be properly aligned for `T` (`align_of::<T>()`).
/// - `s.len()` must be an exact multiple of `size_of::<T>()`.
/// - The bytes in `s` must represent valid bit patterns for type `T`.
unsafe fn transmute_from_u8<T>(s: &[u8]) -> &[T] {
    let count = s.len() / std::mem::size_of::<T>();
    // SAFETY: The caller guarantees proper alignment, size multiple, and bit validity for `T`.
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const T, count) }
}

pub struct DiffRef<'a> {
    data: &'a [u8],
    solved_len: u8,
    corner_len: u8,
    center_len: u8,
}

impl<'a> DiffRef<'a> {
    pub fn solved_diff(&self) -> impl Iterator<Item = &'a (GridPosition, u8)> {
        let len = self.solved_len as usize * size_of::<(GridPosition, u8)>();
        // SAFETY:
        // 1. Alignment: `(GridPosition, u8)` has align 1 (both fields are 1 byte), so any byte offset is aligned.
        // 2. Length: `len` is `solved_len * 2`, an exact multiple of `size_of::<(GridPosition, u8)>()`.
        // 3. Validity: The first `len` bytes were serialized in `DiffStack::push` from valid `(GridPosition, u8)` pairs.
        unsafe { transmute_from_u8(&self.data[..len]) }.iter()
    }
    pub fn corner_diff(&self) -> impl Iterator<Item = &'a (GridPosition, NumberBucket)> {
        let start = self.solved_len as usize * size_of::<(GridPosition, u8)>();
        let len = self.corner_len as usize * size_of::<(GridPosition, NumberBucket)>();
        // SAFETY:
        // 1. Alignment: In `push()`, preamble is 4 bytes and `solved_diff` occupies an even number of bytes (`solved_len * 2`).
        //    Thus `start` is an even offset within `self.data`, satisfying the 2-byte alignment for `(GridPosition, NumberBucket)`.
        // 2. Length: `len` is `corner_len * 4`, an exact multiple of `size_of::<(GridPosition, NumberBucket)>()`.
        // 3. Validity: These bytes were serialized directly from valid `(GridPosition, NumberBucket)` slices in `push()`.
        unsafe { transmute_from_u8(&self.data[start..][..len]) }.iter()
    }

    pub fn center_diff(&self) -> impl Iterator<Item = &'a (GridPosition, NumberBucket)> {
        let start = self.solved_len as usize * size_of::<(GridPosition, u8)>()
            + self.corner_len as usize * size_of::<(GridPosition, NumberBucket)>();
        let len = self.center_len as usize * size_of::<(GridPosition, NumberBucket)>();
        // SAFETY:
        // 1. Alignment: Preamble (4 bytes), `solved_diff` (even bytes), and `corner_diff` (multiples of 4 bytes)
        //    ensure `start` is an even byte offset, satisfying the 2-byte alignment for `(GridPosition, NumberBucket)`.
        // 2. Length: `len` is `center_len * 4`, an exact multiple of `size_of::<(GridPosition, NumberBucket)>()`.
        // 3. Validity: These bytes were serialized directly from valid `(GridPosition, NumberBucket)` slices in `push()`.
        unsafe { transmute_from_u8(&self.data[start..][..len]) }.iter()
    }
}

pub struct DiffStack {
    stack: Vec<u8>,
    pointer: usize,
}

impl DiffStack {
    const PREAMBLE_SIZE: usize = 4;
    pub fn new() -> Self {
        Self {
            stack: vec![0; Self::PREAMBLE_SIZE],
            pointer: 0,
        }
    }

    pub fn push(&mut self, diff: Diff) {
        // SAFETY: `self.pointer` is guaranteed to be <= `self.stack.len()` and <= `self.stack.capacity()`.
        // The elements up to `self.pointer` are already initialized bytes.
        // Truncating the length to `self.pointer` drops any invalidated redo history without reallocating.
        unsafe {
            self.stack.set_len(self.pointer);
        }
        let total_len = (&diff).solved_diff.len() * 2
            + (&diff).corner_diff.len() * 4
            + (&diff).center_diff.len() * 4
            + 2 * Self::PREAMBLE_SIZE;

        self.stack.reserve(total_len + Self::PREAMBLE_SIZE);
        // each vec can at most have 9*9 = 81 elements
        // we can use a u8 each to describe the len
        self.stack.push((&diff).solved_diff.len() as u8);
        self.stack.push((&diff).corner_diff.len() as u8);
        self.stack.push((&diff).center_diff.len() as u8);
        self.stack.push(0xFF); // pad

        self.stack
            .extend_from_slice(transmute_to_u8(&(&diff).solved_diff));
        self.stack
            .extend_from_slice(transmute_to_u8(&(&diff).corner_diff));
        self.stack
            .extend_from_slice(transmute_to_u8(&(&diff).center_diff));

        self.stack.push((&diff).solved_diff.len() as u8);
        self.stack.push((&diff).corner_diff.len() as u8);
        self.stack.push((&diff).center_diff.len() as u8);
        self.stack.push(0xFF); // pad

        self.pointer += total_len;
        debug_assert!(self.pointer == self.stack.len());
        self.stack.extend_from_slice(&[0; 4]);
    }

    pub fn pop<'a>(&'a mut self) -> Option<DiffRef<'a>> {
        if self.pointer == 0 {
            return None;
        }
        let lens = &self.stack[self.pointer - Self::PREAMBLE_SIZE..self.pointer];
        let solved_len = lens[0];
        let corner_len = lens[1];
        let center_len = lens[2];
        let data_len = solved_len as usize * 2 + corner_len as usize * 4 + center_len as usize * 4;

        let diff = DiffRef {
            data: &self.stack
                [self.pointer - data_len - Self::PREAMBLE_SIZE..self.pointer - Self::PREAMBLE_SIZE],
            solved_len,
            corner_len,
            center_len,
        };
        self.pointer -= data_len + 2 * Self::PREAMBLE_SIZE;
        Some(diff)
    }

    pub fn unpop<'a>(&'a mut self) -> Option<DiffRef<'a>> {
        let lens = &self.stack[self.pointer..][..Self::PREAMBLE_SIZE];
        if lens.iter().all(|x| *x == 0) {
            return None;
        }
        let solved_len = lens[0];
        let corner_len = lens[1];
        let center_len = lens[2];
        let data_len = solved_len as usize * 2 + corner_len as usize * 4 + center_len as usize * 4;

        let diff = DiffRef {
            data: &self.stack[self.pointer + Self::PREAMBLE_SIZE..][..data_len],
            solved_len,
            corner_len,
            center_len,
        };
        self.pointer += data_len + 2 * Self::PREAMBLE_SIZE;
        Some(diff)
    }

    pub fn can_undo(&self) -> bool {
        return self.pointer != 0;
    }

    pub fn can_redo(&self) -> bool {
        let lens = &self.stack[self.pointer..][..Self::PREAMBLE_SIZE];
        !lens.iter().all(|x| *x == 0)
    }

    #[allow(unused)]
    pub fn log_state(&self) {
        info!("ptr: {:?}", self.pointer);
        info!("vec_len: {:?}", self.stack.len());
        if self.pointer > 0 {
            info!(
                "below lens: {:?}",
                &self.stack[self.pointer - Self::PREAMBLE_SIZE..self.pointer]
            );
        } else {
            info!("below lens: --");
        }
        info!(
            "lens above: {:?}",
            &self.stack[self.pointer..][..Self::PREAMBLE_SIZE]
        );
    }
}

#[derive(Clone, Copy)]
pub struct GridLayout {
    pub left: f32,
    pub top: f32,
    pub size: f32,
}

impl GridLayout {
    pub fn bold_stroke(&self) -> Stroke {
        Stroke {
            width: self.size / 9.0 / 20.0,
            miter_limit: 4.0,
            line_cap: LineCap::Square,
            line_join: LineJoin::Bevel,
            dash: None,
        }
    }
    pub fn light_stroke(&self) -> Stroke {
        Stroke {
            width: self.size / 9.0 / 20.0 / 3.0,
            miter_limit: 4.0,
            line_cap: LineCap::Square,
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

impl Sudoku {
    pub fn draw_grid(layout: &GridLayout, canvas: &mut Canvas<'_>) {
        // outline
        let mut pb = PathBuilder::new();
        pb.move_to(layout.left, layout.top);
        pb.line_to(layout.left + layout.size, layout.top);
        pb.line_to(layout.left + layout.size, layout.top + layout.size);
        pb.line_to(layout.left, layout.top + layout.size);
        // pb.line_to(layout.left, layout.top); // TODO necessary?
        pb.close();

        canvas.stroke_path(
            &pb.finish().unwrap(),
            Swatch::GridLine,
            &layout.bold_stroke(),
        );

        // fat lines
        let mut pb = PathBuilder::new();
        for i in 1..=2 {
            pb.move_to(layout.left, layout.top + layout.size * (i as f32 / 3.0));
            pb.line_to(
                layout.left + layout.size,
                layout.top + layout.size * (i as f32 / 3.0),
            );
        }
        for i in 1..=2 {
            pb.move_to(layout.left + layout.size * (i as f32 / 3.0), layout.top);
            pb.line_to(
                layout.left + layout.size * (i as f32 / 3.0),
                layout.top + layout.size,
            );
        }

        canvas.stroke_path(
            &pb.finish().unwrap(),
            Swatch::GridLine,
            &layout.bold_stroke(),
        );

        // horizontal
        let mut pb = PathBuilder::new();
        for i in 0..3 {
            for j in 1..3 {
                pb.move_to(
                    layout.left,
                    layout.top + layout.size * (i as f32 / 3.0 + j as f32 / 9.0),
                );
                pb.line_to(
                    layout.left + layout.size,
                    layout.top + layout.size * (i as f32 / 3.0 + j as f32 / 9.0),
                );
            }
        }

        // vertical
        for i in 0..3 {
            for j in 1..3 {
                pb.move_to(
                    layout.left + layout.size * (i as f32 / 3.0 + j as f32 / 9.0),
                    layout.top,
                );
                pb.line_to(
                    layout.left + layout.size * (i as f32 / 3.0 + j as f32 / 9.0),
                    layout.top + layout.size,
                );
            }
        }
        canvas.stroke_path(
            &pb.finish().unwrap(),
            Swatch::GridLine,
            &layout.light_stroke(),
        );
    }

    pub fn draw_cells(&self, canvas: &mut Canvas<'_>, layout: &GridLayout) {
        for pos in PositionBucket::all().into_iter() {
            let rect = layout.get_rect(pos);
            let cell = self.get(pos);

            if let Some(num) = cell.given_number {
                canvas.draw_num(*num, rect, Swatch::GridNumbers);
                continue;
            }
            if let Some(num) = cell.solved_number {
                if num != self.get(pos).solution {
                    canvas.fill_rect(rect, Swatch::WrongNumber);
                }
                canvas.draw_num(*num, rect, Swatch::GridNumbers);
                continue;
            }

            if !cell.center_notes.is_empty() {
                canvas.draw_center_notes(rect, *cell.center_notes, Swatch::GridNumbers);
            }

            if !cell.corner_notes.is_empty() {
                canvas.draw_corner_notes(rect, *cell.corner_notes, Swatch::GridNumbers);
            }
        }
    }

    fn highlight_cells(
        canvas: &mut Canvas<'_>,
        color: Swatch,
        layout: &GridLayout,
        bucket: &PositionBucket,
    ) {
        for pos in bucket.into_iter() {
            canvas.fill_rect(layout.get_rect(pos), color);
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        marks: &[(Swatch, PositionBucket)],
        layout: &GridLayout,
    ) {
        canvas.fill_rect(
            Rect::from_xywh(layout.left, layout.top, layout.size, layout.size),
            Swatch::GridBackground,
        );
        for (col, bucket) in marks {
            Self::highlight_cells(canvas, *col, layout, bucket);
        }
        self.draw_cells(canvas, layout);
        Self::draw_grid(layout, canvas);
    }
}
