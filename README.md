# wasm-bindgen-switch

Switch between a pure-Rust implementation and a `#[wasm_bindgen]`-backed
implementation based on whether your code is compiled for WebAssembly.

See [`tests/person.rs`](tests/person.rs) (which uses
[`tests/person.js`](tests/person.js) in `wasm` builds) for an example. Both
`cargo test` and `cargo test --target wasm32-unknown-unknown` test the same
code and succeed.

The goal is to be able to write Rust code that is used the exact same way, no
matter the actual implementation (JS or Rust). Ideally, such a project would
not be needed: the Rust core would be Rust-only and a separate, clearly
defined layer would glue Rust and JavaScript together. However, if your
project is designed to work in the browser and uses many browser APIs,
separating all code in such a way might be counterproductive. This is why this
package exists.

My initial approach was to use traits with a macro that would generate
a

```rust
#[wasm_bindgen]
extern "C" {
    // ...
}
```

block which imports functions and implements that trait for a JS value, but
this forces all relevant APIs to be generic on that interface. With dozens
of shared types and hundreds of functions, this is too much work.

More work needed:
- [ ] More tests.
- [ ] Better documentation.
- [ ] [`extends`](https://rustwasm.github.io/docs/wasm-bindgen/reference/attributes/on-js-imports/extends.html),
      [`js_namespace`](https://rustwasm.github.io/docs/wasm-bindgen/reference/attributes/on-js-imports/js_namespace.html).
- [ ] Shims for `JsValue`, `JsError`, `JsCast`, and other types in `js_sys`
      and `web_sys`.
- [ ] Probably a better name?
