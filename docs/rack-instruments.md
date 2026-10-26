# Rack instruments

`treble::patch` compiles `treble-meta` 0.3 format-1 JSON instrument documents
into the existing `core::graph::System`. There is one DSP graph implementation,
shared by native playback and portable/headless hosts. The published
`treble-derive` 0.3 macros describe and validate built-in module configuration.

## Built-in contracts (module version 1)

| Type ID | Ports | Parameters |
| --- | --- | --- |
| `treble.oscillator` | `trigger`: note input; `out`: stereo audio output | `frequency`, `amplitude`, `follow_keys`, `waveform` |
| `treble.low_pass` | `in`: summed stereo audio; `out`: stereo audio | `cutoff`, `resonance` |
| `treble.delay` | `in`: summed stereo audio; `out`: stereo audio | `time`, `feedback`, `mix` |
| `treble.output` | `in`: summed stereo audio; `out`: stereo audio | `level` |
| `treble.note_input` | `in`: note input; `out`: note output | none |

Call `ModuleRegistry::builtins().descriptors()` for exact typed defaults, ranges,
units, update requirements and descriptions. The registry is built-in only;
third-party factories are not yet supported. Unknown IDs/versions are rejected.
Raw implementations remain available through the legacy core API.

The instrument interface selects the audio output endpoints; an Output panel
does not implicitly select the device destination. Exposing its `out` sends it
to the final `AudioOutputSink`, which applies the existing master level/limiter.
Multiple exposed audio outputs sum. Source/filter fanout and summed input ports
use the existing System routes and input accumulators.

For a keyboard bus, expose `treble.note_input`'s `in` as the instrument's
`interface.note_input`, then cable its `out` to one or more oscillator `trigger`
ports. A direct oscillator trigger may also be exposed. Notes route only along
those cables. Unconnected trigger sockets are silent in monophonic mode.

## Loading, editing and saving

`parse_document(&str)` and `serialize_document(&document)` validate only the
portable document structure. Missing plugins, disconnected modules and
unfinished patches remain saveable. Opaque `editor` JSON round-trips unchanged;
it can hold rack positions, scope taps, meter panels or cable appearance without
inventing engine modules. Runtime phases, held keys and effect buffers are never
saved. Hosts own dialogs, paths and atomic writes. Legacy `InstrumentSpec` JSON
is a separate format; there is no implicit migration to a rack document.

`PatchSession::replace_document` accepts a structurally valid draft. Use it for
structural transactions and undo/redo. It does not replace playback. Changes to
the document invalidate older preparations, even if the draft changes back.

```rust
use treble::patch::{ModuleRegistry, PatchSession, parse_document};
let document = parse_document(include_str!("../examples/fixtures/rack.trbl"))?;
let mut session = PatchSession::new(ModuleRegistry::builtins(), document)?;
let snapshot = session.snapshot();
// Move snapshot to a worker if needed; no audio device is needed to prepare.
let prepared = snapshot.prepare(48000.0, 256)?;
let install = session.commit(prepared)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Queue `install` through `App::send_patch` after starting native audio. Prepare
with the running engine rate (`app.audio_metrics().engine_sample_rate`) or the
configured rate before startup. Preparation at a different device rate is wrong
when the engine resamples output. `send_patch` checks the engine rate and returns
`PatchSendError { command, reason }` on mismatch, unstarted engine, full queue or
closed queue; retain/retry `*error.command` when appropriate. It never blocks.

For headless/portable hosts, keep a `PatchRuntime` beside the existing `System`
and call `runtime.apply(&mut system, command)` between rendered blocks. See
`cargo run --example rack --no-default-features` for a complete runnable example.

`set_parameter(module_id, parameter_id, value)` validates and updates the saved
base value, returning `ParameterEdit`. A live edit contains a command that can be
queued immediately. `None` requires preparation: this includes rebuild settings,
an uncommitted draft, or any pending structural/rebuild change. Native live
commands use compiled bindings and never allocate descriptors or compile graphs
on the render path. All declared built-in parameters are user-owned.
Built-in float fields use `f32`: execution validation converts values before
checking the generated, quantized bounds, matching the released derive setters.
Thus a JSON value `0.001` can reach a delay's `0.001` lower bound. Saving retains
the supplied base value; this is representation conversion, not range clamping.

Oscillator `waveform` and delay `time` require preparation. Waveforms use stable
IDs `sine`, `saw`, `square`, `triangle`, `white_noise`, `pink_noise`. Delay sizing
allocates during preparation instead of resizing on a live knob command.

## Revision and ownership rules

Keep one session per playback runtime. `commit` rejects stale/foreign snapshots.
Its returned command must be queued, or retained for retry: commit records host
intent, not device acknowledgement. Committing a newer generation supersedes
older queued installations and controls. Parameter revisions independently discard
out-of-order updates to the same parameter. The native dispatcher uses the same
`PatchRuntime::apply` path as the headless integration tests.

Do not mix legacy graph replacement or instrument scheduling with rack ownership
in one running renderer. Legacy graph replacements reset rack control ownership.
To attach a different session, stop/restart native playback or reset the
host-owned runtime explicitly. Keep asynchronous snapshot results associated with
the same live session and queue each accepted command once.

## Playback support and limits

- Supported: continuous drones and monophonic last-note priority. Each oscillator
  keeps its own held-key stack; releasing the most recent key restores the prior
  held key. `follow_keys = false` retains the base frequency while notes still
  gate amplitude; velocity multiplies the saved amplitude.
- `note_on`/`note_off` target the committed exposed note interface. They require
  monophonic voice configuration. Events are block-boundary controls in this API.
  There is no per-note envelope control in the initial rack oscillator: note-off
  immediately stops that note's source contribution. Filters/delays may ring out.
- Continuous mode starts all oscillators on installation; `all_notes_off` kills
  sources, including drones. It does not erase effect tails.
- Live scalar edits preserve DSP state. Structural installations reset oscillator
  phase, held notes and delay/filter state. There is no state migration or
  crossfade in the rack adapter yet; hosts must retrigger notes if desired.
- Polyphony, feedback cycles, CV/gate/control-rate modulation and custom plugins
  are not implemented. They fail validation rather than silently changing
  meaning. No modulation socket is advertised until its published base/depth/
  mapping semantics are implemented. Base values remain separate and persistent.
- Stereo audio is currently the only audio channel contract. Sample rates must
  be finite and within 8000..=192000 Hz; blocks within 1..=65536 frames.
- This adapter preserves existing render-thread allocation behaviour; it does not
  promise allocation-free graph processing or graph reclamation. Compilation,
  factory construction and delay buffer allocation happen during preparation.

Automatic preparation, debouncing, playback feedback, undo grouping and UI
layout belong to the host. Failed preparation leaves the last installed graph
playing and does not discard the editable draft.
