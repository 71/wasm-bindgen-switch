//! The [`#[wasm_bindgen_switch]`](wasm_bindgen_switch) and
//! [`#[wasm_bindgen_switch_test]`](wasm_bindgen_switch_test) attribute macros.
//!
//! `#[wasm_bindgen_switch]` is used to replace Rust items with [`wasm-bindgen`
//! JavaScript imports](https://rustwasm.github.io/docs/wasm-bindgen/reference/attributes/on-js-imports/index.html)
//! at compile-time when targeting `wasm`. This, among other things, allows a
//! package to contain a "default" implementation of a type for testing, and
//! a JavaScript implementation of a type when running in the browser.

pub use wasm_bindgen_switch_macro::{wasm_bindgen_switch, wasm_bindgen_switch_test};
