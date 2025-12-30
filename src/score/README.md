# Score system

A score is a music sheet with a tempo & key signature.
It comprises multiple staves, each corresponding to an instrument.

## Score

The score structure contains the global score data such as the name, tempo (in bpm) and signature. Signature change is not supported yet.

It contains a set of staves, each corresponding to an instrument in the instrument vector. This vector is ignored during serialization since it would involve complicated steps for serde to work.

### Score Playback

The `play()` method of the Score creates an optimized representation of the score using the `CompiledScore` structure, plays it, and then reassigns the instruments back to the score. This approach provides efficient playback while keeping the score structure clean and focused on representation rather than playback details.

## Staff

A staff contains a vector of measures as well as an instrument index, used to map the staff to an instrument in the vector of instrument of the score.

### Staff Instance

A `StaffInstance` is a runtime representation of a Staff used for playback. It contains:

- The instrument for this staff (as a `Box<dyn Instrument>`)
- A `VecDeque` of chords for O(1) access to the next chord to be played
- Current playback position tracking

The `StaffInstance` provides methods to retrieve and play the next chord, making it an essential part of the optimized playback system.

## Measure

A measure contains `Timesignature.0 * Crotchet::duration()` slots for chords to be (e.g. a 4/4 signature will make the measure contain `4 * 64` slots). This allows for the placement of notes down to a Demi-Semi-Hemi-Demi-Semi-Quaver (64th of a crotchet).

The measure contains two main elements:

- `notes: Vec<Chord>` a vector of size `Timesignature.0 * Crotchet::duration()` containing the ordered notes (with potentially many empty spaces). This field is ignored by serde since it contains so many empty spaces.
- `notes_set: Vec<(usize, Chord)>` a vector of indexed chords. This vector contains only the actually declared chords, allowing it to be much smaller than the previous one. This one is saved in the serialized versions.

## Chord

## CompiledScore

A `CompiledScore` is an optimized runtime representation of a Score, designed specifically for efficient playback. It provides the following advantages:

- **Pre-processed Chords**: Converts the complex structure of staves and measures into a simple, linear representation
- **O(1) Access**: Uses data structures that allow constant-time access to the next chord to be played
- **Memory Efficiency**: Only keeps the necessary data for playback, avoiding storing empty spaces
- **Smart Note Handling**: Converts between score notes and instrument notes as needed

The `CompiledScore` tracks the current playback position and manages all the instruments. It handles timing based on the score's tempo and advances all instruments at each tick. When a chord is due to be played, it retrieves it from the appropriate staff instance and plays it using the corresponding instrument.

Here's a typical usage flow:

1. Create a `Score` and populate it with staves, measures, and notes
2. Call the `play()` method on the `Score`
3. The `Score` creates a `CompiledScore` internally
4. The `CompiledScore` optimizes the representation and manages playback
5. After playback, instruments are returned to the `Score`
