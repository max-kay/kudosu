use core::panic;
use std::{
    ops::{BitAnd, BitOr, BitXor, Index, IndexMut, Not},
    str::FromStr,
};

use ttf_parser::Face;

mod iter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition {
    row: u8,
    col: u8,
}

impl GridPosition {
    pub fn new(row: u8, col: u8) -> Self {
        assert!(row < 9, "Gridposition with invalid row: `{}`", row);
        assert!(col < 9, "Gridposition with invalid col: `{}`", col);
        Self { row, col }
    }

    pub fn row(&self) -> u8 {
        self.row
    }

    pub fn col(&self) -> u8 {
        self.col
    }

    pub fn box_(&self) -> u8 {
        (self.row / 3) * 3 + self.col / 3
    }

    fn as_mask(&self) -> u128 {
        1 << (self.row * 9 + self.col)
    }
}

impl From<GridPosition> for PositionBucket {
    fn from(value: GridPosition) -> Self {
        PositionBucket(value.as_mask())
    }
}

impl GridPosition {
    pub fn sees_by_sudoku(&self) -> PositionBucket {
        PositionBucket::col(self.col())
            | PositionBucket::row(self.row())
            | PositionBucket::box_(self.box_())
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
            panic!("col with number {} dne", num)
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
            panic!("box with number {} dne", num);
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

impl Not for PositionBucket {
    type Output = PositionBucket;

    fn not(self) -> Self::Output {
        PositionBucket::all() ^ self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Number(pub(super) u8);

impl Number {
    pub fn new(val: u8) -> Self {
        if val != 0 && val <= 9 {
            Self(val)
        } else {
            panic!("tried to create a number out of range `{}`", val)
        }
    }

    fn as_mask(&self) -> u16 {
        1 << (self.0 - 1)
    }

    pub fn as_char(&self) -> char {
        match self.0 {
            1..=9 => (b'0' + self.0) as char,
            _ => unreachable!(),
        }
    }

    fn as_u8(&self) -> u8 {
        self.0
    }

    fn as_path(&self, face: Face) -> tiny_skia::Path {
        todo!()
    }
}

#[derive(Clone, Copy)]
pub struct NumberBucket(u16);

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

    pub fn empty(&self) -> bool {
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

impl Not for NumberBucket {
    type Output = NumberBucket;

    fn not(self) -> Self::Output {
        NumberBucket::all() ^ self
    }
}

pub struct NineGrid<T>([[T; 9]; 9]);

impl<T: Copy> NineGrid<T> {
    pub fn filled(val: T) -> Self {
        Self([[val; _]; _])
    }
}

impl<T> IndexMut<GridPosition> for NineGrid<T> {
    fn index_mut(&mut self, index: GridPosition) -> &mut Self::Output {
        &mut self.0[index.row as usize][index.col as usize]
    }
}

impl<T> Index<GridPosition> for NineGrid<T> {
    type Output = T;

    fn index(&self, index: GridPosition) -> &Self::Output {
        &self.0[index.row as usize][index.col as usize]
    }
}

impl FromStr for NineGrid<Option<Number>> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut numbers = [[None; 9]; 9];
        let mut chars = s.chars();
        for j in 0..9 {
            for i in 0..9 {
                let c = match chars.next() {
                    Some(c) => c,
                    None => return Err(()),
                };
                match c {
                    '1'..='9' => {
                        let num = Number::new((c as u32 - '0' as u32) as u8);
                        numbers[j][i] = Some(num);
                    }
                    '.' => (),
                    _ => return Err(()),
                }
            }
        }
        if chars.next().is_some() {
            return Err(());
        }
        Ok(Self(numbers))
    }
}

impl FromStr for NineGrid<Number> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut numbers = [[Number::new(1); 9]; 9];
        let mut chars = s.chars();
        for j in 0..9 {
            for i in 0..9 {
                let c = match chars.next() {
                    Some(c) => c,
                    None => return Err(()),
                };
                match c {
                    '1'..='9' => {
                        let num = Number::new((c as u32 - '0' as u32) as u8);
                        numbers[j][i] = num;
                    }
                    _ => return Err(()),
                }
            }
        }
        if chars.next().is_some() {
            return Err(());
        }
        Ok(Self(numbers))
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

pub struct Sudoku {
    given_numbers: NineGrid<Option<Number>>,
    solution: NineGrid<Number>,

    solved_numbers: NineGrid<Option<Number>>,
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
}

impl Default for Sudoku {
    fn default() -> Self {
        // source https://qqwing.com/generate.html
        let given =
            ".9.8.5...6.1......78...1....5.7.....4.8...9...6..4.....275.6.....61...75...9..82.";
        let sol =
            "294865137631427589785391264352719648478653912169248753827536491946182375513974826";
        Self::new(
            FromStr::from_str(given).unwrap(),
            FromStr::from_str(sol).unwrap(),
        )
    }
}
pub mod test {
    use super::*;
    pub fn number_bucket() {
        let all = NumberBucket::all();
        assert_eq!(all.0.count_ones(), 9);
        assert_eq!(all.0.trailing_ones(), 9);

        let mut bucket = NumberBucket::new();
        bucket.insert(Number::new(1));
        bucket.insert(Number::new(5));
        bucket.insert(Number::new(9));

        assert!(bucket.contains(Number::new(1)));
        assert!(bucket.contains(Number::new(5)));
        assert!(bucket.contains(Number::new(9)));

        assert!(!bucket.contains(Number::new(2)));
        assert!(!bucket.contains(Number::new(3)));
        assert!(!bucket.contains(Number::new(7)));

        let mut iter = bucket.into_iter();
        assert_eq!(iter.next(), Some(Number::new(1)));
        assert_eq!(iter.next(), Some(Number::new(5)));
        assert_eq!(iter.next(), Some(Number::new(9)));
        assert_eq!(iter.next(), None);
    }

    pub fn pos_bucket() {
        let all = PositionBucket::all();
        assert_eq!(all.0.count_ones(), 81);
        assert_eq!(all.0.trailing_ones(), 81);

        let mut bucket = PositionBucket::new();
        bucket.insert(GridPosition::new(1, 1));
        bucket.insert(GridPosition::new(5, 3));
        bucket.insert(GridPosition::new(0, 6));
        bucket.insert(GridPosition::new(1, 4));

        assert!(bucket.contains(GridPosition::new(1, 1)));
        assert!(bucket.contains(GridPosition::new(5, 3)));
        assert!(bucket.contains(GridPosition::new(0, 6)));
        assert!(bucket.contains(GridPosition::new(1, 4)));

        assert!(!bucket.contains(GridPosition::new(3, 3)));
        assert!(!bucket.contains(GridPosition::new(8, 0)));
        assert!(!bucket.contains(GridPosition::new(2, 2)));

        let mut iter = bucket.into_iter();
        assert_eq!(iter.next(), Some(GridPosition::new(0, 6)));
        assert_eq!(iter.next(), Some(GridPosition::new(1, 1)));
        assert_eq!(iter.next(), Some(GridPosition::new(1, 4)));
        assert_eq!(iter.next(), Some(GridPosition::new(5, 3)));
        assert_eq!(iter.next(), None);
    }
}
