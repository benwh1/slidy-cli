use clap::ValueEnum;
use palette::rgb::Rgba;
use slidy::puzzle::{
    coloring::{Coloring, Monochrome, Rainbow},
    label::label::{
        Checkerboard, Diagonals, Fringe, Label, RowGrids, Rows, SplitFringe, SplitSquareFringe,
        SquareFringe, Trivial,
    },
    size::Size,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum LabelType {
    /// Every position has the same label.
    Trivial,
    /// Every position has its own label.
    #[default]
    RowGrids,
    /// One label per row.
    Rows,
    /// One label per fringe (a row and column together), working in from the top left.
    Fringe,
    /// Rows or columns, until the rest of the puzzle is square, then fringes.
    SquareFringe,
    /// Like fringe, but the row and column of each fringe get different labels.
    SplitFringe,
    /// Like square-fringe, but with split fringes in the square part.
    SplitSquareFringe,
    /// One label per bottom-left to top-right diagonal, starting from the top left.
    Diagonals,
    /// Two labels, alternating like a checkerboard.
    Checkerboard,
}

impl LabelType {
    pub fn pdb_file_name(self) -> &'static str {
        match self {
            Self::Trivial => "trivial",
            Self::RowGrids => "row-grids",
            Self::Rows => "rows",
            Self::Fringe => "fringe",
            Self::SquareFringe => "square-fringe",
            Self::SplitFringe => "split-fringe",
            Self::SplitSquareFringe => "split-square-fringe",
            Self::Diagonals => "diagonals",
            Self::Checkerboard => "checkerboard",
        }
    }
}

impl Label for LabelType {
    fn position_label(&self, size: Size, pos: (u64, u64)) -> u64 {
        match self {
            Self::Trivial => Trivial.position_label(size, pos),
            Self::RowGrids => RowGrids.position_label(size, pos),
            Self::Rows => Rows.position_label(size, pos),
            Self::Fringe => Fringe.position_label(size, pos),
            Self::SquareFringe => SquareFringe.position_label(size, pos),
            Self::SplitFringe => SplitFringe.position_label(size, pos),
            Self::SplitSquareFringe => SplitSquareFringe.position_label(size, pos),
            Self::Diagonals => Diagonals.position_label(size, pos),
            Self::Checkerboard => Checkerboard.position_label(size, pos),
        }
    }

    fn num_labels(&self, size: Size) -> u64 {
        match self {
            Self::Trivial => Trivial.num_labels(size),
            Self::RowGrids => RowGrids.num_labels(size),
            Self::Rows => Rows.num_labels(size),
            Self::Fringe => Fringe.num_labels(size),
            Self::SquareFringe => SquareFringe.num_labels(size),
            Self::SplitFringe => SplitFringe.num_labels(size),
            Self::SplitSquareFringe => SplitSquareFringe.num_labels(size),
            Self::Diagonals => Diagonals.num_labels(size),
            Self::Checkerboard => Checkerboard.num_labels(size),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ColoringType {
    /// Transparent.
    None,
    /// A different colour for each label.
    #[default]
    Rainbow,
    /// Black.
    Black,
}

impl ColoringType {
    pub fn to_box_dyn_coloring(self) -> Box<dyn Coloring> {
        match self {
            Self::None => Box::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 0.0))),
            Self::Rainbow => Box::new(Rainbow::default()),
            Self::Black => Box::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 1.0))),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum StateFormatter {
    /// On one line, with rows separated by `/`.
    #[default]
    Inline,
    /// One row per line, with the columns aligned.
    Grid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum Metric {
    /// Single-tile metric: moving one tile one space is one move.
    #[default]
    Stm,
    /// Multi-tile metric: moving any number of tiles in a line at once is one move.
    Mtm,
}

impl Metric {
    pub fn pdb_file_name(self) -> &'static str {
        match self {
            Self::Stm => "stm",
            Self::Mtm => "mtm",
        }
    }
}

/// The pattern database the 4x4 STM solver searches with. Bigger ones solve faster but take more
/// memory and disk space, and longer to build the first time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum Pdb4x4Stm {
    /// 5.9 MB, builds in about a second.
    Small,
    /// 75 MB, builds in about 10 seconds. About 3 times as fast as small.
    #[default]
    Medium,
    /// 1.27 GB, builds in about 2 minutes. About 7 times as fast as medium.
    Large,
    /// 17 GB, builds in about 40 minutes using up to 25 GB of memory (64-bit only). About twice
    /// as fast as large.
    Huge,
}

impl Pdb4x4Stm {
    pub fn pdb_file_name(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Huge => "huge",
        }
    }
}
