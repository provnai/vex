//! # JCS Extreme Hardening
//!
//! Property-based stress-tests for RFC 8785 compliance and order independence.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    use serde_json::{json, Value};

    // Strategy to generate arbitrary JSON values including edge cases
    fn arb_json() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            // Focus on tricky numbers: 0, -0, very large, very small, integers as floats
            prop_oneof![
                Just(0.0),
                Just(-0.0),
                Just(1e30),
                Just(1e-30),
                Just(1.0),
                any::<f64>(),
            ]
            .prop_map(|f| {
                if f.is_finite() {
                    json!(f)
                } else {
                    json!(0.0)
                }
            }),
            // Strings with poisonous characters (Valid UTF-8)
            prop_oneof![
                any::<String>(),
                Just("\0".to_string()),
                Just("\x1f".to_string()),
                Just("€".to_string()),
                Just("\u{1234}".to_string()),
                Just("\u{10FFFF}".to_string()),
            ]
            .prop_map(Value::String),
        ];

        leaf.prop_recursive(
            8,   // 8 levels deep
            256, // 256 nodes total
            10,  // max 10 elements per collection
            |inner: BoxedStrategy<Value>| {
                prop_oneof![
                    prop::collection::vec(inner.clone(), 0..10).prop_map(Value::Array),
                    prop::collection::btree_map(any::<String>(), inner, 0..10).prop_map(|m| {
                        let mut map = serde_json::Map::new();
                        for (k, v) in m {
                            map.insert(k, v);
                        }
                        Value::Object(map)
                    }),
                ]
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(5000))]

        /// Property: JCS output must be identical for the same logical object,
        /// even across multiple random key permutations.
        #[test]
        fn test_jcs_shuffled_keys(ref val in arb_json()) {
            if let Value::Object(ref map) = val {
                let jcs_ref = serde_jcs::to_vec(val).unwrap();

                let mut keys: Vec<_> = map.keys().cloned().collect();
                let mut rng = thread_rng();

                // Shuffle 20 times per case to keep it efficient but thorough
                for _ in 0..20 {
                    keys.shuffle(&mut rng);
                    let mut shuffled_map = serde_json::Map::new();
                    for key in &keys {
                        shuffled_map.insert(key.clone(), map.get(key).unwrap().clone());
                    }
                    let val_shuffled = Value::Object(shuffled_map);
                    let jcs_shuffled = serde_jcs::to_vec(&val_shuffled).unwrap();

                    prop_assert_eq!(&jcs_ref, &jcs_shuffled, "JCS failed order-independence after shuffle");
                }
            }
        }

        /// Property: JCS must be identical regardless of whether the input
        /// was parsed from pretty-printed or compact JSON.
        #[test]
        fn test_jcs_whitespace_invariance(ref val in arb_json()) {
            let compact_json = serde_json::to_string(val).unwrap();
            let pretty_json = serde_json::to_string_pretty(val).unwrap();

            let val_compact: Value = serde_json::from_str(&compact_json).unwrap();
            let val_pretty: Value = serde_json::from_str(&pretty_json).unwrap();

            let jcs_compact = serde_jcs::to_vec(&val_compact).unwrap();
            let jcs_pretty = serde_jcs::to_vec(&val_pretty).unwrap();

            prop_assert_eq!(jcs_compact, jcs_pretty, "JCS must ignore input whitespace variations");
        }

        /// Property: JCS must be deterministic.
        #[test]
        fn test_jcs_determinism(ref val in arb_json()) {
            let jcs1 = serde_jcs::to_vec(val).unwrap();
            let jcs2 = serde_jcs::to_vec(val).unwrap();
            prop_assert_eq!(jcs1, jcs2);
        }

        /// Property: JCS must handle deep nesting and arbitrary strings without crashing.
        #[test]
        fn test_jcs_robustness(ref val in arb_json()) {
            let _ = serde_jcs::to_vec(val).unwrap();
        }
    }

    #[test]
    fn test_rfc8785_static_edge_cases() {
        // 1. Unicode reordering (lexicographical sort)
        let val = json!({"b": 1, "a": 2, "€": 3, "z": 4});
        let jcs = serde_jcs::to_string(&val).unwrap();
        assert_eq!(jcs, r#"{"a":2,"b":1,"z":4,"€":3}"#);

        // 2. Numbers (Scientific notation vs minimal representation)
        // RFC 8785: "1e1" -> "10", "1.0" -> "1"
        assert_eq!(serde_jcs::to_string(&json!(1.0)).unwrap(), "1");
        assert_eq!(serde_jcs::to_string(&json!(10.0)).unwrap(), "10");
        assert_eq!(serde_jcs::to_string(&json!(1e1)).unwrap(), "10");
        assert_eq!(serde_jcs::to_string(&json!(0.0000001)).unwrap(), "1e-7");

        // 3. Negative zero
        assert_eq!(serde_jcs::to_string(&json!(-0.0)).unwrap(), "0");

        // 4. Empty objects/arrays
        assert_eq!(serde_jcs::to_string(&json!({})).unwrap(), "{}");
        assert_eq!(serde_jcs::to_string(&json!([])).unwrap(), "[]");
    }
}
