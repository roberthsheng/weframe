use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoClip {
    pub id: String,
    pub source_file: String,
    pub start_time: Duration,
    pub end_time: Duration,
    pub track: usize,
    pub effects: Vec<Effect>,
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub id: String,
    pub effect_type: EffectType,
    pub start_time: Duration,
    pub end_time: Duration,
    pub parameters: HashMap<String, f64>,
}

impl Effect {
    pub fn new(effect_type: EffectType, value: f64) -> Self {
        let mut parameters = HashMap::new();
        parameters.insert("value".to_string(), value);
        Self {
            id: format!("effect-{}", Uuid::new_v4().to_string()),
            effect_type,
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(0),
            parameters,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffectType {
    Brightness,
    Contrast,
    Saturation,
    Hue,
    Grayscale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    ClientOperation(OTOperation),
    NewClient { client_id: String, name: String },
    ClientDisconnected(String),
    ProjectUpdate(VideoProject),
    ChatMessage { client_id: String, message: String },
    Error { client_id: String, message: String },
    Ping(u64),
    Pong(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub transition_type: TransitionType,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionType {
    Fade,
    Wipe,
    Dissolve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProject {
    pub id: String,
    pub name: String,
    pub clips: Vec<VideoClip>,
    pub duration: Duration,
    pub collaborators: Vec<Collaborator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub id: String,
    pub name: String,
    pub cursor_position: CursorPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub track: usize,
    pub time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOperation {
    AddClip(VideoClip),
    RemoveClip(String),
    MoveClip {
        id: String,
        new_start_time: Duration,
        new_track: usize,
    },
    TrimClip {
        id: String,
        new_start_time: Duration,
        new_end_time: Duration,
    },
    AddEffect {
        clip_id: String,
        effect: Effect,
    },
    RemoveEffect {
        clip_id: String,
        effect_id: String,
    },
    AddTransition {
        clip_id: String,
        transition: Transition,
    },
    RemoveTransition {
        clip_id: String,
    },
    SetProjectDuration(Duration),
    UpdateCollaboratorCursor {
        collaborator_id: String,
        new_position: CursorPosition,
    },
    RenameProject(String),
    AddCollaborator(Collaborator),
    RemoveCollaborator(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OTOperation {
    pub client_id: String,
    pub client_version: usize,
    pub server_version: usize,
    pub operation: EditOperation,
}

impl VideoProject {
    pub fn new(id: String, name: String, client_id: String, client_name: String) -> Self {
        VideoProject {
            id,
            name,
            clips: Vec::new(),
            duration: Duration::from_secs(300),
            collaborators: vec![Collaborator {
                id: client_id,
                name: client_name,
                cursor_position: CursorPosition {
                    track: 0,
                    time: Duration::from_secs(0),
                },
            }],
        }
    }

    pub fn apply_operation(&mut self, op: &EditOperation) {
        match op {
            EditOperation::AddClip(clip) => self.clips.push(clip.clone()),
            EditOperation::RemoveClip(id) => self.clips.retain(|c| c.id != *id),
            EditOperation::MoveClip {
                id,
                new_start_time,
                new_track,
            } => {
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *id) {
                    let duration = clip.end_time - clip.start_time;
                    clip.start_time = *new_start_time;
                    clip.end_time = *new_start_time + duration;
                    clip.track = *new_track;
                }
            }
            EditOperation::TrimClip {
                id,
                new_start_time,
                new_end_time,
            } => {
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *id) {
                    clip.start_time = *new_start_time;
                    clip.end_time = *new_end_time;
                }
            }
            EditOperation::AddEffect { clip_id, effect } => {
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *clip_id) {
                    clip.effects.push(effect.clone());
                }
            }
            EditOperation::RemoveEffect { clip_id, effect_id } => {
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *clip_id) {
                    clip.effects.retain(|e| e.id != *effect_id);
                }
            }
            EditOperation::AddTransition {
                clip_id,
                transition,
            } => {
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *clip_id) {
                    clip.transition = Some(transition.clone());
                }
            }
            EditOperation::RemoveTransition { clip_id } => {
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *clip_id) {
                    clip.transition = None;
                }
            }
            EditOperation::SetProjectDuration(new_duration) => {
                self.duration = *new_duration;
            }
            EditOperation::UpdateCollaboratorCursor {
                collaborator_id,
                new_position,
            } => {
                if let Some(collaborator) = self
                    .collaborators
                    .iter_mut()
                    .find(|c| c.id == *collaborator_id)
                {
                    collaborator.cursor_position = new_position.clone();
                }
            }
            EditOperation::RenameProject(new_name) => {
                self.name = new_name.clone();
            }
            EditOperation::AddCollaborator(collaborator) => {
                self.collaborators.push(collaborator.clone());
            }
            EditOperation::RemoveCollaborator(collaborator_id) => {
                self.collaborators.retain(|c| c.id != *collaborator_id);
            }
        }
    }

    pub fn transform_operation(
        &self,
        client_op: &OTOperation,
        server_version: usize,
    ) -> OTOperation {
        let mut transformed_op = client_op.clone();
        transformed_op.server_version = server_version;

        // Implement more sophisticated transformation logic here if needed
        // This is a simplified version that just updates the server version

        transformed_op
    }
}
