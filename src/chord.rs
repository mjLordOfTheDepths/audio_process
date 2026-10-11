#[derive(Debug, Clone)]
pub struct ChordTemplate {
    pub name: String,
    pub profile: [f32; 12],
}

pub fn generate_guitar_chord_templates() -> Vec<ChordTemplate> {
    let note_names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let mut templates = Vec::new();

    for root in 0..12 { // iterate through all 12 roots
        let root_name = note_names[root];

        let mut maj = [0.0; 12]; // Major
        maj[root] = 1.0; // root
        maj[(root + 4) % 12] = 0.7; // third
        maj[(root + 7) % 12] = 0.8; // fifth
        templates.push(ChordTemplate {
            name: root_name.to_string(),
            profile: maj,
        });

        let mut min = [0.0; 12]; // Minor
        min[root] = 1.0; // root
        min[(root + 3) % 12] = 0.7; // third
        min[(root + 7) % 12] = 0.8; // fifth
        templates.push(ChordTemplate {
            name: format!("{}m", root_name),
            profile: min,
        });

        let mut dom7 = [0.0; 12]; // Dominant 7th
        dom7[root] = 1.0; // root
        dom7[(root + 4) % 12] = 0.6; // third
        dom7[(root + 7) % 12] = 0.7; // fifth
        dom7[(root + 10) % 12] = 0.6; // seventh
        templates.push(ChordTemplate {
            name: format!("{}7", root_name),
            profile: dom7,
        });
    }

    templates
}

/// converting chord names to frequencies for guitar string
pub fn chord_to_frequencies(chord_name: &str) -> Vec<f32> {
    let (root_str, is_minor, is_dom7) = if chord_name.ends_with('m') { // check if the chord is minor
        (&chord_name[..chord_name.len() - 1], true, false) // check root minor
    } else if chord_name.ends_with('7') {
        (&chord_name[..chord_name.len() - 1], false, true) // check root seventh
    } else {
        (chord_name, false, false) // check root major
    };

    let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let root_idx = note_names.iter().position(|&n| n == root_str).unwrap_or(0); // find the index of the root note in the note_names array

    // Gemini 3.8, prompt: "How can I convert chord names to frequencies for guitar strings ?"
    // Open guitar string positions: E2=40, A2=45, D3=50, G3=55, B3=59, E4=64
    let midi_notes = match (root_idx, is_minor, is_dom7) { // return MIDI note numbers
        (4, false, false) => vec![40, 47, 52, 56, 59, 64], // E Major
        (4, true, false)  => vec![40, 47, 52, 55, 59, 64], // E Minor
        (4, false, true)  => vec![40, 47, 50, 56, 59, 64], // E7

        (9, false, false) => vec![45, 52, 57, 61, 64],    // A Major
        (9, true, false)  => vec![45, 52, 57, 60, 64],    // A Minor
        (9, false, true)  => vec![45, 52, 55, 61, 64],    // A7

        (0, false, false) => vec![48, 52, 55, 60, 64],    // C Major
        (0, false, true)  => vec![48, 52, 55, 58, 64],    // C7

        (7, false, false) => vec![43, 47, 50, 55, 59, 67], // G Major
        (7, false, true)  => vec![43, 47, 50, 55, 58, 67], // G7

        (2, false, false) => vec![50, 57, 62, 66],        // D Major
        (2, true, false)  => vec![50, 57, 62, 65],        // D Minor
        (2, false, true)  => vec![50, 57, 60, 66],        // D7

        _ => {
            let root = 40 + root_idx as i32;
            let third = if is_minor { root + 3 } else { root + 4 }; // calculate the third based on whether the chord is minor or major
            vec![root, root + 7, third + 12, root + 19] 
        }
    };

    midi_notes
        .into_iter()
        .map(|m| 440.0 * 2.0f32.powf((m as f32 - 69.0) / 12.0))
        .collect()
}

