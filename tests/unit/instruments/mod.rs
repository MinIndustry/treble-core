//! Instrument Unit Tests
//! Tests for instrument implementations including drums and keyboards

pub mod drum;
pub mod keyboard;
#[cfg(test)]
pub mod registry;
#[cfg(test)]
pub mod spec;

#[cfg(test)]
mod instrument_trait_tests {
    use treble::core::utils::{NOTES, Note};
    use treble::instruments::Instrument;
    use treble::instruments::prelude::{KeyboardBuilder, Synth, SynthConfig};

    #[test]
    fn legacy_instruments_accept_notes_above_the_frequency_table() {
        let note = Note::new(NOTES::A, 42);
        let mut synth = Synth::new(SynthConfig::sine());
        let mut keyboard = KeyboardBuilder::new().with_voices(1).build();

        synth.start_note(note, 1.0);
        keyboard.start_note(note, 1.0);
    }
}
