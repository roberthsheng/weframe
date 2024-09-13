use js_sys::global;
use serde_wasm_bindgen::to_value;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::{console, MessageEvent, WebSocket};
use weframe_shared::{
    CursorPosition, EditOperation, Effect, EffectType, OTOperation, ServerMessage, VideoClip,
    VideoProject,
};
#[wasm_bindgen]
pub struct WeframeClient {
    ws: WebSocket,
    project: Rc<RefCell<VideoProject>>,
    client_id: String,
    client_version: Rc<RefCell<usize>>,
}

#[wasm_bindgen]
impl WeframeClient {
    #[wasm_bindgen(constructor)]
    pub fn new(ws_url: &str, client_id: &str, client_name: &str) -> Result<WeframeClient, JsValue> {
        console::log_1(&JsValue::from_str("Creating new WeframeClient"));
        let ws = WebSocket::new(ws_url)?;
        let project = Rc::new(RefCell::new(VideoProject::new(
            uuid::Uuid::new_v4().to_string(),
            "New Project".to_string(),
            client_id.to_string(),
            client_name.to_string(),
        )));

        let client = WeframeClient {
            ws,
            project,
            client_id: client_id.to_string(),
            client_version: Rc::new(RefCell::new(0)),
        };

        client.setup_ws_handlers();
        Ok(client)
    }

    fn setup_ws_handlers(&self) {
        let project = self.project.clone();
        let client_version = self.client_version.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let txt_string = txt.as_string().unwrap();
                match serde_json::from_str::<ServerMessage>(&txt_string) {
                    Ok(ServerMessage::ClientOperation(operation)) => {
                        console::log_1(&JsValue::from_str(&format!(
                            "Received operation: {:?}",
                            operation
                        )));
                        let mut project = project.borrow_mut();
                        project.apply_operation(&operation.operation);
                        *client_version.borrow_mut() = operation.server_version;

                        // Use js_sys::global() to access the global object
                        let global = global();
                        if let Some(post_message) =
                            js_sys::Reflect::get(&global, &JsValue::from_str("postMessage")).ok()
                        {
                            if let Some(post_message_func) =
                                post_message.dyn_ref::<js_sys::Function>()
                            {
                                let _ = post_message_func.call2(
                                    &global,
                                    &JsValue::from_str(&txt_string),
                                    &JsValue::from_str("*"),
                                );
                            }
                        }
                    }
                    Ok(other_message) => {
                        console::log_1(&JsValue::from_str(&format!(
                            "Received other message: {:?}",
                            other_message
                        )));
                    }
                    Err(e) => {
                        console::error_1(&JsValue::from_str(&format!(
                            "Failed to parse message: {:?}",
                            e
                        )));
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        self.ws
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
    }

    fn send_operation(&self, operation: &OTOperation) -> Result<(), JsValue> {
        let message = serde_json::to_string(&operation)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize operation: {:?}", e)))?;
        self.ws.send_with_str(&message)
    }

    #[wasm_bindgen]
    pub fn get_project(&self) -> Result<JsValue, JsValue> {
        let project = self.project.borrow();
        to_value(&*project).map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
    }

    #[wasm_bindgen]
    pub fn update_cursor_position(&self, track: usize, time: f64) -> Result<(), JsValue> {
        let new_position = CursorPosition {
            track,
            time: std::time::Duration::from_secs_f64(time),
        };

        let mut project = self.project.borrow_mut();
        if let Some(collaborator) = project
            .collaborators
            .iter_mut()
            .find(|c| c.id == self.client_id)
        {
            collaborator.cursor_position = new_position.clone();
        }

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: *self.client_version.borrow(),
            server_version: 0,
            operation: EditOperation::UpdateCollaboratorCursor {
                collaborator_id: self.client_id.clone(),
                new_position,
            },
        };

        *self.client_version.borrow_mut() += 1;
        self.send_operation(&operation).map_err(|e| {
            JsValue::from_str(&format!(
                "Failed to send update_cursor_position operation: {:?}",
                e
            ))
        })
    }

    #[wasm_bindgen]
    pub fn move_clip(
        &self,
        clip_id: &str,
        new_start_time: f64,
        new_track: usize,
    ) -> Result<(), JsValue> {
        console::log_1(&JsValue::from_str(&format!(
            "Moving clip {} to start time {} and track {}",
            clip_id, new_start_time, new_track
        )));
        let mut project = self.project.borrow_mut();
        let clip_index = project
            .clips
            .iter()
            .position(|c| c.id == clip_id)
            .ok_or_else(|| JsValue::from_str("Clip not found"))?;
        let mut clip = project.clips.remove(clip_index);
        clip.start_time = std::time::Duration::from_secs_f64(new_start_time);
        clip.track = new_track;
        project.clips.push(clip);

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: *self.client_version.borrow(),
            server_version: 0,
            operation: EditOperation::MoveClip {
                id: clip_id.to_string(),
                new_start_time: std::time::Duration::from_secs_f64(new_start_time),
                new_track,
            },
        };

        *self.client_version.borrow_mut() += 1;
        self.send_operation(&operation)
    }

