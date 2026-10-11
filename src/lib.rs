pub mod analysis;
pub mod audio;
pub mod chord;
pub mod models;
pub mod playback;
pub mod storage;

pub use analysis::analyze_audio_to_chords;
pub use models::NoteSegment;
pub use playback::play_chords_from_csv;
pub use storage::write_csv;

// Library file for module exports