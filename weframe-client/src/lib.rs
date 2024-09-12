// weframe-client/src/lib.rs
use js_sys::Object;
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use web_sys::{console, MessageEvent, WebSocket};
use weframe_shared::{
    Collaborator, CursorPosition, EditOperation, Effect, EffectType, OTOperation, VideoClip,
    VideoProject,
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
        console::log_1(&JsValue::from_str(&format!(
            "Attempting to connect to WebSocket at {}",
            ws_url
        )));
        let ws = WebSocket::new(ws_url).map_err(|e| {
            console::error_1(&JsValue::from_str(&format!(
                "Failed to create WebSocket: {:?}",
                e
            )));
            e
        })?;

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

        // Update error handling
        let onerror_callback = Closure::wrap(Box::new(move |e: JsValue| {
            let error_obj = Object::from(e);
            let error_message = js_sys::Reflect::get(&error_obj, &JsValue::from_str("message"))
                .unwrap_or(JsValue::from_str("Unknown error"));
            console::error_1(&JsValue::from_str(&format!(
                "WebSocket error: {:?}",
                error_message
            )));
        }) as Box<dyn FnMut(_)>);
        self.ws
            .set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }

    pub fn send_operation(&self, operation: JsValue) -> Result<(), JsValue> {
        let operation: OTOperation = from_value(operation)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse operation: {:?}", e)))?;
        {
            let mut project = self.project.lock().unwrap();
            project.apply_operation(&operation.operation);
        }

        let message = serde_json::to_string(&operation)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize operation: {:?}", e)))?;
        self.ws.send_with_str(&message)
    }

    pub fn get_project(&self) -> JsValue {
        let project = self.project.lock().unwrap();
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Getting project. Number of clips: {}",
            project.clips.len()
        )));

        for (index, clip) in project.clips.iter().enumerate() {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Clip {}: id: {}, effects: {:?}",
                index, clip.id, clip.effects
            )));
        }

        let project_value = to_value(&*project).unwrap();
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Project serialized to JsValue: {:?}",
            project_value
        )));

        project_value
    }

    #[wasm_bindgen]
    pub fn update_cursor_position(&mut self, track: usize, time: f64) -> Result<(), JsValue> {
        console::log_1(&JsValue::from_str(
            "update_cursor_position called from JavaScript",
        ));

        let mut project = self
            .project
            .lock()
            .map_err(|_| JsValue::from_str("Failed to lock project"))?;

        let collaborator = project
            .collaborators
            .iter_mut()
            .find(|c| c.id == self.client_id)
            .ok_or_else(|| JsValue::from_str("Collaborator not found"))?;

        collaborator.cursor_position = CursorPosition {
            track,
            time: std::time::Duration::from_secs_f64(time),
        };

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: self.client_version,
            server_version: 0,
            operation: EditOperation::UpdateCollaboratorCursor {
                collaborator_id: self.client_id.clone(),
                new_position: collaborator.cursor_position.clone(),
            },
        };

        self.client_version += 1;
        self.send_operation(
            to_value(&operation)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))?,
        )
    }

    #[wasm_bindgen]
    pub fn move_clip(
        &mut self,
        clip_id: &str,
        new_start_time: f64,
        new_track: usize,
    ) -> Result<(), JsValue> {
        let mut project = self
            .project
            .lock()
            .map_err(|_| JsValue::from_str("Failed to lock project"))?;

        let clip = project
            .clips
            .iter_mut()
            .find(|c| c.id == clip_id)
            .ok_or_else(|| JsValue::from_str("Clip not found"))?;

        let duration = clip.end_time - clip.start_time;
        clip.start_time = std::time::Duration::from_secs_f64(new_start_time);
        clip.end_time = clip.start_time + duration;
        clip.track = new_track;

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: self.client_version,
            server_version: 0,
            operation: EditOperation::MoveClip {
                id: clip_id.to_string(),
                new_start_time: std::time::Duration::from_secs_f64(new_start_time),
                new_track,
            },
        };

        self.client_version += 1;
        self.send_operation(
            to_value(&operation)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))?,
        )
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
        let clip_id = format!("clip-{}", uuid::Uuid::new_v4().to_string());
        let new_clip = VideoClip {
            id: clip_id,
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
            server_version: 0,
            operation: EditOperation::AddClip(new_clip),
        };
        self.client_version += 1;
        self.send_operation(to_value(&operation).unwrap())
    }

    #[wasm_bindgen]
    pub fn apply_effect(
        &mut self,
        clip_id: &str,
        effect_type: &str,
        value: f64,
    ) -> Result<(), JsValue> {
        console::log_1(&JsValue::from_str("apply_effect called from JavaScript"));

        let effect_type = match effect_type {
            "brightness" => EffectType::Brightness,
            "contrast" => EffectType::Contrast,
            "saturation" => EffectType::Saturation,
            "hue" => EffectType::Hue,
            "grayscale" => EffectType::Grayscale,
            _ => return Err(JsValue::from_str("Unsupported effect type")),
        };

        let effect = Effect {
            id: format!("effect-{}", uuid::Uuid::new_v4().to_string()),
            effect_type,
            start_time: std::time::Duration::from_secs(0),
            end_time: std::time::Duration::from_secs(0),
            parameters: vec![("value".to_string(), value)].into_iter().collect(),
        };

        let mut project = self
            .project
            .lock()
            .map_err(|_| JsValue::from_str("Failed to lock project"))?;

        let clip = project
            .clips
            .iter_mut()
            .find(|c| c.id == clip_id)
            .ok_or_else(|| JsValue::from_str("Clip not found"))?;

        // Remove existing effects of the same type
        clip.effects.retain(|e| e.effect_type != effect.effect_type);
        // Add the new effect
        clip.effects.push(effect.clone());

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: self.client_version,
            server_version: 0,
            operation: EditOperation::AddEffect {
                clip_id: clip_id.to_string(),
                effect,
            },
        };

        self.client_version += 1;
        self.send_operation(
            to_value(&operation)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))?,
        )
    }
}
