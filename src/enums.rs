use clap::ValueEnum;
use palette::rgb::Rgba;
use slidy::puzzle::{
    coloring::{Coloring, Monochrome, Rainbow},
    label::{
        label::{
            Checkerboard, Diagonals, Fringe, Label, RowGrids, Rows, SplitFringe, SplitSquareFringe,
            SquareFringe, Trivial,
        },
        scaled::Scaled,
    },
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
    Grids,
}

impl LabelType {
    pub fn to_box_dyn_label(self, grid_size: Option<(u64, u64)>) -> Option<Box<dyn Label>> {
        match self {
            Self::Trivial => Some(Box::new(Trivial)),
            Self::RowGrids => Some(Box::new(RowGrids)),
            Self::Rows => Some(Box::new(Rows)),
            Self::Fringe => Some(Box::new(Fringe)),
            Self::SquareFringe => Some(Box::new(SquareFringe)),
            Self::SplitFringe => Some(Box::new(SplitFringe)),
            Self::SplitSquareFringe => Some(Box::new(SplitSquareFringe)),
            Self::Diagonals => Some(Box::new(Diagonals)),
            Self::Checkerboard => Some(Box::new(Checkerboard)),
            Self::Grids => grid_size
                .and_then(|g| Scaled::new(RowGrids, g).ok())
                .map(|l| Box::new(l) as Box<dyn Label>),
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
