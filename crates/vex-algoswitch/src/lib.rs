//! AlgoSwitch - Self-Optimizing Algorithm Runtime
//! 
//! A runtime library that automatically selects the optimal algorithm for your data patterns.
//! 
//! # Quick Start
//! 
//! ```rust,ignore
//! use algoswitch::{sort, select, Config};
//! 
//! let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
//! 
//! let result = select(
//!     vec![
//!         ("quicksort", sort::quicksort),
//!         ("mergesort", sort::mergesort),
//!         ("heapsort", sort::heapsort),
//!         ("insertionsort", sort::insertionsort),
//!     ],
//!     &mut data,
//!     Config::default(),
//! );
//! 
//! println!("Winner: {} ({}ns)", result.winner, result.time_ns);
//! ```

use std::time::Instant;
use std::fmt::Debug;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// ============================================================================
// Core Types
// ============================================================================

/// Selection configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Number of warmup runs before selecting winner
    pub warmup_runs: u32,
    /// Enable caching of winning algorithms
    pub cache_enabled: bool,
    /// Enable debug logging
    pub debug: bool,
    /// Enable smart pattern detection
    pub smart_detection: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            warmup_runs: 3,
            cache_enabled: true,
            debug: false,
            smart_detection: true,
        }
    }
}

impl Config {
    pub fn new() -> Self { Self::default() }
    pub fn with_warmup(mut self, runs: u32) -> Self { self.warmup_runs = runs; self }
    pub fn with_cache(mut self, enabled: bool) -> Self { self.cache_enabled = enabled; self }
    pub fn with_debug(mut self, enabled: bool) -> Self { self.debug = enabled; self }
    pub fn with_smart_detection(mut self, enabled: bool) -> Self { self.smart_detection = enabled; self }
}

/// Selection result
#[derive(Debug, Clone)]
pub struct SelectResult<O> {
    /// The output from the winning algorithm
    pub output: O,
    /// Name of the winning algorithm
    pub winner: String,
    /// Time taken by winner in nanoseconds
    pub time_ns: u64,
    /// All algorithm timings for comparison
    pub timings: Vec<AlgoTiming>,
    /// Detected data pattern (if smart detection enabled)
    pub pattern: Option<DataPattern>,
}

/// Timing info for a single algorithm
#[derive(Debug, Clone)]
pub struct AlgoTiming {
    pub name: String,
    pub time_ns: u64,
}

// ============================================================================
// Pattern Detection - PHASE 3 INTELLIGENCE
// ============================================================================

/// Data pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataPattern {
    /// Already sorted or nearly sorted
    Sorted,
    /// Reverse sorted
    ReverseSorted,
    /// Mostly sorted with few elements out of place
    NearlySorted,
    /// Random distribution
    Random,
    /// Many duplicate values
    FewUnique,
    /// Unknown pattern
    Unknown,
}

impl DataPattern {
    /// Get recommended algorithms for this pattern
    pub fn recommended_sort(&self) -> Vec<&'static str> {
        match self {
            DataPattern::Sorted | DataPattern::NearlySorted => {
                vec!["insertionsort", "quicksort"]
            }
            DataPattern::ReverseSorted => {
                vec!["insertionsort"]
            }
            DataPattern::FewUnique => {
                vec!["radixsort", "insertionsort"]
            }
            DataPattern::Random => {
                vec!["quicksort", "mergesort", "heapsort"]
            }
            DataPattern::Unknown => {
                vec!["quicksort", "mergesort", "heapsort", "insertionsort"]
            }
        }
    }
}

