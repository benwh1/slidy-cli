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
    pub fn to_box_dyn_label(&self, grid_size: Option<(u64, u64)>) -> Option<Box<dyn Label>> {
        match self {
            LabelType::Trivial => Some(Box::new(Trivial)),
            LabelType::RowGrids => Some(Box::new(RowGrids)),
            LabelType::Rows => Some(Box::new(Rows)),
            LabelType::Fringe => Some(Box::new(Fringe)),
            LabelType::SquareFringe => Some(Box::new(SquareFringe)),
            LabelType::SplitFringe => Some(Box::new(SplitFringe)),
            LabelType::SplitSquareFringe => Some(Box::new(SplitSquareFringe)),
            LabelType::Diagonals => Some(Box::new(Diagonals)),
            LabelType::Checkerboard => Some(Box::new(Checkerboard)),
            LabelType::Grids => grid_size
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
    pub fn to_box_dyn_coloring(&self) -> Box<dyn Coloring> {
        match self {
            ColoringType::None => Box::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 0.0))),
            ColoringType::Rainbow => Box::new(Rainbow::default()),
            ColoringType::Black => Box::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 1.0))),
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
