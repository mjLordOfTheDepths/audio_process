
# Simple Chord Detection Tool
## This is a chord detection tool I intend to learn from / implement further in my final year project.
To try it out, simply clone the repo, place a .wav file into the /src directory, and run `cargo run`.   
I've used FFTs, built chroma vectors from them, compared the vectors to a template of guitar chords, then matched said chord to MIDI note frequencies.   
The "strumming" sounds inaccurate, though that is not an important factor right now.