/// Detect the pattern in data (optimized)
pub fn detect_pattern(data: &[i64]) -> DataPattern {
    if data.is_empty() {
        return DataPattern::Unknown;
    }

    let n = data.len();
    
    // For large data, use sampling for everything
    let use_full_check = n <= 1000;
    let sample_size = n.min(1000);
    let step = if use_full_check { 1 } else { n / sample_size };
    
    // Check for few unique values (sampled)
    let mut unique_count = 0usize;
    let mut seen = std::collections::HashSet::new();
    for i in (0..n).step_by(step) {
        if seen.insert(data[i]) {
            unique_count += 1;
            if unique_count > sample_size / 2 {
                break; // Enough unique values
            }
        }
    }
    
    let checked = (n / step).max(1);
    let unique_ratio = unique_count as f64 / checked as f64;
    
    if unique_ratio < 0.30 {
        return DataPattern::FewUnique;
    }
    
    // Check for sorted (sampled)
    let mut sorted_count = 0usize;
    let mut reverse_count = 0usize;
    let mut sorted_checked = 0usize;
    
    for i in (0..n.saturating_sub(1)).step_by(step) {
        if data[i] <= data[i + 1] { sorted_count += 1; }
        if data[i] >= data[i + 1] { reverse_count += 1; }
        sorted_checked += 1;
    }
    
    if sorted_checked == 0 { sorted_checked = 1; }
    let sorted_ratio = sorted_count as f64 / sorted_checked as f64;
    let reverse_ratio = reverse_count as f64 / sorted_checked as f64;
    
    if sorted_ratio > 0.99 {
        return DataPattern::Sorted;
    }
    
    if reverse_ratio > 0.99 {
        return DataPattern::ReverseSorted;
    }
    
    if sorted_ratio > 0.80 {
        return DataPattern::NearlySorted;
    }
    
    DataPattern::Random
}

/// Get pattern as string
pub fn pattern_name(pattern: &DataPattern) -> &'static str {
    match pattern {
        DataPattern::Sorted => "sorted",
        DataPattern::ReverseSorted => "reverse sorted",
        DataPattern::NearlySorted => "nearly sorted",
        DataPattern::Random => "random",
        DataPattern::FewUnique => "few unique",
        DataPattern::Unknown => "unknown",
    }
}

// ============================================================================
// Global Cache - PHASE 3 INTELLIGENCE
// ============================================================================

/// Cache entry for a pattern
#[derive(Debug, Clone)]
struct CacheEntry {
    winner: String,
    count: u32,
}

/// Global algorithm cache (pattern -> winner)
static ALGORITHM_CACHE: Lazy<Mutex<HashMap<String, CacheEntry>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Get cached winner for pattern
pub fn get_cached(pattern: &DataPattern) -> Option<String> {
    let cache = ALGORITHM_CACHE.lock().ok()?;
    let key = pattern_name(pattern);
    cache.get(key).map(|e| e.winner.clone())
}

/// Cache a winner for pattern
pub fn cache_winner(pattern: &DataPattern, winner: &str) {
    if let Ok(mut cache) = ALGORITHM_CACHE.lock() {
        let key = pattern_name(pattern);
        cache.insert(key.to_string(), CacheEntry {
            winner: winner.to_string(),
            count: 1,
        });
    }
}

/// Clear the cache
pub fn clear_cache() {
    if let Ok(mut cache) = ALGORITHM_CACHE.lock() {
        cache.clear();
    }
}

/// Get cache statistics
pub fn cache_stats() -> (usize, Vec<(String, String, u32)>) {
    let cache = match ALGORITHM_CACHE.lock() {
        Ok(c) => c,
        Err(c) => c.into_inner(),
    };
    let entries: Vec<_> = cache.iter()
        .map(|(k, v)| (k.clone(), v.winner.clone(), v.count))
        .collect();
    (cache.len(), entries)
}

// ============================================================================
// Sorting Algorithms
// ============================================================================

pub mod sort {
    use super::*;

    /// QuickSort algorithm
    pub fn quicksort(data: &mut [i64]) {
        if data.len() <= 1 { return; }
        let pivot = data.len() / 2;
        data.swap(pivot, data.len() - 1);
        let mut i = 0;
        for j in 0..data.len() - 1 {
            if data[j] <= data[data.len() - 1] {
                data.swap(i, j);
                i += 1;
            }
        }
        data.swap(i, data.len() - 1);
        quicksort(&mut data[..i]);
        quicksort(&mut data[i + 1..]);
    }

    /// MergeSort algorithm  
    pub fn mergesort(data: &mut [i64]) {
        if data.len() <= 1 { return; }
        let mid = data.len() / 2;
        let mut left = data[..mid].to_vec();
        let mut right = data[mid..].to_vec();
        mergesort(&mut left);
        mergesort(&mut right);
        
        let mut i = 0;
        let mut j = 0;
        let mut k = 0;
        while i < left.len() && j < right.len() {
            if left[i] <= right[j] {
                data[k] = left[i];
                i += 1;
            } else {
                data[k] = right[j];
                j += 1;
            }
            k += 1;
        }
        while i < left.len() { data[k] = left[i]; i += 1; k += 1; }
        while j < right.len() { data[k] = right[j]; j += 1; k += 1; }
    }

