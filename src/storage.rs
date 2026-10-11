use crate::models::NoteSegment;
use csv::Writer;
use std::error::Error;
use std::fs::File;
use std::path::Path;

pub fn write_csv<P: AsRef<Path>>(path: P, segments: &[NoteSegment]) -> Result<(), Box<dyn Error>> { // write the NoteSegment data to a CSV file at the specified path
    let file = File::create(path)?;
    let mut writer = Writer::from_writer(file);
    for segment in segments {
        writer.serialize(segment)?;
    }
    writer.flush()?;
    Ok(())
}

