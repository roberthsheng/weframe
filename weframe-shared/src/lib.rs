use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    Brightness,
    Contrast,
    Saturation,
    Hue,
    Grayscale,
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
    // etc
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProject {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OTOperation {
    pub client_id: String,
    pub client_version: usize,
    pub server_version: usize,
    pub operation: EditOperation,
}

impl VideoProject {
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
        }
    }

    pub fn transform_operation(
        &self,
        server_op: &OTOperation,
        client_op: &OTOperation,
    ) -> OTOperation {
        match (&server_op.operation, &client_op.operation) {
            (EditOperation::AddClip(server_clip), EditOperation::AddClip(client_clip)) => {
                if server_op.client_id < client_op.client_id {
                    OTOperation {
                        client_id: client_op.client_id.clone(),
                        client_version: client_op.client_version,
                        server_version: server_op.server_version + 1,
                        operation: EditOperation::AddClip(VideoClip {
                            start_time: client_clip.start_time + server_clip.end_time
                                - server_clip.start_time,
                            ..client_clip.clone()
                        }),
                    }
                } else {
                    client_op.clone()
                }
            }
            (EditOperation::RemoveClip(server_id), EditOperation::RemoveClip(client_id)) => {
                if server_id == client_id {
                    OTOperation {
                        client_id: client_op.client_id.clone(),
                        client_version: client_op.client_version,
                        server_version: server_op.server_version,
                        operation: EditOperation::RemoveClip(client_id.clone()),
                    }
                } else {
                    client_op.clone()
                }
            }
            (
                EditOperation::MoveClip { id: server_id, .. },
                EditOperation::MoveClip { id: client_id, .. },
            ) => {
                if server_id == client_id {
                    server_op.clone()
                } else {
                    client_op.clone()
                }
            }
            (
                EditOperation::TrimClip { id: server_id, .. },
                EditOperation::TrimClip { id: client_id, .. },
            ) => {
                if server_id == client_id {
                    let (_, server_start, server_end) = server_op.operation.as_trim_clip();
                    let (_, client_start, client_end) = client_op.operation.as_trim_clip();
                    OTOperation {
                        client_id: client_op.client_id.clone(),
                        client_version: client_op.client_version,
                        server_version: server_op.server_version,
                        operation: EditOperation::TrimClip {
                            id: client_id.clone(),
                            new_start_time: std::cmp::max(*server_start, *client_start),
                            new_end_time: std::cmp::min(*server_end, *client_end),
                        },
                    }
                } else {
                    client_op.clone()
                }
            }
            (EditOperation::SetProjectDuration(_), EditOperation::SetProjectDuration(_)) => {
                server_op.clone()
            }
            _ => client_op.clone(),
        }
    }
}

impl EditOperation {
    fn as_trim_clip(&self) -> (&String, &Duration, &Duration) {
        match self {
            EditOperation::TrimClip {
                id,
                new_start_time,
                new_end_time,
            } => (id, new_start_time, new_end_time),
            _ => panic!("Called as_trim_clip on non-TrimClip operation"),
        }
    }
}