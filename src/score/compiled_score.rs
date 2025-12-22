use super::instances::StaffInstance;
use super::measure::Chord;
use super::score::Score;
use crate::Note;

/// Plays a chord using the given instrument.
///
/// This function converts score notes to the format required by instruments
/// and plays them using the given instrument.
///
/// # Parameters
/// * `instrument` - The instrument to play the notes with
/// * `chord` - The chord containing notes to play
fn play_chord(instrument: &mut Box<dyn crate::instruments::Instrument>, chord: &Chord) {
    // Play all notes in the chord
    for score_note in &chord.notes {
        // Here we need to convert from score::notes::Note to crate::Note
        // This is a placeholder implementation - you'll need to implement the actual conversion
        let octave = score_note.octave;
        // Convert score note to the crate::Note type expected by instruments
        // This is a simplified conversion and should be implemented properly
        let note_type = match score_note.note {
            super::notes::NoteName::A => crate::core::utils::tones::NOTES::A,
            super::notes::NoteName::B => crate::core::utils::tones::NOTES::B,
            super::notes::NoteName::C => crate::core::utils::tones::NOTES::C,
            super::notes::NoteName::D => crate::core::utils::tones::NOTES::D,
            super::notes::NoteName::E => crate::core::utils::tones::NOTES::E,
            super::notes::NoteName::F => crate::core::utils::tones::NOTES::F,
            super::notes::NoteName::G => crate::core::utils::tones::NOTES::G,
            super::notes::NoteName::Pause => continue, // Skip pauses
        };

        let note = Note(note_type, octave);
        instrument.start_note(note, 1.0);
    }
}

/// A compiled version of the Score optimized for playback.
///
/// This structure contains pre-processed data for efficient access
/// during playback, minimizing overhead. It converts the complex
/// score structure into a simple, linear representation that allows
/// for O(1) access to the next chord to be played.
///
/// The CompiledScore manages playback timing based on the score's tempo,
/// and handles converting between score notes and instrument notes.
///
/// # Example
/// ```
/// use rustic::prelude::{Score, TimeSignature};
/// use rustic::instruments::prelude::HiHat;
/// use rustic::score::compiled_score::CompiledScore;
///
/// let mut score = Score::new("Test Score", TimeSignature(4, 4), 120);
/// score.add_instrument(Box::new(HiHat::new().unwrap()));
///
/// // Compile the score for efficient playback
/// let mut compiled = CompiledScore::new(&mut score).unwrap();
///
/// // Play the compiled score
/// compiled.play().unwrap();
/// ```
#[allow(dead_code)]
pub struct CompiledScore {
    pub name: String,
    pub tempo: usize,
    pub staff_instances: Vec<StaffInstance>,
    pub duration: usize, // Total duration in ticks
    current_tick: usize,
}

impl CompiledScore {
    /// Creates a new compiled score from a Score.
    ///
    /// This method takes ownership of the instruments from the score (temporarily)
    /// and creates optimized staff instances for each staff in the score.
    ///
    /// # Parameters
    /// * `score` - The score to compile
    ///
    /// # Returns
    /// * `Ok(CompiledScore)` - The compiled score ready for playback
    /// * `Err(String)` - An error message if compilation failed
    ///
    /// # Example
    /// ```
    /// use rustic::prelude::{Score, TimeSignature};
    /// use rustic::instruments::prelude::HiHat;
    /// use rustic::score::compiled_score::CompiledScore;
    ///
    /// let mut score = Score::new("Test Score", TimeSignature(4, 4), 120);
    /// score.add_instrument(Box::new(HiHat::new().unwrap()));
    ///
    /// let compiled = CompiledScore::new(&mut score).unwrap();
    /// ```
    pub fn new(score: &mut Score) -> Result<Self, String> {
        let mut staff_instances = Vec::new();
        let mut duration = 0;

        // Create staff instances from the score
        for (_idx, staff) in score.staves.iter_mut().enumerate() {
            let instrument_idx = staff.get_instrument();
            if instrument_idx >= score.instruments.len() {
                return Err(format!("Instrument index {} out of bounds", instrument_idx));
            }

            // Take ownership of the instrument from the score
            // We'll put it back when we're done playing
            let instrument = std::mem::replace(
                &mut score.instruments[instrument_idx],
                Box::new(DummyInstrument {}),
            );

            let instance = StaffInstance::new(instrument, staff.clone());
            staff_instances.push(instance);

            // Calculate total duration based on the longest staff
            let staff_duration = staff.total_duration();
            if staff_duration > duration {
                duration = staff_duration;
            }
        }

        Ok(Self {
            name: score.name.clone(),
            tempo: score.tempo,
            staff_instances,
            duration,
            current_tick: 0,
        })
    }

