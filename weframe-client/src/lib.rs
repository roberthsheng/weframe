use serde_wasm_bindgen::{from_value, to_value};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use web_sys::{console, MessageEvent, WebSocket};
use weframe_shared::{
    Collaborator, CursorPosition, EditOperation, OTOperation, VideoClip, VideoProject,
};

#[wasm_bindgen]
pub struct WeframeClient {
    ws: WebSocket,
    project: Arc<Mutex<VideoProject>>,
    client_id: String,
    client_version: usize,
}

#[wasm_bindgen]
impl WeframeClient {
    #[wasm_bindgen(constructor)]
    pub fn new(ws_url: &str, client_id: &str, client_name: &str) -> Result<WeframeClient, JsValue> {
        let ws = WebSocket::new(ws_url)?;

        let project = Arc::new(Mutex::new(VideoProject {
            clips: Vec::new(),
            duration: std::time::Duration::from_secs(300),
            collaborators: vec![Collaborator {
                id: client_id.to_string(),
                name: client_name.to_string(),
                cursor_position: CursorPosition {
                    track: 0,
                    time: std::time::Duration::from_secs(0),
                },
            }],
        }));

        let client = WeframeClient {
            ws,
            project,
            client_id: client_id.to_string(),
            client_version: 0,
        };

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
        let mut operation: OTOperation = from_value(operation).unwrap();
        operation.client_id = self.client_id.clone();
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

    #[wasm_bindgen]
    pub fn update_cursor_position(&self, track: usize, time: f64) -> Result<(), JsValue> {
        console::log_1(&JsValue::from_str("update_cursor_position called"));
        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: 0, // This should be managed properly in a real implementation
            server_version: 0, // This should be updated based on server responses
            operation: EditOperation::UpdateCollaboratorCursor {
                collaborator_id: self.client_id.clone(),
                new_position: CursorPosition {
                    track,
                    time: std::time::Duration::from_secs_f64(time),
                },
            },
        };
        self.send_operation(to_value(&operation).unwrap())
    }

    #[wasm_bindgen]
    pub fn move_clip(
        &mut self,
        clip_id: String,
        new_start_time: f64,
        new_track: usize,
    ) -> Result<(), JsValue> {
        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: self.client_version,
            server_version: 0,
            operation: EditOperation::MoveClip {
                id: clip_id,
                new_start_time: std::time::Duration::from_secs_f64(new_start_time),
                new_track,
            },
        };
        self.client_version += 1;
        self.send_operation(to_value(&operation).unwrap())
    }

    #[wasm_bindgen]
    pub fn resize_clip(&mut self, clip_id: String, new_end_time: f64) -> Result<(), JsValue> {
        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: self.client_version,
            server_version: 0,
            operation: EditOperation::TrimClip {
                id: clip_id,
                new_start_time: std::time::Duration::from_secs(0),
                new_end_time: std::time::Duration::from_secs_f64(new_end_time),
            },
        };
        self.client_version += 1;
        self.send_operation(to_value(&operation).unwrap())
    }

    #[wasm_bindgen]
    pub fn add_clip(
        &mut self,
        start_time: f64,
        end_time: f64,
        track: usize,
        source_file: &str,
    ) -> Result<(), JsValue> {
        let new_clip = VideoClip {
            id: format!("clip-{}", self.client_version),
            source_file: source_file.to_string(),
            start_time: std::time::Duration::from_secs_f64(start_time),
            end_time: std::time::Duration::from_secs_f64(end_time),
            track,
            effects: Vec::new(),
            transition: None,
        };

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: self.client_version,
            server_version: 0, // This should be updated based on server responses
            operation: EditOperation::AddClip(new_clip),
        };
        self.client_version += 1;
        self.send_operation(to_value(&operation).unwrap())
    }
}
