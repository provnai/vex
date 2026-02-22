use vex_algoswitch::{
    cache_stats, cache_winner, clear_cache, detect_pattern, get_cached, pattern_name, DataPattern,
};
use vex_algoswitch::{hash, search, select, select_hash, select_search, sort, Config};

#[test]
fn test_quicksort() {
    let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let expected = vec![1, 1, 2, 3, 4, 5, 6, 9];

    sort::quicksort(&mut data);
    assert_eq!(data, expected);
}

#[test]
fn test_mergesort() {
    let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let expected = vec![1, 1, 2, 3, 4, 5, 6, 9];

    sort::mergesort(&mut data);
    assert_eq!(data, expected);
}

#[test]
fn test_heapsort() {
    let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let expected = vec![1, 1, 2, 3, 4, 5, 6, 9];

    sort::heapsort(&mut data);
    assert_eq!(data, expected);
}

#[test]
fn test_insertionsort() {
    let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let expected = vec![1, 1, 2, 3, 4, 5, 6, 9];

    sort::insertionsort(&mut data);
    assert_eq!(data, expected);
}

#[test]
fn test_radixsort() {
    let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let expected = vec![1, 1, 2, 3, 4, 5, 6, 9];

    sort::radixsort(&mut data);
    assert_eq!(data, expected);
}

#[test]
fn test_select() {
    let mut data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let expected = vec![1, 1, 2, 3, 4, 5, 6, 9];

    let result = select(
        vec![
            ("quicksort", sort::quicksort),
            ("mergesort", sort::mergesort),
            ("heapsort", sort::heapsort),
            ("insertionsort", sort::insertionsort),
        ],
        &mut data,
        Config::default(),
    );

    assert_eq!(result.output, expected);
    assert!(!result.winner.is_empty());
    assert!(result.time_ns > 0);
}

#[test]
fn test_pattern_detection_sorted() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let pattern = detect_pattern(&data);
    assert_eq!(pattern, DataPattern::Sorted);
}

#[test]
fn test_pattern_detection_reverse() {
    let data = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];
    let pattern = detect_pattern(&data);
    assert_eq!(pattern, DataPattern::ReverseSorted);
}

#[test]
fn test_pattern_detection_random() {
    let data = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let pattern = detect_pattern(&data);
    assert_eq!(pattern, DataPattern::Random);
}

#[test]
fn test_pattern_detection_nearly_sorted() {
    let data = vec![1, 2, 3, 5, 4, 6, 7, 8, 9];
    let pattern = detect_pattern(&data);
    assert_eq!(pattern, DataPattern::NearlySorted);
}

#[test]
fn test_pattern_detection_few_unique() {
    let data = vec![1, 2, 1, 2, 1, 2, 1, 2, 1, 2]; // 2 unique values (20%), definitely few unique
    let pattern = detect_pattern(&data);
    assert_eq!(pattern, DataPattern::FewUnique);
}

#[test]
fn test_pattern_name() {
    assert_eq!(pattern_name(&DataPattern::Sorted), "sorted");
    assert_eq!(pattern_name(&DataPattern::Random), "random");
    assert_eq!(pattern_name(&DataPattern::NearlySorted), "nearly sorted");
}

#[test]
fn test_caching() {
    clear_cache();

    // Cache a winner for sorted data
    cache_winner(&DataPattern::Sorted, "insertionsort");

    // Should find it in cache
    let winner = get_cached(&DataPattern::Sorted);
    assert_eq!(winner, Some("insertionsort".to_string()));

    // Random should not be cached
    let winner = get_cached(&DataPattern::Random);
    assert_eq!(winner, None);

    // Check stats
    let (count, entries) = cache_stats();
    assert_eq!(count, 1);
    assert_eq!(entries[0].0, "sorted");
    assert_eq!(entries[0].1, "insertionsort");

    clear_cache();
}

#[test]
fn test_smart_selection_with_pattern() {
    clear_cache();

    // Test with sorted data - should detect and cache
    let sorted_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    let result = select(
        vec![
            ("quicksort", sort::quicksort),
            ("mergesort", sort::mergesort),
            ("heapsort", sort::heapsort),
            ("insertionsort", sort::insertionsort),
        ],
        &mut sorted_data.clone(),
        Config::default().with_debug(false),
    );

    // Should detect sorted pattern
    assert_eq!(result.pattern, Some(DataPattern::Sorted));

    // Should have cached the winner
    let cached = get_cached(&DataPattern::Sorted);
    assert!(cached.is_some());

    clear_cache();
}

#[test]
fn test_search_linear() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    assert_eq!(search::linear(&data, 5), Some(4));
    assert_eq!(search::linear(&data, 1), Some(0));
    assert_eq!(search::linear(&data, 9), Some(8));
    assert_eq!(search::linear(&data, 10), None);
}

#[test]
fn test_search_binary() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    assert_eq!(search::binary(&data, 5), Some(4));
    assert_eq!(search::binary(&data, 1), Some(0));
    assert_eq!(search::binary(&data, 9), Some(8));
    assert_eq!(search::binary(&data, 10), None);
}

#[test]
fn test_search_interpolation() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    assert_eq!(search::interpolation(&data, 5), Some(4));
    assert_eq!(search::interpolation(&data, 1), Some(0));
    assert_eq!(search::interpolation(&data, 9), Some(8));
}

#[test]
fn test_select_search() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    let (result, winner, time) = select_search(&data, 5);

    assert_eq!(result, Some(4));
    assert!(!winner.is_empty());
    assert!(time > 0);
}

#[test]
fn test_hash_functions() {
    let data = b"hello world";

    let h1 = hash::fnv(data);
    let h2 = hash::djb2(data);
    let h3 = hash::simple(data);

    assert!(h1 != 0);
    assert!(h2 != 0);
    assert!(h3 != 0);

    assert_eq!(hash::fnv(data), hash::fnv(data));
    assert_eq!(hash::djb2(data), hash::djb2(data));
}

#[test]
fn test_select_hash() {
    let data = b"hello world";

    let (result, winner, time) = select_hash(data);

    assert!(result != 0);
    assert!(!winner.is_empty());
    assert!(time > 0);
}

#[test]
fn test_config() {
    let config = Config::default();
    assert_eq!(config.warmup_runs, 3);
    assert!(config.cache_enabled);
    assert!(config.smart_detection);

    let config = Config::new()
        .with_warmup(5)
        .with_cache(false)
        .with_debug(true)
        .with_smart_detection(false);

    assert_eq!(config.warmup_runs, 5);
    assert!(!config.cache_enabled);
    assert!(config.debug);
    assert!(!config.smart_detection);
}

#[test]
fn test_pattern_recommendations() {
    let sorted = DataPattern::Sorted;
    let recs = sorted.recommended_sort();
    assert!(recs.contains(&"insertionsort"));

    let random = DataPattern::Random;
    let recs = random.recommended_sort();
    assert!(recs.contains(&"quicksort"));
}
