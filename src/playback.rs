use crate::chord::chord_to_frequencies;
use crate::models::NoteSegment;
use csv::Reader;
use rodio::{OutputStream, Sink, Source}; // rodio is a audio playback crate for streaming audio sources
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

// Explanation of what's going on here:
// Rust is an imperative language rather than an object oriented language, as such we don't have "classes". Instead we use 'structs' [akin to C], and then implement functions onto them with 'impl'
// So in OOP terms, the struct is essentially the "class", and the impl blocks are its functions / methods

/// Combines all guitar strings into a single audio source
pub struct PolyphonicChordSource {
    frequencies: Vec<f32>,
    sample_rate: u32,
    num_samples: u64,
    current_sample: u64,
    volume: f32,
    strum_delay_samples: usize,
}

impl PolyphonicChordSource {
    pub fn new(frequencies: Vec<f32>, duration: Duration, sample_rate: u32, volume: f32) -> Self {
        let num_samples = (duration.as_secs_f64() * sample_rate as f64) as u64;
        // 3ms delay between string attacks to simulate a chord being struck
        let strum_delay_samples = (sample_rate as f64 * 0.003) as usize;

        Self {
            frequencies,
            sample_rate,
            num_samples,
            current_sample: 0,
            volume,
            strum_delay_samples,
        }
    }
}

impl Iterator for PolyphonicChordSource { 
    type Item = f32; 

    fn next(&mut self) -> Option<Self::Item> { // calculate and return the next audio sample, or None when completed
        if self.current_sample >= self.num_samples { // check if we have reached the total duration of the chord
            return None; 
        }

        let current_sample_idx = self.current_sample as usize;
        self.current_sample += 1; 

        let mut mixed_sample = 0.0f32; // accumulator for summing the wave amplitudes of all active strings
        let num_strings = self.frequencies.len() as f32; 

        for (string_idx, &freq) in self.frequencies.iter().enumerate() { // iterate through each "guitar string" and its frequency
            let offset = string_idx * self.strum_delay_samples; 
            if current_sample_idx >= offset { 
                let active_sample = current_sample_idx - offset; // samples elapsed since this specific string was struck
                let t = active_sample as f32 / self.sample_rate as f32; // elapsed time in seconds

                // Gemini 3.8, prompt: "How can I make more natural sounding guitar strings in Rust based on my existing code, using rodio ?"
                let fundamental = (2.0 * std::f32::consts::PI * freq * t).sin(); // base sine wave at the fundamental frequency
                let h2 = 0.25 * (2.0 * std::f32::consts::PI * (freq * 2.0) * t).sin(); // 2nd harmonic (octave) at 25% amplitude
                let h3 = 0.10 * (2.0 * std::f32::consts::PI * (freq * 3.0) * t).sin(); // 3rd harmonic (octave + fifth) at 10% amplitude

                let string_wave = fundamental + h2 + h3; // combine fundamental and harmonics to simulate guitar string timbre
                let envelope = (-3.5 * t).exp(); // exponential decay curve for natural acoustic string ring-out

                mixed_sample += string_wave * envelope; // apply the decay envelope and add into the polyphonic mix
            }
        }

        Some((mixed_sample / num_strings.sqrt()) * self.volume) // normalize mix by sqrt of string count to avoid clipping, then scale by volume
    }
}

impl Source for PolyphonicChordSource { // using rodio's Source trait to get audio playback
    fn current_frame_len(&self) -> Option<usize> { 
        None
    }
    fn channels(&self) -> u16 { 
        1
    }
    fn sample_rate(&self) -> u32 { 
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> { 
        Some(Duration::from_secs_f64(
            self.num_samples as f64 / self.sample_rate as f64,
        ))
    }
}

pub fn play_chords_from_csv<P: AsRef<Path>>(csv_path: P) -> Result<(), Box<dyn Error>> {
    let file = File::open(csv_path)?;
    let mut reader = Reader::from_reader(file);

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?; // creating sink for audio playback

    println!("{:<12} {:<12} {:<15}", "Start (s)", "End (s)", "Playing Chord");
    println!("{}", "-".repeat(40));

    for result in reader.deserialize() {
        let segment: NoteSegment = result?;
        let segment_duration_sec = (segment.end_sec - segment.start_sec).max(0.1);

        let beat_duration_sec = 60.0 / segment.bpm as f64; // 60 seconds per minute / BPM = one beat in seconds
        let strum_interval = Duration::from_secs_f64(beat_duration_sec); // time interval between consecutive strums

        let total_strums = (segment_duration_sec / beat_duration_sec).round().max(1.0) as usize; // total strums that fit in the segment

        print!( // segment info for debugging
            "\r{:>5.2}s       {:>5.2}s       {:<15}",
            segment.start_sec, segment.end_sec, segment.chord_name
        );
        std::io::stdout().flush()?;  

        if segment.chord_name != "N.C." { // don't play nothing lol
            let freqs = chord_to_frequencies(&segment.chord_name);

            for strum in 0..total_strums {
                let volume = if strum % 2 == 0 { 0.15 } else { 0.10 }; // alternating volume for natural sound

                // passing all frequencies directly into one mixed wave source; a chord
                let chord_source = PolyphonicChordSource::new(
                    freqs.clone(),
                    strum_interval,
                    segment.sample_rate,
                    volume,
                );

                sink.append(chord_source);
                std::thread::sleep(strum_interval);
            }
        } else {
            std::thread::sleep(Duration::from_secs_f64(segment_duration_sec));
        }
    }

    sink.sleep_until_end();
    println!("\nSynthesized strumming complete.");
    Ok(())
}