    #[wasm_bindgen]
    pub fn resize_clip(&self, clip_id: &str, new_end_time: f64) -> Result<(), JsValue> {
        let mut project = self.project.borrow_mut();

        let clip = project
            .clips
            .iter_mut()
            .find(|c| c.id == clip_id)
            .ok_or_else(|| JsValue::from_str("Clip not found"))?;

        let new_end_time = std::time::Duration::from_secs_f64(new_end_time);
        clip.end_time = new_end_time;

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: *self.client_version.borrow(),
            server_version: 0,
            operation: EditOperation::TrimClip {
                id: clip_id.to_string(),
                new_start_time: clip.start_time,
                new_end_time,
            },
        };

        *self.client_version.borrow_mut() += 1;
        self.send_operation(&operation)
    }

    #[wasm_bindgen]
    pub fn add_clip(
        &self,
        start_time: f64,
        end_time: f64,
        track: usize,
        source_file: &str,
    ) -> Result<(), JsValue> {
        let clip_id = format!("clip-{}", Uuid::new_v4().to_string());
        let new_clip = VideoClip {
            id: clip_id.clone(),
            source_file: source_file.to_string(),
            start_time: std::time::Duration::from_secs_f64(start_time),
            end_time: std::time::Duration::from_secs_f64(end_time),
            track,
            effects: Vec::new(),
            transition: None,
        };

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: *self.client_version.borrow(),
            server_version: 0,
            operation: EditOperation::AddClip(new_clip.clone()),
        };

        *self.client_version.borrow_mut() += 1;
        self.send_operation(&operation)?;

        let mut project = self.project.borrow_mut();
        project.clips.push(new_clip);

        Ok(())
    }

    #[wasm_bindgen]
    pub fn apply_effect(
        &self,
        clip_id: &str,
        effect_type: &str,
        value: f64,
    ) -> Result<(), JsValue> {
        console::log_1(&JsValue::from_str(&format!(
            "Applying effect: {} with value {} to clip {}",
            effect_type, value, clip_id
        )));

        let effect_type = match effect_type {
            "brightness" => EffectType::Brightness,
            "contrast" => EffectType::Contrast,
            "saturation" => EffectType::Saturation,
            "hue" => EffectType::Hue,
            "grayscale" => EffectType::Grayscale,
            _ => return Err(JsValue::from_str("Unsupported effect type")),
        };

        let effect = Effect::new(effect_type, value);

        let mut project = self.project.borrow_mut();

        let clip = project
            .clips
            .iter_mut()
            .find(|c| c.id == clip_id)
            .ok_or_else(|| JsValue::from_str("Clip not found"))?;

        // Remove existing effect of the same type
        clip.effects.retain(|e| e.effect_type != effect.effect_type);
        clip.effects.push(effect.clone());

        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: *self.client_version.borrow(),
            server_version: 0,
            operation: EditOperation::AddEffect {
                clip_id: clip_id.to_string(),
                effect,
            },
        };

        *self.client_version.borrow_mut() += 1;
        self.send_operation(&operation).map_err(|e| {
            JsValue::from_str(&format!("Failed to send apply_effect operation: {:?}", e))
        })
    }

    #[wasm_bindgen]
    pub fn rename_project(&self, new_name: &str) -> Result<(), JsValue> {
        let operation = OTOperation {
            client_id: self.client_id.clone(),
            client_version: *self.client_version.borrow(),
            server_version: 0,
            operation: EditOperation::RenameProject(new_name.to_string()),
        };

        *self.client_version.borrow_mut() += 1;
        self.send_operation(&operation)?;

        let mut project = self.project.borrow_mut();
        project.name = new_name.to_string();

        Ok(())
    }
}
