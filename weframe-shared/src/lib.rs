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
}
