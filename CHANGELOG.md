# Changelog

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
  CV modulation, feedback and custom registry factories remain unsupported.
- Repair no-default-feature test/example gates and exercise portable tests in CI.

This is a minor version bump for a pre-1.0 crate: `AudioMessage` gains a `Patch`
variant (downstream exhaustive matches must handle it), and public metadata now
uses the 0.3 crate identity. Existing low-level graph/instrument APIs remain.
See [rack integration](docs/rack-instruments.md) for host migration and limits.
