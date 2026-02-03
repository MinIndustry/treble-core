# Filters Review -- DSP Correctness Analysis

This document provides a comprehensive technical review of every filter implementation
in `rustic/src/core/filters/`. Each filter is analyzed for mathematical correctness,
numerical stability, and adherence to established DSP references (primarily Robert
Bristow-Johnson's Audio EQ Cookbook and Julius O. Smith's online DSP resources).

A final section lists commonly needed audio filters that are not yet present in the
codebase.

---

## Table of Contents

1. [LowPassFilter (pass/lowpass.rs)](#1-lowpassfilter)
2. [HighPassFilter (pass/highpass.rs)](#2-highpassfilter)
3. [BandPass (pass/bandpass.rs)](#3-bandpass)
4. [ResonantBandpassFilter (resonant_bandpass.rs)](#4-resonantbandpassfilter)
5. [GainFilter (amplifier.rs)](#5-gainfilter)
6. [Clipper (clipper.rs)](#6-clipper)
7. [DelayFilter (delay.rs)](#7-delayfilter)
8. [MovingAverage (moving_average.rs)](#8-movingaverage)
9. [CombinatorFilter (combinator.rs)](#9-combinatorfilter)
10. [DuplicateFilter (structural.rs)](#10-duplicatefilter)
11. [Tremolo (tremolo.rs)](#11-tremolo)
12. [Compressor (compressor.rs)](#12-compressor)
13. [Missing Filters](#13-missing-filters)

---

## 1. LowPassFilter

**File:** `pass/lowpass.rs`

### What It Does

Implements a first-order IIR low-pass filter. The output is a weighted blend of the
current input and the previous output, controlled by a smoothing coefficient `alpha`.

### Current Implementation

```rust
let alpha = self.cutoff_frequency / (self.cutoff_frequency + 1.0);
let output = alpha * input + (1.0 - alpha) * self.previous_output;
```

### Correctness: INCORRECT

The coefficient calculation is **fundamentally wrong**. The formula
`alpha = fc / (fc + 1.0)` treats the cutoff frequency as a dimensionless number and
ignores the sample rate entirely. This means:

- The filter's behavior changes depending on the magnitude of the cutoff frequency
  value rather than its relationship to the Nyquist frequency.
- The cutoff frequency does not correspond to the actual -3 dB point.
- There is no relationship to any standard filter design methodology.

**Standard first-order IIR low-pass** (exponential moving average / one-pole filter):

The correct coefficient derivation uses the bilinear transform or the analog RC
time constant mapped to discrete time:

```
RC = 1 / (2 * pi * fc)
dt = 1 / sample_rate
alpha = dt / (RC + dt)
```

Which simplifies to:

```
alpha = 1 - exp(-2 * pi * fc / sample_rate)
```

Or, using the simpler (but less accurate at high frequencies) approximation:

```
omega_c = 2 * pi * fc / sample_rate
alpha = omega_c / (omega_c + 1.0)
```

### Required Modifications

1. **Add `sample_rate` as a constructor parameter** and store it in the struct.
2. **Fix the alpha calculation** to:
   ```rust
   let omega_c = 2.0 * std::f32::consts::PI * self.cutoff_frequency / self.sample_rate;
   let alpha = omega_c / (omega_c + 1.0);
   ```
   Or for better accuracy (true exponential mapping):
   ```rust
   let alpha = 1.0 - (-2.0 * std::f32::consts::PI * self.cutoff_frequency / self.sample_rate).exp();
   ```
3. The difference equation itself (`alpha * input + (1 - alpha) * prev`) is correct
   for a first-order low-pass once alpha is properly computed.
4. Consider clamping `cutoff_frequency` to `[0, sample_rate / 2]` to prevent
   instability when fc exceeds Nyquist.

### Additional Notes

- A first-order filter has only a -6 dB/octave slope. For most musical applications,
  a second-order (biquad) low-pass with -12 dB/octave is preferred. See the
  Bristow-Johnson cookbook for the standard biquad LPF coefficients.
- The `previous_output` field should be initialized to 0.0 and ideally a `reset()`
  method should be provided (as done in `ResonantBandpassFilter`).

---

## 2. HighPassFilter

**File:** `pass/highpass.rs`

### What It Does

Implements a first-order IIR high-pass filter. Intended to pass frequencies above
the cutoff while attenuating those below.

### Current Implementation

```rust
let alpha = 1.0 / (self.cutoff_frequency + 1.0);
let output = alpha * input + alpha * self.previous_output;
```

### Correctness: INCORRECT

This implementation has **two critical errors**:

**Error 1 -- Missing sample rate:** Same issue as the low-pass filter. The
coefficient `alpha = 1 / (fc + 1)` has no relationship to the actual cutoff
frequency in Hz relative to the sample rate.

**Error 2 -- Wrong difference equation:** A first-order high-pass filter is
derived from the low-pass by computing `HPF = input - LPF(input)`. The standard
first-order high-pass difference equation is:

```
alpha = 1 / (1 + 2 * pi * fc * dt)     // where dt = 1 / sample_rate
y[n] = alpha * (y[n-1] + x[n] - x[n-1])
```

The current equation `y = alpha * x + alpha * y_prev` does not implement a
high-pass filter. It is actually a leaky integrator (a form of low-pass) with
unusual scaling. The formula needs access to the **previous input** as well as the
previous output, which the struct does not currently store.

### Required Modifications

1. **Add `sample_rate` as a constructor parameter.**
2. **Add a `previous_input: f32` field** to the struct.
3. **Fix the difference equation:**
   ```rust
   let dt = 1.0 / self.sample_rate;
   let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff_frequency);
   let alpha = rc / (rc + dt);
   let output = alpha * (self.previous_output + input - self.previous_input);
   self.previous_input = input;
   self.previous_output = output;
   ```
4. Add a `reset()` method to zero out `previous_input` and `previous_output`.

### Additional Notes

- Like the low-pass, a first-order high-pass only provides -6 dB/octave rolloff.
  A biquad (second-order) implementation is strongly recommended for musical use.
- The current implementation will produce incorrect frequency response at all
  frequencies, not just near the cutoff.

---

## 3. BandPass

**File:** `pass/bandpass.rs`

### What It Does

Implements a band-pass filter by cascading a `HighPassFilter` followed by a
`LowPassFilter`. The intent is to pass frequencies between `low` and `high`.

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    self.filters.0.push(self.source, 0);  // feed to HPF
    let value = *self.filters.0.transform().first().unwrap_or(&0.0);
    self.filters.1.push(value, 0);        // feed HPF output to LPF
    self.filters.1.transform()
}
```

### Correctness: INCORRECT (due to dependencies)

The cascade architecture (HPF -> LPF) is a **valid approach** to building a
band-pass filter. However:

1. **Both sub-filters are broken.** Since `LowPassFilter` and `HighPassFilter` have
   incorrect coefficient calculations (see sections 1 and 2), this band-pass filter
   inherits all their defects.
2. **Parameter semantics are confusing.** The `low` field is used as the high-pass
   cutoff (i.e., the lower edge of the passband) and `high` is the low-pass cutoff
   (upper edge). This is correct in terms of signal flow, but the field naming could
   be clearer (e.g., `lower_cutoff` and `upper_cutoff`).
3. **No resonance/Q control.** A cascaded first-order HPF + LPF only gives -6 dB/oct
   on each side with no resonance peak. For most audio applications, a biquad BPF
   (as in the Bristow-Johnson cookbook) with controllable Q/bandwidth is far more
   useful.

### Required Modifications

1. Fix the underlying `LowPassFilter` and `HighPassFilter` first.
2. Pass `sample_rate` through to the sub-filters.
3. Consider replacing with a proper biquad BPF for better control and steeper rolloff.

---

## 4. ResonantBandpassFilter

**File:** `resonant_bandpass.rs`

### What It Does

Implements a second-order (biquad) resonant bandpass filter using Direct Form II
Transposed. Based on the UCSD reference for biquadratic resonant filters.

### Current Implementation

Coefficient calculation:
```rust
let period = 1.0 / sample_frequency;
let bandwidth = center_frequency / quality;
let r: f64 = (-PI * bandwidth as f64 * period as f64).exp();

let b: [f64; 3] = [1.0, 0.0, -r];
let a: [f64; 3] = [
    1.0,
    -2.0 * r * (2.0 * PI * center_frequency as f64 * period as f64).cos(),
    r * r,
];
```

Processing (Direct Form II Transposed):
```rust
let output = self.b[0] * self.source as f64 + self.zs[0];
self.zs[0] = self.b[2] * self.source as f64 - self.a[1] * output + self.zs[1];
self.zs[1] = -self.a[2] * output;
```

### Correctness: MOSTLY CORRECT

This is the best-implemented filter in the codebase. The analysis:

**Coefficient calculation:**
- The pole radius `r = exp(-pi * BW / fs)` is correct for a resonant filter with
  bandwidth `BW = fc / Q`. This places poles at radius `r` inside the unit circle,
  ensuring stability.
- The denominator coefficients `a1 = -2r * cos(2*pi*fc/fs)` and `a2 = r^2` are
  standard for a second-order resonator.
- The numerator `b = [1, 0, -r]` creates a bandpass shape (zero at DC and at
  Nyquist, peak at the center frequency). This matches the UCSD reference.

**Processing (Direct Form II Transposed):**
- The TDF-II implementation is correct. The state update equations properly implement:
  ```
  y[n] = b0*x[n] + z1[n-1]
  z1[n] = b2*x[n] - a1*y[n] + z2[n-1]
  z2[n] = -a2*y[n]
  ```
  Note: `b1 = 0` so the `b1*x[n]` term is correctly omitted from the `z1` update.

**Stability:**
- The filter is stable as long as `r < 1`, which is guaranteed because
  `exp(-positive_value)` is always in `(0, 1)`.
- Using `f64` for coefficients and internal state is good practice, avoiding
  precision issues that can arise with `f32` in recursive filters, especially at
  high Q values.

### Issues Found

1. **Gain is not normalized.** The peak gain of this filter depends on Q. At high Q
   values, the resonant peak can produce very large output amplitudes. The standard
   approach is to normalize by setting `b0 = 1 - r` (or `b0 = (1 - r*r) / 2` for
   the version with `b2 = -r`), so that the peak gain at the center frequency is
   approximately unity.

   Current: `b = [1.0, 0.0, -r]` -- peak gain grows with Q.
   Suggested: `b = [1.0 - r, 0.0, -(1.0 - r) * r]` or normalize by `(1 - r)`.

2. **No runtime parameter update.** The coefficients are computed only in `new()`.
   If the center frequency or Q needs to change during playback, there is no method
   to recalculate coefficients. Adding a `set_parameters(fc, q, fs)` method that
   recomputes `b` and `a` (without resetting `zs`) would allow smooth parameter
   modulation.

3. **The `reset()` method exists**, which is excellent for percussive sounds. No
   issue here.

### Required Modifications

1. Normalize the gain so the passband peak is near unity:
   ```rust
   let gain = 1.0 - r;
   let b: [f64; 3] = [gain, 0.0, -gain * r];
   ```
2. Add a `set_parameters()` method for runtime coefficient updates.

---

## 5. GainFilter

**File:** `amplifier.rs`

### What It Does

Multiplies the input signal by a constant factor (linear gain).

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    let output: f32 = self.sources.map(|f| f * self.factor).iter().sum();
    vec![output]
}
```

### Correctness: CORRECT

This is a trivial gain stage and the implementation is mathematically correct.
The multiplication `input * factor` is the standard linear gain operation.

### Notes and Suggestions

1. **No issues with correctness or stability.** A constant multiplier cannot
   introduce instability.
2. **Consider adding dB-based construction:**
   ```rust
   pub fn from_db(db: f32) -> Self {
       Self::new(10.0f32.powf(db / 20.0))
   }
   ```
   This is more intuitive for audio work where gains are typically expressed in dB.
3. **The `sources` array is fixed at size 1**, so the `.map().iter().sum()` pattern
   is correct but slightly over-engineered for a single input. Not a bug, just a
   minor readability note.

---

## 6. Clipper

**File:** `clipper.rs`

### What It Does

Hard-clips the input signal at a maximum amplitude threshold.

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    vec![if self.source > self.max_ampl {
        self.max_ampl
    } else {
        self.source
    }]
}
```

### Correctness: PARTIALLY INCORRECT

**Bug: Asymmetric clipping.** The current implementation only clips the positive
side of the waveform. Negative values below `-max_ampl` pass through unchanged.
This introduces a DC offset into the signal, which is a serious audio artifact.

For symmetric clipping (the standard behavior), the code should be:

```rust
fn transform(&mut self) -> Vec<f32> {
    vec![self.source.clamp(-self.max_ampl, self.max_ampl)]
}
```

### Required Modifications

1. **Fix asymmetric clipping** by clamping to `[-max_ampl, max_ampl]`. This is a
   one-line fix using `f32::clamp()`.
2. **Consider soft clipping** as an alternative or additional mode. Hard clipping
   introduces harsh odd harmonics. Common soft-clipping functions:
   - `tanh(x)` -- smooth saturation
   - `x / (1 + |x|)` -- cheap approximation
   - `(3/2) * x * (1 - x^2/3)` for `|x| < 1` -- polynomial soft clip

---

## 7. DelayFilter

**File:** `delay.rs`

### What It Does

Delays the input signal by a fixed number of samples, implemented as a FIFO ring
buffer using `VecDeque`.

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    let input = self.sources[0];
    let output = self.buffer.pop_front().unwrap_or(0.0);
    self.buffer.push_back(input);
    vec![output]
}
```

### Correctness: CORRECT

The FIFO delay line implementation is correct. The `VecDeque` is pre-filled with
zeros to the desired delay length, and the pop-front / push-back pattern correctly
implements a fixed delay.

### Issues and Suggestions

1. **`VecDeque` is not ideal for real-time audio.** While functionally correct,
   `VecDeque` may reallocate memory. For a fixed-size delay, a circular buffer
   (ring buffer) using a fixed `Vec` with a read/write index is more efficient and
   avoids potential allocations:
   ```rust
   struct DelayFilter {
       buffer: Vec<f32>,
       write_pos: usize,
       // ...
   }

   fn transform(&mut self) -> Vec<f32> {
       let read_pos = self.write_pos; // oldest sample
       let output = self.buffer[read_pos];
       self.buffer[self.write_pos] = self.sources[0];
       self.write_pos = (self.write_pos + 1) % self.buffer.len();
       vec![output]
   }
   ```
2. **No fractional delay support.** The delay is quantized to integer samples. For
   effects like chorus, flanger, or pitch shifting, fractional-sample delay with
   linear or allpass interpolation is needed.
3. **The Display impl shows `delay_for` in "samples" but the value is actually in
   seconds.** The format string says "samples" but `delay_for` is stored in seconds
   (as set in `new(sample_rate, delay)` where `delay` is in seconds).
4. **`postponable()` returns `true`**, which is correct for use in feedback loops
   within the audio graph.

---

## 8. MovingAverage

**File:** `moving_average.rs`

### What It Does

Implements a simple moving average (SMA) FIR filter that averages the current
input with the previous N samples.

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    let output =
        (self.buffer.iter().fold(0.0, |p, e| p + e) + self.source) / (self.size + 1) as f32;
    for i in (self.size - 1)..0 {
        self.buffer[i] = self.buffer[i - 1];
    }
    self.buffer[0] = self.source;
    vec![output]
}
```

### Correctness: INCORRECT (critical bug)

**Bug: The buffer shift loop never executes.** The range `(self.size - 1)..0`
is an **empty range** in Rust when `self.size - 1 > 0` (which is always the case
for size >= 2). Rust ranges `a..b` are empty when `a >= b`. The loop should use a
**reverse** range:

```rust
for i in (1..self.size).rev() {
    self.buffer[i] = self.buffer[i - 1];
}
```

Because the shift loop never runs, only `self.buffer[0]` ever gets updated. The
remaining buffer elements stay at 0.0 forever. This means:

- For a size-N moving average, the output is `(source + 0 + 0 + ... + 0) / (N+1)`
  which is simply `source / (N+1)` -- an incorrect attenuation rather than an
  average.

### Required Modifications

1. **Fix the buffer shift loop:**
   ```rust
   for i in (1..self.size).rev() {
       self.buffer[i] = self.buffer[i - 1];
   }
   self.buffer[0] = self.source;
   ```
2. **Performance improvement:** The element-by-element shift is O(N). A circular
   buffer approach would make this O(1):
   ```rust
   struct MovingAverage {
       buffer: Vec<f32>,
       write_pos: usize,
       running_sum: f32,
       size: usize,
       // ...
   }

   fn transform(&mut self) -> Vec<f32> {
       self.running_sum -= self.buffer[self.write_pos];
       self.buffer[self.write_pos] = self.source;
       self.running_sum += self.source;
       self.write_pos = (self.write_pos + 1) % self.size;
       vec![self.running_sum / self.size as f32]
   }
   ```
   This also avoids re-summing the entire buffer every sample (the current
   `fold` is O(N) per sample).
3. **The averaging includes `size + 1` samples** (buffer of size N plus the current
   input). This is a design choice, but it means a `MovingAverage::new(4)` actually
   averages 5 samples. Consider whether this is intentional and document it clearly.
4. The `Display` impl is hard-coded to say "4 samples" regardless of the actual
   size. This should use `self.size`.

---

## 9. CombinatorFilter

**File:** `combinator.rs`

### What It Does

Combines multiple input sources into one or more outputs using weighted summation.
Each input has an associated weight, and the output is the weighted sum replicated
to all output ports.

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    let output = self.sources
        .iter()
        .zip(&self.weights)
        .map(|(source, weight)| source * weight)
        .sum();
    vec![output; self.outputs]
}
```

### Correctness: CORRECT

The weighted sum implementation is mathematically correct. This is a standard
mixing/summing operation.

### Notes and Suggestions

1. **Weights are initialized to 1.0 but cannot be changed after construction.**
   Consider adding a `set_weight(port, weight)` method.
2. **No bounds checking in `push()`** -- the function logs an error if the port is
   out of bounds but does not prevent the out-of-bounds access. The
   `self.sources[port]` access will panic. Either add a bounds check with early
   return, or use `.get_mut()`.
3. **No normalization option.** When summing multiple signals, the output can exceed
   `[-1, 1]`. An optional normalization mode (divide by number of active inputs)
   would be useful.

---

## 10. DuplicateFilter

**File:** `structural.rs`

### What It Does

Takes a single input and duplicates it to two outputs. A signal splitter.

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    let source_value = self.sources[0];
    vec![source_value, source_value]
}
```

### Correctness: CORRECT

This is a trivial pass-through that produces two identical copies of the input.
There is nothing that can be mathematically wrong here.

### Notes

- Works as intended. No modifications needed.
- Could be generalized to N outputs using a configurable output count, similar to
  how `CombinatorFilter` supports configurable input/output counts.

---

## 11. Tremolo

**File:** `tremolo.rs`

### What It Does

Applies amplitude modulation (tremolo) to the input signal using a sinusoidal LFO.
The modulation depth is controlled by `upper_range` and `lower_range` parameters.

### Current Implementation

```rust
fn transform(&mut self) -> Vec<f32> {
    self.time += 1.0 / 44100.0;
    vec![
        self.source
            * ((self.frequency * self.time).sin() * (self.upper_range - self.lower_range)
                + (self.lower_range + self.upper_range) / 2.0),
    ]
}
```

### Correctness: PARTIALLY INCORRECT

**Issue 1 -- Hard-coded sample rate of 44100 Hz.** The time increment
`1.0 / 44100.0` assumes a fixed sample rate. If the system runs at 48000 Hz or
96000 Hz, the tremolo frequency will be incorrect. The sample rate should be a
constructor parameter.

**Issue 2 -- Missing 2*PI in the sine argument.** The sine function is called as
`(self.frequency * self.time).sin()`. For a frequency `f` in Hz, the correct
angular argument is `2 * PI * f * t`. Without `2 * PI`, a tremolo frequency of
1 Hz will actually oscillate at `1 / (2*PI) ~= 0.159 Hz`, which is approximately
6.28x too slow.

**Issue 3 -- Floating-point time accumulation.** The `time` field accumulates
indefinitely. After extended playback (hours), `f32` precision degrades
significantly. For example, at 44100 Hz, after ~6 minutes `time` exceeds 16384,
and the least significant bit of an `f32` exceeds the time step, causing the LFO to
freeze. Solutions:
- Use a phase accumulator that wraps at `2*PI` (or 1.0) instead of an unbounded
  time counter.
- Use `f64` for the time/phase accumulator.

**Issue 4 -- The modulation formula produces values outside [0, 1] range** when
`sin()` returns values near -1 or +1, depending on the `lower_range` and
`upper_range` settings. The midpoint is `(lower + upper) / 2` and the swing is
`(upper - lower)`. Since `sin` ranges from -1 to +1, the modulator ranges from
`(lower + upper)/2 - (upper - lower)` to `(lower + upper)/2 + (upper - lower)`,
which is `(3*lower - upper)/2` to `(3*upper - lower)/2`. This can go negative,
inverting the signal phase. The intended range is probably `[lower, upper]`, which
requires dividing the swing by 2:
```rust
(self.frequency * 2.0 * PI * self.time).sin() * (self.upper_range - self.lower_range) / 2.0
    + (self.lower_range + self.upper_range) / 2.0
```

### Required Modifications

1. **Add `sample_rate` parameter** to the constructor.
2. **Fix the sine argument** to include `2 * PI`:
   ```rust
   (2.0 * std::f32::consts::PI * self.frequency * self.time).sin()
   ```
3. **Replace unbounded time with a wrapping phase accumulator:**
   ```rust
   let phase_increment = 2.0 * PI * self.frequency / self.sample_rate;
   self.phase += phase_increment;
   if self.phase >= 2.0 * PI {
       self.phase -= 2.0 * PI;
   }
   let modulator = self.phase.sin() * (self.upper_range - self.lower_range) / 2.0
       + (self.lower_range + self.upper_range) / 2.0;
   ```
4. **Fix the modulation depth scaling** (divide swing by 2, see above).

---

## 12. Compressor

**File:** `compressor.rs`

### What It Does

The file exists but is **completely empty**. No compressor is implemented.

### Required Implementation

A dynamics compressor should include at minimum:

- **Threshold** (dB): level above which compression begins.
- **Ratio** (e.g. 4:1): how much to reduce the signal above threshold.
- **Attack time** (ms): how quickly the compressor engages.
- **Release time** (ms): how quickly the compressor disengages.
- **Makeup gain** (dB): post-compression gain to restore perceived loudness.

The standard approach:
1. Compute the envelope of the input signal (using a ballistics filter with
   attack/release time constants).
2. Compute gain reduction in dB based on threshold and ratio.
3. Smooth the gain reduction with the attack/release envelope.
4. Apply the gain reduction to the input signal.

---

## 13. Missing Filters

The following filters are commonly needed in audio applications and are not
currently present in the codebase. They are grouped by category.

### Frequency-Domain Filters (Biquad-Based)

All of these can be implemented using a single biquad (second-order IIR) structure
with different coefficient formulas. The Robert Bristow-Johnson Audio EQ Cookbook
provides the standard coefficient calculations for all of them.

| Filter | Description | Priority |
|--------|-------------|----------|
| **Biquad LPF** | Second-order low-pass, -12 dB/oct rolloff, with Q control | High |
| **Biquad HPF** | Second-order high-pass, -12 dB/oct rolloff, with Q control | High |
| **Biquad BPF** | Second-order band-pass with Q/bandwidth control | High |
| **Notch (Band-Reject)** | Removes a narrow frequency band; essential for removing hum/feedback | High |
| **All-Pass** | Passes all frequencies but shifts phase; used in phasers and reverbs | Medium |
| **Peaking EQ** | Boosts or cuts a frequency band with adjustable gain, center frequency, and Q | High |
| **Low Shelf** | Boosts or cuts all frequencies below a threshold | Medium |
| **High Shelf** | Boosts or cuts all frequencies above a threshold | Medium |

**Recommendation:** Implement a generic `Biquad` struct with a `FilterType` enum.
All the above filter types share the same processing code (Direct Form II
Transposed); only the coefficient calculation differs. This avoids code
duplication and makes it trivial to add new filter types.

```rust
enum BiquadType {
    LowPass, HighPass, BandPass, Notch, AllPass, PeakingEQ, LowShelf, HighShelf,
}

struct Biquad {
    filter_type: BiquadType,
    b: [f64; 3],
    a: [f64; 3],
    z: [f64; 2],
    sample_rate: f64,
    // ...
}
```

### Time-Domain / Delay-Based Filters

| Filter | Description | Priority |
|--------|-------------|----------|
| **Comb Filter (Feedforward)** | `y[n] = x[n] + g * x[n-M]`; used in flangers, reverbs | Medium |
| **Comb Filter (Feedback / IIR)** | `y[n] = x[n] + g * y[n-M]`; core building block for reverbs | Medium |
| **Allpass Comb** | Combines feedforward and feedback comb; used in Schroeder/Moorer reverbs | Medium |
| **Fractional Delay** | Delay with sub-sample interpolation (linear, cubic, allpass) | Medium |

### Dynamics Processors

| Filter | Description | Priority |
|--------|-------------|----------|
| **Compressor** | Reduces dynamic range above a threshold (file exists but empty) | High |
| **Limiter** | Hard ratio compressor (infinity:1) with fast attack | Medium |
| **Noise Gate** | Silences signal below a threshold | Low |
| **Expander** | Increases dynamic range below a threshold | Low |

### Distortion / Waveshaping

| Filter | Description | Priority |
|--------|-------------|----------|
| **Soft Clipper** | Smooth saturation using tanh, polynomial, or similar curves | Medium |
| **Waveshaper** | Arbitrary transfer function applied to the signal | Low |
| **Bitcrusher** | Reduces bit depth and/or sample rate for lo-fi effects | Low |

### Modulation Effects

| Filter | Description | Priority |
|--------|-------------|----------|
| **Chorus** | Modulated short delay for thickening | Medium |
| **Flanger** | Very short modulated delay with feedback | Medium |
| **Phaser** | Cascaded allpass filters with LFO modulation | Medium |
| **Vibrato** | Pitch modulation via modulated delay (no dry signal) | Low |
| **Ring Modulator** | Multiplies signal by a carrier oscillator | Low |

### Utility

| Filter | Description | Priority |
|--------|-------------|----------|
| **DC Blocker** | Removes DC offset; essential after nonlinear processing | High |
| **Crossfader** | Smooth transition between two signals | Low |
| **Panner** | Stereo positioning (when stereo is supported) | Low |
| **Envelope Follower** | Extracts amplitude envelope from a signal; useful for sidechaining | Medium |

---

## Summary of Critical Issues

| Filter | Severity | Issue |
|--------|----------|-------|
| LowPassFilter | **High** | Missing sample rate in coefficient calculation; incorrect alpha formula |
| HighPassFilter | **High** | Missing sample rate; wrong difference equation (not a high-pass at all) |
| BandPass | **High** | Inherits all defects from LPF and HPF |
| ResonantBandpassFilter | Low | Gain not normalized (can produce very hot output at high Q) |
| Clipper | **Medium** | Only clips positive values; introduces DC offset |
| MovingAverage | **High** | Buffer shift loop never executes; filter only attenuates |
| Tremolo | **Medium** | Hard-coded sample rate; missing 2*PI; unbounded time accumulator; wrong modulation depth |
| Compressor | **Medium** | File is empty; no implementation exists |

Filters with no issues: GainFilter, DelayFilter, CombinatorFilter, DuplicateFilter.
