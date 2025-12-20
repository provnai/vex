// Removed unused import
use vex_macros::vex_tool;

#[vex_tool]
#[allow(dead_code, unused_variables)]
fn test_macro_tool(name: String, age: i32, height: Option<f64>) -> String {
    format!("{} is {} years old", name, age)
}

#[test]
fn verify_macro_parameters() {
    let schema = TEST_MACRO_TOOL_TOOL.parameters;
    assert!(schema.contains("\"name\":{\"type\":\"string\"}"));
    assert!(schema.contains("\"age\":{\"type\":\"integer\"}"));
    assert!(schema.contains("\"height\":{\"type\":\"number\"}"));
    assert!(schema.contains("\"required\":[\"age\",\"name\"]"));
}
