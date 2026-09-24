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
        let index = self.0.trailing_zeros() as u8;
        self.0 &= !(1 << index);
        Some(GridPosition(index))
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
        Some(Number::new(num as u8))
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
            // SAFETY: `self.sudoku` points to a valid, live `Sudoku` for lifetime `'a`.
            // Furthermore, `pos_iter` yields each `GridPosition` at most once, guaranteeing
            // that successive calls to `next()` produce disjoint mutable borrows to distinct
            // cells. Extending the returned `SCellMut` lifetime to `'a` is therefore sound
            // and never introduces aliased mutable references, analogous to `slice::IterMut`.
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
