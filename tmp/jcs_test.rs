use serde_jcs;
use serde_json::json;

fn main() {
    let map1 = json!({
        "z": 1,
        "a": 2
    });
    let map2 = json!({
        "a": 2,
        "z": 1
    });

    let jcs1 = serde_jcs::to_string(&map1).unwrap();
    let jcs2 = serde_jcs::to_string(&map2).unwrap();

    println!("JCS1: {}", jcs1);
    println!("JCS2: {}", jcs2);
    assert_eq!(jcs1, jcs2);
}
