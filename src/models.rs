use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct NoteSegment { // a segment of audio 
    pub start_frame: usize,
    pub end_frame: usize,
    pub start_sec: f64,
    pub end_sec: f64,
    pub chord_name: String,
    pub bpm: f32,
    pub sample_rate: u32,
}

