use std::ops::Range;

use slidy::algorithm::{
    algorithm::{Algorithm, SliceError},
    slice::AlgorithmSlice,
};

use crate::enums::Metric;

pub trait AlgorithmExt {
    fn len_metric(&self, metric: Metric) -> u64;
    fn slice_metric(
        &self,
        metric: Metric,
        range: Range<u64>,
    ) -> Result<AlgorithmSlice<'_>, SliceError>;
}

impl AlgorithmExt for Algorithm {
    fn len_metric(&self, metric: Metric) -> u64 {
        match metric {
            Metric::Stm => self.len_stm(),
            Metric::Mtm => self.len_mtm(),
        }
    }

    fn slice_metric(
        &self,
        metric: Metric,
        range: Range<u64>,
    ) -> Result<AlgorithmSlice<'_>, SliceError> {
        match metric {
            Metric::Stm => self.try_slice(range),
            Metric::Mtm => self.try_slice_mtm(range),
        }
    }
}
