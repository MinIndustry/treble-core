# Changelog

## 0.5.2

- Add `treble.adsr`: a note-triggered sample-rate control envelope with live
  attack, decay, sustain and release parameters. Release and retrigger begin at
  the current level; zero-length stages, overlapping notes and panic are defined.
- Add Gate / One-shot / Loop envelope modes and Current / Reset / Legato retrigger
  policies, plus typed sample gate and retrigger inputs.
- Add explicit Control to gate and Gate to control adapters; schedule source stream
  inputs without averaging or rendering shared modulators twice.
- Add oscillator `gate_mode = envelope` for pitched release tails driven by amplitude
  CV. Its default note gating behavior remains unchanged.
- Add `PatchSession::kill_notes` for host voice reuse without killing independent
  constants or modulators. `all_notes_off` retains its global panic behavior.
- Preserve the 0.3 metadata identity and existing module version-1 contracts.

## 0.5.1

- Expand the rack registry from 7 to 32 modules: signed arithmetic, constants,
  channel split/merge/adapters, gain, pan/auto-pan, high/band-pass, moving average,
  reverb, tremolo, compressor, limiter and clipper.
- Route descriptor input/output indices, including mono audio and control streams,
  through the same audio/control schedule. Expose prepared filter handles to hosts.
- Add sample modulation receivers for high-pass, pan, gain, offset, clamp and constants.
- Correct compressor ratio behaviour (ratio 1 is unity; zero threshold stays finite)
  and inject host sample rates into compressor/limiter coefficients.
- Keep existing IDs/version-1 contracts and treble-meta/derive 0.3 identities.
  Buffer-resizing settings remain rebuild-only; signed arithmetic has no implicit
  audio-range clipping. Existing polyphony/feedback limits remain.

## 0.5.0

- Consume published `treble-meta` 0.3 and `treble-derive` 0.3 from crates.io.
- Add portable `.trbl` document parse/save helpers and a built-in module registry
  with typed, stable metadata for oscillators, low-pass, delay, output and note bus.
- Compile validated instruments into the existing `System`, supporting stereo
  fanout/summing and continuous or monophonic key-following/fixed-pitch sources.
- Add explicit background preparation, revision-checked sessions and native
  block-boundary installation/live controls with recoverable nonblocking submission.
- Keep incomplete/unavailable documents saveable and last valid audio running
  after preparation errors. Structural swaps reset DSP/held-note state; polyphony,
  feedback and custom registry factories remain unsupported.
- Apply sample-rate modulation blocks in a combined audio/control dependency
  order. Add continuously running LFOs and explicit audio-to-control conversion;
  support linear oscillator FM, AM, cutoff/resonance and gain CV without mutating
  bases or introducing hidden block latency. Preserve legacy-only graph behavior.
- Repair no-default-feature test/example gates and exercise portable tests in CI.

This is a minor version bump for a pre-1.0 crate: `AudioMessage` gains a `Patch`
variant (downstream exhaustive matches must handle it), and public metadata now
uses the 0.3 crate identity. Existing low-level graph/instrument APIs remain.
See [rack integration](docs/rack-instruments.md) for host migration and limits.
