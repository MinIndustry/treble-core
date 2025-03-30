# Filters
Filters act on the raw music data coming from instruments.
Instruments are expected to provide enveloped signals (amplitude & pitch-wise) and filters are expected to act on these signals.

## List
### Low Pass Filter
A low pass filter is a filter that passes signals with a frequency lower than a certain cutoff frequency and attenuates signals with frequencies higher than the cutoff frequency.
This filter is implemented using a simple IIR filter.
