use crate::audio::decode_audio;
use crate::chord::generate_guitar_chord_templates;
use crate::models::NoteSegment;
use realfft::RealFftPlanner; // realfft is a crate for FFT processing
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::path::Path;

// FFT [Fast Fourier Transform] is a maths algorithm for converting a signal from time to frequency, here it's being used to analyse audio to extract chords - realfft simplifies the process by implementing the algorithm
// Chroma Vector is a 12 dimensional vector representing the intensity of pitch classes and is being used here to detect chords from the FFT output. Gemini 3.8, prompt: "What would I use to detect chords from FFT data ?"

pub fn detect_bpm(samples: &[f32], sample_rate: u32) -> f32 {
    let frame_size = (sample_rate as f64 * 0.02) as usize; // 20ms frames
    let energies: Vec<f32> = samples 
        .chunks(frame_size)
        .map(|chunk| chunk.iter().map(|&s| s * s).sum::<f32>())
        .collect(); // calculate energy for each frame

    let frames_per_sec = sample_rate as f32 / frame_size as f32;
    let min_lag = (frames_per_sec * 60.0 / 180.0) as usize;
    let max_lag = (frames_per_sec * 60.0 / 60.0) as usize;

    let mut best_lag = min_lag; 
    let mut max_corr = 0.0f32;

    for lag in min_lag..=max_lag { // calculate autocorrelation for each lag
        let mut corr = 0.0f32;
        for i in 0..(energies.len() - lag) { // sum the product of energies at time 
            corr += energies[i] * energies[i + lag];
        }
        if corr > max_corr { // if correlation is higher than the previous
            max_corr = corr;
            best_lag = lag;
        }
    }

    let beat_duration_sec = best_lag as f32 / frames_per_sec;
    60.0 / beat_duration_sec // convert to BPM
}

pub fn extract_chroma_vector( // extracts a 12-dimensional chroma vector from the FFT output 
    output_buf: &[realfft::num_complex::Complex32], 
    sample_rate: u32, 
    window_size: usize,
) -> [f32; 12] {
    let mut chroma = [0.0f32; 12];

    for (bin, complex) in output_buf.iter().enumerate().skip(1) { // skip the DC component at bin 0
        let freq = bin as f32 * sample_rate as f32 / window_size as f32;

        if (80.0..=1000.0).contains(&freq) { // only consider frequencies in the range of 80Hz to 1000Hz
            let mag = complex.norm();
            let midi_float = 12.0 * (freq / 440.0).log2() + 69.0; // convert frequency to MIDI note number: Gemini 3.8, prompt: "How would I convert frequency to MIDI note number ? Is there a formula ?"
            let pitch_class = (midi_float.round() as i32).rem_euclid(12) as usize; 
            chroma[pitch_class] += (1.0 + mag).ln();
        }
    }

    let max_val = chroma.iter().cloned().fold(0.0f32, f32::max);
    if max_val > 0.0 {
        for v in chroma.iter_mut() {
            *v /= max_val; // keep chroma values between 0 and 1
        }
    }

    chroma
}

pub fn cosine_similarity(a: &[f32; 12], b: &[f32; 12]) -> f32 { // calculates the cosine similarity between two 12-dimensional vectors for chord matching
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..12 {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i]; 
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else { // return the cosine similarity, which means the closer to 1, the more similar the two vectors are
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

pub fn get_majority_string(history: &VecDeque<String>) -> String {
    let mut counts = HashMap::new();
    for val in history {
        *counts.entry(val.clone()).or_insert(0) += 1;
    }
    counts // return string with highest count
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(k, _)| k)
        .unwrap_or_else(|| "N.C.".to_string())
}

pub fn condense_chords_by_beat(
    raw: Vec<(usize, f64, String)>,
    bpm: f32,
    sample_rate: u32,
) -> Vec<NoteSegment> {
    let beat_duration = 60.0 / bpm as f64; // the duration of one beat in seconds
    let block_duration = beat_duration * 2.0; // group two beats together for accurate chord analysis

    let mut segments: Vec<NoteSegment> = Vec::new();

    if raw.is_empty() {
        return segments;
    }

    let total_duration = raw.last().map(|(_, t, _)| *t).unwrap_or(0.0); // total duration of segment
    let total_blocks = (total_duration / block_duration).ceil() as usize; 

    for b in 0..total_blocks {
        let block_start = b as f64 * block_duration;
        let block_end = block_start + block_duration; // define the start and end time of the current block

        let mut block_chords = HashMap::new();
        let mut min_frame = usize::MAX;
        let mut max_frame = 0; 

        for (frame_idx, time_sec, chord) in &raw { // iterate through segments and count chords
            if *time_sec >= block_start && *time_sec < block_end {
                *block_chords.entry(chord.clone()).or_insert(0) += 1;
                min_frame = min_frame.min(*frame_idx);
                max_frame = max_frame.max(*frame_idx);
            }
        }

        let dominant_chord = block_chords // find the chord with the highest count
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(k, _)| k)
            .unwrap_or_else(|| "N.C.".to_string());

        if let Some(last) = segments.last_mut() {
            if last.chord_name == dominant_chord {
                last.end_frame = max_frame + 1;
                last.end_sec = block_end;
                continue;
            }
        }

        segments.push(NoteSegment { // create a new NoteSegment for the current block
            start_frame: min_frame,
            end_frame: max_frame + 1,
            start_sec: block_start,
            end_sec: block_end,
            chord_name: dominant_chord,
            bpm,
            sample_rate,
        });
    }

    segments
}

