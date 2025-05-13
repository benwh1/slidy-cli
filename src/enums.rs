use clap::ValueEnum;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LabelType {
    RowGrids,
    Rows,
    Fringe,
    SquareFringe,
    SplitFringe,
    SplitSquareFringe,
    Diagonals,
    Checkerboard,
    Grids,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ColoringType {
    None,
    Rainbow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum StateFormatter {
    Inline,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Metric {
    Stm,
    Mtm,
}