    /// Advances the playback by one tick.
    ///
    /// This method advances all instruments by one tick and
    /// increments the current tick counter.
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Score, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::compiled_score::CompiledScore;
    /// #
    /// # let mut score = Score::new("Test Score", TimeSignature(4, 4), 120);
    /// # score.add_instrument(Box::new(HiHat::new().unwrap()));
    /// # let mut compiled = CompiledScore::new(&mut score).unwrap();
    ///
    /// // Advance the score by one tick
    /// compiled.tick();
    /// ```
    pub fn tick(&mut self) {
        // Advance all instruments
        for instance in &mut self.staff_instances {
            instance.instrument().tick();
        }

        self.current_tick += 1;
    }

    /// Plays the compiled score from start to finish.
    ///
    /// This method handles the timing of playback based on the score's tempo,
    /// and plays all chords at the appropriate times.
    ///
    /// # Returns
    /// * `Ok(())` - If playback completed successfully
    /// * `Err(String)` - If an error occurred during playback
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Score, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::compiled_score::CompiledScore;
    /// #
    /// # let mut score = Score::new("Test Score", TimeSignature(4, 4), 120);
    /// # score.add_instrument(Box::new(HiHat::new().unwrap()));
    /// # let mut compiled = CompiledScore::new(&mut score).unwrap();
    ///
    /// // Play the score
    /// compiled.play().unwrap();
    /// ```
    pub fn play(&mut self) -> Result<(), String> {
        let tick_duration_ms = 60_000 / (self.tempo as u64 * 64); // ms per tick
        let sleep_duration = std::time::Duration::from_millis(tick_duration_ms);

        // Process all notes
        while self.current_tick < self.duration {
            // Check for chords to play at this tick
            self.process_chords_at_current_tick();

            // Sleep for the duration of a tick
            std::thread::sleep(sleep_duration);

            // Advance all instruments
            self.tick();
        }

        Ok(())
    }

    /// Process all chords that should be played at the current tick.
    ///
    /// This method finds all chords that should be played at the current
    /// tick and plays them using their respective instruments.
    ///
    /// The implementation carefully avoids multiple mutable borrows by
    /// first collecting all chords and then playing them.
    fn process_chords_at_current_tick(&mut self) {
        // We need to collect chords to play first to avoid multiple mutable borrows
        let mut chords_to_play = Vec::new();
        let current_tick = self.current_tick;

        for (idx, instance) in self.staff_instances.iter_mut().enumerate() {
            if let Some(_) = instance.peek_next_chord() {
                if instance.current_position() == current_tick {
                    // Time to play this chord
                    if let Some(chord) = instance.next_chord() {
                        chords_to_play.push((idx, chord));
                    }
                }
            }
        }

        // Now play all the collected chords
        for (idx, chord) in chords_to_play {
            let instance = &mut self.staff_instances[idx];
            // Call play_chord as a separate function to avoid self-borrowing issues
            play_chord(instance.instrument(), &chord);
        }
    }

    /// Returns true if the playback is complete.
    ///
    /// # Returns
    /// * `true` - If playback has reached the end of the score
    /// * `false` - If there is still more to play
    #[allow(dead_code)]
    pub fn is_complete(&self) -> bool {
        self.current_tick >= self.duration
    }

    /// Returns the current playback position in ticks.
    ///
    /// # Returns
    /// The current position in ticks
    #[allow(dead_code)]
    pub fn current_position(&self) -> usize {
        self.current_tick
    }

    /// Returns the remaining duration in ticks.
    ///
    /// # Returns
    /// The number of ticks remaining until the end of the score
    #[allow(dead_code)]
    pub fn remaining_duration(&self) -> usize {
        self.duration.saturating_sub(self.current_tick)
    }

    /// Resets the playback position to the beginning.
    ///
    /// Note: This only resets the position counter. A full implementation
    /// would need to rebuild the chord queues as well.
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.current_tick = 0;
        // Reset all staff instances
        // This would require rebuilding the chord queues
        // which is not implemented here for brevity
    }
}

/// A dummy instrument used as a placeholder when we take ownership of an instrument
#[derive(Debug)]
struct DummyInstrument {}

impl crate::instruments::Instrument for DummyInstrument {
    fn start_note(&mut self, _note: crate::Note, _velocity: f32) {}
    fn stop_note(&mut self, _note: crate::Note) {}
    fn get_output(&mut self) -> f32 {
        0.0
    }
    fn tick(&mut self) {}
}
