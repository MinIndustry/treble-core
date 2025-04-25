use super::measure::Chord;
use super::staff::Staff;
use crate::instruments::Instrument;
use std::collections::VecDeque;

/// StaffInstance provides a runtime representation of a Staff optimized for playback.
///
/// It contains a queue of chords that need to be played in order, allowing for
/// O(1) access to the next chord. It also tracks the current playback position
/// and manages the instrument that plays these chords.
///
/// # Example
/// ```
/// use rustic::prelude::{Staff, TimeSignature};
/// use rustic::instruments::prelude::HiHat;
/// use rustic::score::instances::StaffInstance;
///
/// let staff = Staff::new(&TimeSignature(4, 4));
/// let instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
/// ```
pub struct StaffInstance {
    instrument: Box<dyn Instrument>,
    chords: VecDeque<Chord>, // Using VecDeque for O(1) access to next chord
    current_position: usize, // Current position in playback (in ticks)
}

impl StaffInstance {
    /// Creates a new StaffInstance with the given instrument and staff.
    ///
    /// This converts the staff's ordered chords into a VecDeque for efficient
    /// access during playback.
    ///
    /// # Parameters
    /// * `instrument` - The instrument that will play the chords
    /// * `staff` - The staff containing the chords to be played
    ///
    /// # Returns
    /// A new StaffInstance ready for playback
    ///
    /// # Example
    /// ```
    /// use rustic::prelude::{Staff, TimeSignature};
    /// use rustic::instruments::prelude::HiHat;
    /// use rustic::score::instances::StaffInstance;
    ///
    /// let staff = Staff::new(&TimeSignature(4, 4));
    /// let instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
    /// ```
    pub fn new(instrument: Box<dyn Instrument>, staff: Staff) -> Self {
        let chords = VecDeque::from(staff.get_orderer_chords());
        Self {
            instrument,
            chords,
            current_position: 0,
        }
    }

    /// Returns the next chord to be played and removes it from the queue.
    ///
    /// This method also updates the current position based on the duration
    /// of the chord being removed.
    ///
    /// # Returns
    /// * `Some(Chord)` - The next chord to be played
    /// * `None` - If there are no more chords to play
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Staff, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::instances::StaffInstance;
    /// #
    /// # let staff = Staff::new(&TimeSignature(4, 4));
    /// # let mut instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
    ///
    /// if let Some(chord) = instance.next_chord() {
    ///     // Play the chord
    ///     println!("Playing chord with {} notes", chord.notes.len());
    /// }
    /// ```
    pub fn next_chord(&mut self) -> Option<Chord> {
        let chord = self.chords.pop_front();
        if let Some(ref chord) = chord {
            self.current_position += chord.duration();
        }
        chord
    }

    /// Peeks at the next chord without removing it from the queue.
    ///
    /// This is useful for checking if a chord should be played at
    /// the current position without actually removing it.
    ///
    /// # Returns
    /// * `Some(&Chord)` - A reference to the next chord
    /// * `None` - If there are no more chords to play
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Staff, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::instances::StaffInstance;
    /// #
    /// # let staff = Staff::new(&TimeSignature(4, 4));
    /// # let instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
    ///
    /// if let Some(chord) = instance.peek_next_chord() {
    ///     println!("Next chord has {} notes", chord.notes.len());
    /// }
    /// ```
    pub fn peek_next_chord(&self) -> Option<&Chord> {
        self.chords.front()
    }

    /// Returns a mutable reference to the instrument for this staff.
    ///
    /// This allows the instrument to be used to play notes or be
    /// modified during playback.
    ///
    /// # Returns
    /// A mutable reference to the instrument
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Staff, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::instances::StaffInstance;
    /// #
    /// # let staff = Staff::new(&TimeSignature(4, 4));
    /// # let mut instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
    ///
    /// let instrument = instance.instrument();
    /// // Now we can use the instrument
    /// instrument.tick();
    /// ```
    pub fn instrument(&mut self) -> &mut Box<dyn Instrument> {
        &mut self.instrument
    }

    /// Returns true if there are no more chords to play.
    ///
    /// # Returns
    /// * `true` - If the chord queue is empty
    /// * `false` - If there are still chords to play
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Staff, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::instances::StaffInstance;
    /// #
    /// # let staff = Staff::new(&TimeSignature(4, 4));
    /// # let instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
    ///
    /// if instance.is_empty() {
    ///     println!("No more chords to play");
    /// } else {
    ///     println!("Still have chords to play");
    /// }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.chords.is_empty()
    }

    /// Returns the current position in ticks.
    ///
    /// The position is updated whenever a chord is removed from the queue.
    ///
    /// # Returns
    /// The current position in ticks
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Staff, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::instances::StaffInstance;
    /// #
    /// # let staff = Staff::new(&TimeSignature(4, 4));
    /// # let instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
    ///
    /// println!("Current position: {}", instance.current_position());
    /// ```
    pub fn current_position(&self) -> usize {
        self.current_position
    }

    /// Takes ownership of the instrument from this instance.
    ///
    /// This is used when returning the instrument to the Score after playback.
    ///
    /// # Returns
    /// The instrument that was being used by this instance
    ///
    /// # Example
    /// ```
    /// # use rustic::prelude::{Staff, TimeSignature};
    /// # use rustic::instruments::prelude::HiHat;
    /// # use rustic::score::instances::StaffInstance;
    /// #
    /// # let staff = Staff::new(&TimeSignature(4, 4));
    /// # let instance = StaffInstance::new(Box::new(HiHat::new().unwrap()), staff);
    ///
    /// let instrument = instance.take_instrument();
    /// // The instrument can now be used elsewhere
    /// ```
    pub fn take_instrument(self) -> Box<dyn Instrument> {
        self.instrument
    }
}
