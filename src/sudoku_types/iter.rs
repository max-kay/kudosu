use crate::{GridPosition, Number, NumberBucket, PositionBucket};

pub struct PosIter(u128);

impl PositionBucket {
    pub fn into_iter(&self) -> PosIter {
        PosIter(self.0)
    }
}

impl Iterator for PosIter {
    type Item = GridPosition;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let bit_idx = self.0.trailing_zeros();
        self.0 &= !(1 << bit_idx);
        Some(GridPosition {
            row: (bit_idx / 9) as u8,
            col: (bit_idx % 9) as u8,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.count_ones() as usize;
        (len, Some(len))
    }
}

impl ExactSizeIterator for PosIter {}

pub struct NumIter(u16);

impl NumberBucket {
    pub fn into_iter(&self) -> NumIter {
        NumIter(self.0)
    }
}

impl Iterator for NumIter {
    type Item = Number;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let num = self.0.trailing_zeros() + 1;
        self.0 &= !(1 << (num - 1));
        Some(Number::new(num as u8)) // TODO do directly
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.count_ones() as usize;
        (len, Some(len))
    }
}

impl ExactSizeIterator for NumIter {}