pub fn analyze_audio_to_chords<P: AsRef<Path>>(
    audio_path: P,
) -> Result<(Vec<NoteSegment>, f32, u32), Box<dyn Error>> {
    let (samples, sample_rate) = decode_audio(&audio_path)?; // touple of audio samples and sample rate
    let bpm = detect_bpm(&samples, sample_rate);

    let window_duration_sec = 0.08; // 80ms window for analysis
    let window_size = (sample_rate as f64 * window_duration_sec) as usize; // samples per window
    let hop_size = window_size;

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(window_size);

    let mut input_buf = fft.make_input_vec();
    let mut output_buf = fft.make_output_vec();
    let mut scratch_buf = fft.make_scratch_vec();

    let chord_templates = generate_guitar_chord_templates(); // generate chord templates from chord.rs
    let history_capacity = 10;
    let mut history: VecDeque<String> = VecDeque::with_capacity(history_capacity); // keep track of the last 10 detected chords for smoothing
    let mut displayed_chord = "N.C.".to_string();

    let mut raw_segments = Vec::new();

    for (frame_idx, chunk) in samples.chunks(hop_size).enumerate() { // iterate through audio samples in chunks of hop_size
        if chunk.len() < window_size {
            break;
        }

        let current_sec = (frame_idx * hop_size) as f64 / sample_rate as f64;

        for (j, &sample) in chunk.iter().take(window_size).enumerate() { // apply Hann window to the samples to reduce spectral leakage: Gemini 3.8, prompt: "How can I reduce noise or inaccurate chord detection in my analysis ?" - The Hann window  works by tapering the edges of the audio segment to zero, which reduces spectral leakage [frequency bleeding] in FFT
            let window_val = 0.5
                * (1.0
                    - (2.0 * std::f32::consts::PI * j as f32 / (window_size - 1) as f32).cos());
            input_buf[j] = sample * window_val;
        }

        if fft.process_with_scratch(&mut input_buf, &mut output_buf, &mut scratch_buf).is_err() {
            continue;
        }

        let chroma = extract_chroma_vector(&output_buf, sample_rate, window_size); // extract the chroma vector from the FFT to get chord info

        let mut raw_chord = "N.C.".to_string();
        let total_mag: f32 = chroma.iter().sum(); // magnitude of chroma vector to see if it's considered a chord

        if total_mag > 0.1 { 
            let mut best_sim = 0.40;
            for template in &chord_templates {
                let sim = cosine_similarity(&chroma, &template.profile); // compare the chroma vector to chords in the chord template
                if sim > best_sim {
                    best_sim = sim;
                    raw_chord = template.name.clone();
                }
            }
        }

        if history.len() == history_capacity { // history queue, lifo
            history.pop_front();
        }
        history.push_back(raw_chord);

        let majority = get_majority_string(&history); // most common chord in the queue for smoothness
        if history.iter().filter(|&c| c == &majority).count() >= 6 {
            displayed_chord = majority;
        }

        raw_segments.push((frame_idx, current_sec, displayed_chord.clone()));
    }

    let segments = condense_chords_by_beat(raw_segments, bpm, sample_rate);
    Ok((segments, bpm, sample_rate))
}

