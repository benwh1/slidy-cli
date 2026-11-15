use slidy::puzzle::{label::label::Label, size::Size};

#[derive(Default)]
pub struct SplitFringePruneTarget4x4;

impl Label for SplitFringePruneTarget4x4 {
    fn position_label(&self, _: Size, (x, y): (u64, u64)) -> u64 {
        let index = (x + 4 * y) as usize;
        [0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 3, 3, 0, 2, 4, 5][index]
    }
}

#[derive(Default)]
pub struct DiagonalsPruneTarget4x4;

impl Label for DiagonalsPruneTarget4x4 {
    fn position_label(&self, _: Size, (x, y): (u64, u64)) -> u64 {
        let index = (x + 4 * y) as usize;
        [0, 1, 2, 2, 1, 2, 2, 3, 2, 2, 3, 4, 2, 3, 4, 5][index]
    }
}
