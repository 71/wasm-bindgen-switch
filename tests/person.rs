use wasm_bindgen_switch::{wasm_bindgen_switch, wasm_bindgen_switch_test};

#[wasm_bindgen_switch(module = "/tests/person.js")]
#[derive(Clone)]
pub struct Person {
    first_name: String,
    last_name: String,
    age: u32,
}

#[wasm_bindgen_switch(camel_case, module = "/tests/person.js")]
impl Person {
    // Constructor.
    #[wasm_bindgen(constructor)]
    pub fn new(first_name: String, last_name: String, age: u32) -> Person {
        Person { first_name, last_name, age }
    }

    // Basic getter.
    #[wasm_bindgen(getter)]
    pub fn age(&self) -> u32 {
        self.age
    }

    // Getter with different name in JS.
    #[wasm_bindgen(getter)]
    pub fn first_name(&self) -> String {
        self.first_name.clone()
    }

    // Getter with different name in JS.
    #[wasm_bindgen(getter)]
    pub fn last_name(&self) -> String {
        self.last_name.clone()
    }

    // Method with different name in JS.
    pub fn full_name(&self) -> String {
        Self::compute_full_name(&self.first_name(), &self.last_name())
    }

    // Static function with custom `js_name`.
    #[wasm_bindgen(js_name = fullName)]
    pub fn compute_full_name(first_name: &str, last_name: &str) -> String {
        format!("{first_name} {last_name}")
    }
}

#[wasm_bindgen_switch_test]
fn test_person() {
    let alice = Person::new("Alice".into(), "Boo".into(), 24);

    assert_eq!(alice.age(), 24);
    assert_eq!(alice.first_name(), "Alice");
    assert_eq!(alice.last_name(), "Boo");
    assert_eq!(alice.full_name(), "Alice Boo");
    assert_eq!(Person::compute_full_name("Barbara", "Boo"), "Barbara Boo");
}
