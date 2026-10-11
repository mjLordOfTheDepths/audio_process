use audio_process::{analyze_audio_to_chords, play_chords_from_csv, write_csv};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let audio_path = "input.wav";
    let output_csv_path = "output.csv";

    println!("1. Analyzing BPM and chord progression from {}...", audio_path);
    let (segments, bpm, sample_rate) = analyze_audio_to_chords(audio_path)?;
    write_csv(output_csv_path, &segments)?;
    println!("   Detected Tempo: {:.1} BPM", bpm);
    println!("   Audio Sample Rate: {} Hz", sample_rate);
    println!("   Saved synchronized chord chart to {}\n", output_csv_path);

    println!("2. Synthesizing strummed guitar playback from {}...", output_csv_path);
    play_chords_from_csv(output_csv_path)?;

    Ok(())
}