use crate::{
    GridPosition, Number, NumberBucket, PositionBucket,
    sudoku_types::{SCellMut, SCellRef, Sudoku},
};

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
        Some(Number(num as u8)) // TODO do directly
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.count_ones() as usize;
        (len, Some(len))
    }
}

impl ExactSizeIterator for NumIter {}

pub struct CellIter<'a> {
    sudoku: &'a Sudoku,
    pos_iter: PosIter,
}

impl<'a> Iterator for CellIter<'a> {
    type Item = SCellRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos_iter.next().map(|p| self.sudoku.get(p))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.pos_iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for CellIter<'a> {}

pub struct CellIterMut<'a> {
    sudoku: &'a mut Sudoku,
    pos_iter: PosIter,
}

impl<'a> Iterator for CellIterMut<'a> {
    type Item = SCellMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos_iter.next().map(|pos| {
            // SAFETY: `pos_iter` yields each position at most once, so the
            // `SCellMut`s produced by successive calls to `next` never
            // reference the same cell. That means it's sound to hand out a
            // mutable borrow with lifetime `'a` instead of the shorter
            // lifetime of `&mut self`, exactly as `slice::IterMut` does.
            let sudoku: &'a mut Sudoku = unsafe { &mut *(self.sudoku as *mut Sudoku) };
            sudoku.get_mut(pos)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.pos_iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for CellIterMut<'a> {}

impl Sudoku {
    pub fn iter_cells(&self, selection: PositionBucket) -> impl Iterator<Item = SCellRef<'_>> {
        CellIter {
            sudoku: self,
            pos_iter: selection.into_iter(),
        }
    }

    pub fn iter_cells_mut(
        &mut self,
        selection: PositionBucket,
    ) -> impl Iterator<Item = SCellMut<'_>> {
        CellIterMut {
            sudoku: self,
            pos_iter: selection.into_iter(),
        }
    }
}
