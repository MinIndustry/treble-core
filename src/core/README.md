# Core
The core functionalities of the Rustic project.

## Generators
The project starts with generators, which are used to create waveforms.
A Generator is a structure that contains a tone generator (a structure implementing the `ToneGenerator` trait) which defines the frequency and type of sound the generator will have (e.g. Square wave, Sine wave, noise, ...), an amplitude envelope (a structure implementing the `Envelope` trait) which defines the amplitude of the sound over time and a pitch envelope (a structure implementing the `Envelope` trait) which defines the pitch of the sound over time.

Multiple other traits allow to extend the generator functionalities;
* `BendableGenerator`; Allows for a modification of the generator's pitch during runtime (variation of the time scale of the generator).
* `VariableGenerator`; Allows for a modification of the generator's fundamental frequency during runtime. The frequency transition can be described by the `FrequencyTransition` enum.
* `VariableBendableGenerator`; Allows for a modification of the generator's frequency and pitch during runtime.
