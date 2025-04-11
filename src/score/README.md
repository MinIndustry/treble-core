# Score system
A score is a music sheet with a tempo & key signature.
It comprises multiple staves, each corresponding to an instrument.

## Score
The score structure contains the global score data such as the name, tempo (in bpm) and signature. Signature change is not supported yet.

It contains a set of staves, each corresponding to an instrument in the instrument vector. This vector is ignored during serialization since it would involve complicated steps for serde to work.

## Staff
A staff contains a vector of measures as well as an instrument index, used to map the staff to an instrument in the vector of instrument of the score.

## Measure
A measure contains `Timesignature.0 * Crotchet::duration()` slots for chords to be (e.g. a 4/4 signature will make the measure contain `4 * 64` slots). This allows for the placement of notes down to a Demi-Semi-Hemi-Demi-Semi-Quaver (64th of a crotchet).

The measure contains two main elements:
* `notes: Vec<Chord>` a vector of size `Timesignature.0 * Crotchet::duration()` containing the ordered notes (with potentially many empty spaces). This field is ignored by serde since it contains so many empty spaces.
* `notes_set: Vec<(usize, Chord)>` a vector of indexed chords. This vector contains only the actually declared chords, allowing it to be much smaller than the previous one. This one is saved in the serialized versions.

## Chord
