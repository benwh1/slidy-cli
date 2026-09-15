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

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LabelType {
    Trivial,
    RowGrids,
    Rows,
    Fringe,
    SquareFringe,
    SplitFringe,
    SplitSquareFringe,
    Diagonals,
    Checkerboard,
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

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ColoringType {
    None,
    Rainbow,
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
