use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WeframeClient {
    // client-side state
}

#[wasm_bindgen]
impl WeframeClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        WeframeClient {
            // client-side state
        }
    }

    pub fn update(&mut self) {
        // update client-side state
    }

    pub fn render(&self) {
        // rendering video editor interface
    }
}