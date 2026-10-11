use std::error::Error;
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer; // symphonia is a Rust audio decoding library for reading various audio files
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn decode_audio<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32), Box<dyn Error>> { 
    let file = File::open(&path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new(); // file extension hint for the audio format
    if let Some(ext) = path.as_ref().extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext); // format detection
    }

    let probed = symphonia::default::get_probe().format( // probe the audio format
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format.default_track().ok_or("No audio track found")?; // if there's no default track, return an error, else get the default track for decoding
    let sample_rate = track.codec_params.sample_rate.ok_or("Unknown sample rate")?;
    let channels = track.codec_params.channels.map_or(1, |c| c.count()); // if there's no channel info, default to 1 channel (mono)

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let mut pcm_samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet, 
            Err(SymphoniaError::IoError(err)) // if there's an I/O error, check if it's an unexpected EOF
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(err) => return Err(Box::new(err)),
        };

        let decoded = decoder.decode(&packet)?;
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec()); // create sample buffer to hold decoded audio samples
        sample_buf.copy_interleaved_ref(decoded);

        let samples = sample_buf.samples();

        if channels >= 2 { // hopefully get rhythm / instrumentals by removing central channel - most recordings have vocals in the central - some Machine Learning Crates exist in Rust for vocal removal - I may explore them in the future. - For now, this works
            for frame in samples.chunks(channels) {
                let left = frame[0];
                let right = frame[1];
                let side_channel = (left - right) * 0.5;
                pcm_samples.push(side_channel);
            }
        } else {
            for frame in samples.chunks(channels) {
                pcm_samples.push(frame[0]);
            }
        }
    }

    Ok((pcm_samples, sample_rate))
}

