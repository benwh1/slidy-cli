use slidy::algorithm::algorithm::Algorithm;

use crate::enums::Metric;

pub trait AlgorithmExt {
    fn len_metric(&self, metric: Metric) -> u64;
}

impl AlgorithmExt for Algorithm {
    fn len_metric(&self, metric: Metric) -> u64 {
        match metric {
            Metric::Stm => self.len_stm(),
            Metric::Mtm => self.len_mtm(),
        }
    }
}
