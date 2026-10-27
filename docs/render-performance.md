# Render performance and host integration

The 0.5.3 changes reduce overhead without changing the module catalog, sample-rate
modulation semantics, or the published treble-meta/derive 0.3 contracts. This is
an incremental optimization release, not an allocation-free rendering guarantee.

## Reuse sink output

Prepare an output block outside the callback, then overwrite it on each cycle:

```rust
use treble::core::{Block, graph::{SimpleSink, Sink}};
let mut sink = SimpleSink::new();
let mut output = Block::with_capacity(256);
sink.consume_into(&mut output);
assert!(output.is_empty());
```

For graph rendering, call `system.run_frames(frames)` and then
`system.get_sink(0)?.consume_into(&mut output)`. Allocate capacity for the maximum
span before rendering. `consume_into` replaces old output rather than appending;
an empty sink empties output. SimpleSink exchanges caller and sink buffers, while
AudioOutputSink writes into the supplied buffer and preserves limiter state and
telemetry. Warm both sides before relying on retained capacity. Larger spans may
still require allocation. Existing/custom sinks compile unchanged: the default
implementation delegates to `consume`, and only optimized implementations promise
storage reuse. `consume` remains unchanged for existing hosts.

Graph nodes retain their input queue allocations and the legacy execution path
retains its source-routing scratch vector. These changes cover both repeated
small spans and normal blocks. Audio Block results, Arc wrappers, multi-input
mix buffers and legacy custom processors can still allocate. A host must not
infer whole-callback allocation freedom from sink reuse alone.

## Bind modulation once per span

`bind_modulation(inputs, parameter_name, effective_base)` returns a borrowed
`ModulationView`; call `view.value(sample_index)` inside the sample loop. It
resolves the parameter name and caches the bounded base/mapping coordinate once,
including octave mapping's logarithm. Signal summing, inverse mapping, clamping
and nonfinite fallback still happen per sample. Multiple wires retain published
Sum semantics for Add/Multiply/Replace. Missing parameters use a finite constant
base. Rebind after a base or input-block change; no saved parameter is modified.

Oscillators, LFOs, constant sources, low/high-pass, gain, pan and math processors
use these views. The legacy `modulated_value` and `ParameterModulation::value`
APIs remain available. There is no automatic conversion of audio-rate FM or gate
streams to block averages, and no approximation of filter coefficients.

## Benchmarks and checks

`cargo bench --bench graph` runs the existing simple/complex graph, six-instrument
session and gain/pan kernel comparisons, plus these patch compiler workloads:

- `patch_static_256`: a continuous source/filter/pan/gain/output chain.
- `patch_cv_256`: sample FM/AM, cutoff, pan and gain modulation on that chain.
- `patch_cv_32`: the same routing in 32-frame spans, as event-heavy hosts produce.
- `patch_cv_8_voices`: eight independent modulated graphs, representing host voices.

Preparation is outside measured iterations. Every measured iteration renders and
consumes its output. CI copies the same PR benchmark harness onto the base commit,
so newly introduced workloads have a comparable baseline. The harness must remain
compatible with the merge base's public API. The default regression threshold and
confidence-interval check remain unchanged; historical per-benchmark waivers have
been removed. Allocation tests separately count allocations on the rendering
thread after warm-up; audio-reference and irregular-span tests check behavior.

Run native and portable tests, strict all-target Clippy, wasm compilation, package
verification and the benchmark gate before release. Use optimized builds for
real-time capacity measurements. Average benchmark throughput does not establish
callback deadline safety: hosts should additionally measure callback deadlines,
worst/p99 time, voice tails, monitor overhead and clipping. Polyphony allocation,
monitor history, sequencing and device buffering remain host responsibilities.

Future work includes a prepared buffer arena with caller-owned source/filter
outputs, deferred destruction of replaced graphs, and host-tail-aware voice
retirement. Those require separate API/behavior design and dedicated benchmarks.
