use slidy::puzzle::{label::label::Label, size::Size};

#[derive(Default)]
pub struct SplitFringePruneTarget4x4;

impl Label for SplitFringePruneTarget4x4 {
    fn position_label(&self, _: Size, (x, y): (u64, u64)) -> u64 {
        let index = (x + 4 * y) as usize;
        [0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 3, 3, 0, 2, 4, 5][index]
    }
}
