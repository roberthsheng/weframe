// weframe-client/src/lib.rs
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use web_sys::{console, MessageEvent, WebSocket};
use weframe_shared::{OTOperation, VideoProject};

#[wasm_bindgen]
pub struct WeframeClient {
    ws: WebSocket,
    project: Arc<Mutex<VideoProject>>,
}

#[wasm_bindgen]
impl WeframeClient {
    #[wasm_bindgen(constructor)]
    pub fn new(ws_url: &str) -> Result<WeframeClient, JsValue> {
        let ws = WebSocket::new(ws_url)?;

        let project = Arc::new(Mutex::new(VideoProject {
            clips: Vec::new(),
            duration: std::time::Duration::from_secs(300),
        }));

        let client = WeframeClient { ws, project };

        // Set up WebSocket event handlers
        client.setup_ws_handlers();

        Ok(client)
    }

    fn setup_ws_handlers(&self) {
        let project = self.project.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            let data = e.data().as_string().unwrap();
            let operation: OTOperation = serde_json::from_str(&data).unwrap();
            console::log_1(&JsValue::from_str(&format!(
                "Received operation: {:?}",
                operation
            )));
            // Apply the operation to the local project state
            let mut project = project.lock().unwrap();
            project.apply_operation(&operation.operation);
        }) as Box<dyn FnMut(_)>);
        self.ws
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
    }

    pub fn send_operation(&self, operation: JsValue) -> Result<(), JsValue> {
        let operation: OTOperation = from_value(operation).unwrap();
        {
            let mut project = self.project.lock().unwrap();
            project.apply_operation(&operation.operation);
        }
        let message = serde_json::to_string(&operation).unwrap();
        self.ws.send_with_str(&message)
    }

    pub fn get_project(&self) -> JsValue {
        let project = self.project.lock().unwrap();
        to_value(&*project).unwrap()
    }
}
