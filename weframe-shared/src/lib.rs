use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoClip {
    pub id: String,
    pub start_time: Duration,
    pub end_time: Duration,
    pub track: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProject {
    pub clips: Vec<VideoClip>,
    pub duration: Duration,
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
    SetProjectDuration(Duration),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OTOperation {
    pub client_id: usize,
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
            EditOperation::SetProjectDuration(new_duration) => {
                self.duration = *new_duration;
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
                // fpr adding clips, prefer the client operation but adjust the start time
                if server_op.client_id < client_op.client_id {
                    OTOperation {
                        client_id: client_op.client_id,
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
                    // if both try to remove the same clip, only the server operation is applied
                    OTOperation {
                        client_id: client_op.client_id,
                        client_version: client_op.client_version,
                        server_version: server_op.server_version,
                        operation: EditOperation::RemoveClip(client_id.clone()),
                    }
                } else {
                    // if different clips, both operations are kept
                    client_op.clone()
                }
            }
            (
                EditOperation::MoveClip { id: server_id, .. },
                EditOperation::MoveClip { id: client_id, .. },
            ) => {
                if server_id == client_id {
                    // if moving the same clip, prefer the server operation
                    server_op.clone()
                } else {
                    // if different clips, both operations are kept
                    client_op.clone()
                }
            }
            (
                EditOperation::TrimClip { id: server_id, .. },
                EditOperation::TrimClip { id: client_id, .. },
            ) => {
                if server_id == client_id {
                    // if trimming the same clip, merge the operations
                    let (_, server_start, server_end) = server_op.operation.as_trim_clip();
                    let (_, client_start, client_end) = client_op.operation.as_trim_clip();
                    OTOperation {
                        client_id: client_op.client_id,
                        client_version: client_op.client_version,
                        server_version: server_op.server_version,
                        operation: EditOperation::TrimClip {
                            id: client_id.clone(),
                            new_start_time: std::cmp::max(*server_start, *client_start),
                            new_end_time: std::cmp::min(*server_end, *client_end),
                        },
                    }
                } else {
                    // if different clips, both operations are kept
                    client_op.clone()
                }
            }
            (EditOperation::SetProjectDuration(_), EditOperation::SetProjectDuration(_)) => {
                // for project duration, prefer the server operation
                server_op.clone()
            }
            _ => {
                // for everything else, prefer the client operation
                client_op.clone()
            }
        }
    }
}

// helper to extract fields of a TrimClip operation
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