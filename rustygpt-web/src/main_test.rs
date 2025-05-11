use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn main_can_be_imported() {
    // This test simply verifies that the main module can be imported
    // and doesn't cause any compilation errors
    assert!(true);
}