    /// HeapSort algorithm
    pub fn heapsort(data: &mut [i64]) {
        let n = data.len();
        for i in (0..n / 2).rev() {
            heapify(data, n, i);
        }
        for i in (1..n).rev() {
            data.swap(0, i);
            heapify(data, i, 0);
        }
    }

    fn heapify(data: &mut [i64], heap_size: usize, root: usize) {
        let mut largest = root;
        let left = 2 * root + 1;
        let right = 2 * root + 2;
        
        if left < heap_size && data[left] > data[largest] { largest = left; }
        if right < heap_size && data[right] > data[largest] { largest = right; }
        
        if largest != root {
            data.swap(root, largest);
            heapify(data, heap_size, largest);
        }
    }

    /// InsertionSort algorithm
    pub fn insertionsort(data: &mut [i64]) {
        for i in 1..data.len() {
            let mut j = i;
            while j > 0 && data[j - 1] > data[j] {
                data.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    /// RadixSort algorithm (for positive integers)
    pub fn radixsort(data: &mut [i64]) {
        if data.is_empty() { return; }
        
        let max = *data.iter().max().unwrap_or(&0);
        if max < 0 { return; }
        
        let mut exp = 1;
        while max / exp > 0 {
            counting_sort(data, exp);
            exp *= 10;
        }
    }

    fn counting_sort(data: &mut [i64], exp: i64) {
        let n = data.len();
        let mut output = vec![0i64; n];
        let mut count = [0i64; 10];
        
        for x in data.iter() {
            let digit = ((x / exp) % 10) as usize;
            count[digit] += 1;
        }
        
        for i in 1..10 {
            count[i] += count[i - 1];
        }
        
        for i in (0..n).rev() {
            let digit = ((data[i] / exp) % 10) as usize;
            count[digit] -= 1;
            output[count[digit] as usize] = data[i];
        }
        
        data.copy_from_slice(&output);
    }
}

// ============================================================================
// Search Algorithms
// ============================================================================

pub mod search {
    use super::*;

    /// Linear search - O(n)
    pub fn linear(data: &[i64], target: i64) -> Option<usize> {
        data.iter().position(|&x| x == target)
    }

    /// Binary search - O(log n) - requires sorted data
    pub fn binary(data: &[i64], target: i64) -> Option<usize> {
        if !is_sorted(data) { return None; }
        binary_helper(data, target, 0, data.len())
    }

    fn binary_helper(data: &[i64], target: i64, left: usize, right: usize) -> Option<usize> {
        if left >= right { return None; }
        let mid = left + (right - left) / 2;
        if data[mid] == target {
            Some(mid)
        } else if data[mid] < target {
            binary_helper(data, target, mid + 1, right)
        } else {
            binary_helper(data, target, left, mid)
        }
    }

    /// Interpolation search - O(log log n) for uniform data
    pub fn interpolation(data: &[i64], target: i64) -> Option<usize> {
        if data.is_empty() || !is_sorted(data) { return None; }
        if data[0] == target { return Some(0); }
        if target < data[0] || target > data[data.len() - 1] { return None; }
        
        let low = 0;
        let high = data.len() - 1;
        
        let pos = low + (((target - data[low]) * (high - low) as i64) 
            / (data[high] - data[low])) as usize;
        
        if data[pos] == target {
            Some(pos)
        } else if data[pos] < target {
            interpolation(&data[pos + 1..], target).map(|i| pos + 1 + i)
        } else {
            interpolation(&data[low..pos], target)
        }
    }

    fn is_sorted(data: &[i64]) -> bool {
        data.windows(2).all(|w| w[0] <= w[1])
    }
}

// ============================================================================
// Hash Algorithms
// ============================================================================

pub mod hash {
    use super::*;

    /// FNV-1a hash
    pub fn fnv(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// DJB2 hash
    pub fn djb2(data: &[u8]) -> u64 {
        let mut hash: u64 = 5381;
        for &byte in data {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u64);
        }
        hash
    }

    /// Simple hash (FNV variant)
    pub fn simple(data: &[u8]) -> u64 {
        let mut hash: u64 = 0;
        for (i, &byte) in data.iter().enumerate() {
            hash = hash.wrapping_add((byte as u64).wrapping_mul(31_u64.wrapping_pow(i as u32)));
        }
        hash
    }
}

// ============================================================================
// Main Selection Function with INTELLIGENCE
// ============================================================================

/// Select the best algorithm with smart pattern detection
pub fn select(algos: Vec<(&str, fn(&mut [i64]))>, data: &mut [i64], config: Config) -> SelectResult<Vec<i64>> {
    // Phase 3: Detect pattern
    let pattern = if config.smart_detection {
        Some(detect_pattern(data))
    } else {
        None
    };

    if config.debug {
        if let Some(p) = &pattern {
            println!("Detected pattern: {:?}", p);
        }
    }

    // Check cache first
    if config.cache_enabled {
        if let Some(p) = &pattern {
            if let Some(cached_winner) = get_cached(p) {
                // Find the cached algorithm
                for (name, algo_fn) in &algos {
                    if name == &cached_winner {
                        let mut d = data.to_vec();
                        let start = Instant::now();
                        algo_fn(&mut d);
                        let elapsed = start.elapsed().as_nanos() as u64;
                        
                        if config.debug {
                            println!("Using cached winner: {}", cached_winner);
                        }
                        
                        return SelectResult {
                            output: d,
                            winner: cached_winner.to_string(),
                            time_ns: elapsed,
                            timings: vec![AlgoTiming {
                                name: cached_winner.to_string(),
                                time_ns: elapsed,
                            }],
                            pattern,
                        };
                    }
                }
            }
        }
    }

    // Run all algorithms and find winner
    let mut timings = Vec::new();
    let mut best_time = u64::MAX;
    let mut best_name = "";
    let mut best_result = data.to_vec();
    
    for (name, algo_fn) in &algos {
        let mut d = data.to_vec();
        
        // Warmup runs
        for _ in 0..config.warmup_runs {
            let mut warmup = data.to_vec();
            algo_fn(&mut warmup);
        }
        
        // Timed run
        let start = Instant::now();
        algo_fn(&mut d);
        let elapsed = start.elapsed().as_nanos() as u64;
        
        timings.push(AlgoTiming {
            name: name.to_string(),
            time_ns: elapsed,
        });
        
        if config.debug {
            println!("  {}: {}ns", name, elapsed);
        }
        
        if elapsed < best_time {
            best_time = elapsed;
            best_name = name;
            best_result = d;
        }
    }
    
    // Cache the winner
    if config.cache_enabled {
        if let Some(p) = &pattern {
            cache_winner(p, best_name);
        }
    }
    
    SelectResult {
        output: best_result,
        winner: best_name.to_string(),
        time_ns: best_time,
        timings,
        pattern,
    }
}

/// Select best search algorithm
pub fn select_search(data: &[i64], target: i64) -> (Option<usize>, String, u64) {
    let algos = [
        ("linear", search::linear as fn(&[i64], i64) -> Option<usize>),
        ("binary", search::binary),
        ("interpolation", search::interpolation),
    ];
    
    let mut best_time = u64::MAX;
    let mut best_result = None;
    let mut best_name = "";
    
    for (name, algo_fn) in &algos {
        let start = Instant::now();
        let result = algo_fn(data, target);
        let elapsed = start.elapsed().as_nanos() as u64;
        
        if elapsed < best_time {
            best_time = elapsed;
            best_result = result;
            best_name = name;
        }
    }
    
    (best_result, best_name.to_string(), best_time)
}

/// Select best hash algorithm
pub fn select_hash(data: &[u8]) -> (u64, String, u64) {
    let algos = [
        ("fnv", hash::fnv as fn(&[u8]) -> u64),
        ("djb2", hash::djb2),
        ("simple", hash::simple),
    ];
    
    let mut best_time = u64::MAX;
    let mut best_result = 0u64;
    let mut best_name = "";
    
    for (name, algo_fn) in &algos {
        let start = Instant::now();
        let result = algo_fn(data);
        let elapsed = start.elapsed().as_nanos() as u64;
        
        if elapsed < best_time {
            best_time = elapsed;
            best_result = result;
            best_name = name;
        }
    }
    
    (best_result, best_name.to_string(), best_time)
}
