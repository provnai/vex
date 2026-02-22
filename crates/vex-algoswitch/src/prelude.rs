//! Prelude - commonly used types and functions

pub use crate::algorithms::sort::SortFamily;
pub use crate::algorithms::search::SearchFamily;
pub use crate::algorithms::hash::HashFamily;
pub use crate::algorithms::traits::{Algorithm, AlgorithmFamily, AlgorithmResult, SelectResult};

pub use crate::core::{Config, ExecutionContext};
pub use crate::core::config::{SelectionStrategy, CacheConfig, ProfileConfig};
pub use crate::runtime::algo::SelectOutput;

pub use crate::runtime::algo::select;

pub use crate::algorithms::sort::{QuickSort, MergeSort, HeapSort, InsertionSort, RadixSort};
pub use crate::algorithms::search::{LinearSearch, BinarySearch, InterpolationSearch, ExponentialSearch};
pub use crate::algorithms::hash::{FnvHash, Djb2Hash, MurmurHash, XxHash};
