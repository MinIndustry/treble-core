//! Count allocations on the calling render thread, excluding preparation and warm-up.
use std::{
    alloc::{GlobalAlloc, Layout, System as Allocator},
    cell::Cell,
    sync::Arc,
};
use treble::core::{
    Block,
    filters::prelude::GainFilter,
    graph::{AudioOutputSink, Entry, SimpleSink, Sink, Source, System},
};

struct CountingAllocator;
thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
fn allocated() {
    if TRACKING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocated();
        unsafe { Allocator.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { Allocator.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocated();
        unsafe { Allocator.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn allocations(action: impl FnOnce()) -> usize {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TRACKING.set(false);
        }
    }
    ALLOCATIONS.set(0);
    TRACKING.set(true);
    let reset = Reset;
    action();
    drop(reset);
    ALLOCATIONS.get()
}

#[test]
fn builtin_sink_output_reuse_has_no_allocations_after_warmup() {
    for mut sink in [
        Box::new(SimpleSink::new()) as Box<dyn Sink>,
        Box::new(AudioOutputSink::new(48000.0)),
    ] {
        let input = Arc::new(vec![[0.25, -0.5]; 256]);
        let mut output = Block::with_capacity(256);
        for _ in 0..2 {
            sink.push(input.clone(), 0);
            sink.consume_into(&mut output);
        }
        let count = allocations(|| {
            for frames in [256, 32, 1, 128, 256] {
                // The larger input intentionally tests that consume_into replaces,
                // rather than appends to, output left by the previous cycle.
                sink.push(input.clone(), 0);
                sink.consume_into(&mut output);
                assert_eq!(output.len(), 256);
                assert_eq!(output[frames - 1], [0.25, -0.5]);
            }
        });
        assert_eq!(count, 0);
        sink.consume_into(&mut output);
        assert!(output.is_empty());
    }
}

#[derive(Debug, Clone)]
struct Constant;
impl Source for Constant {
    fn pull(&mut self, frames: usize) -> Block {
        vec![[0.25; 2]; frames]
    }
}
#[test]
fn graph_queues_do_not_add_per_block_allocations_after_warmup() {
    let mut system = System::new();
    let source = system.add_source(Box::new(Constant));
    let gain = system.add_filter(Box::new(GainFilter::new(0.5)));
    let sink = system.add_sink(Box::new(SimpleSink::new()));
    system.connect_source(source, gain, 0);
    system.connect_sink(gain, sink, 0);
    system.compute().unwrap();
    let mut output = Vec::with_capacity(256);
    for _ in 0..3 {
        system.run_frames(256);
        system.get_sink(0).unwrap().consume_into(&mut output);
    }
    let count = allocations(|| {
        for _ in 0..100 {
            system.run_frames(256);
            system.get_sink(0).unwrap().consume_into(&mut output);
        }
    });
    // Existing source/filter Block and Arc results still allocate. Input queues,
    // the source routing scratch vector and sink output must not add to those.
    assert!(count <= 600, "unexpected routing allocations: {count}");
    assert_eq!(output, vec![[0.125; 2]; 256]);
}

#[test]
fn reused_output_matches_allocating_sink_and_telemetry_across_block_sizes() {
    let mut old = AudioOutputSink::new(48000.0);
    let mut reuse = old.clone();
    let mut output = Vec::new();
    for frames in [256, 1, 32, 512, 0, 17] {
        let input: Arc<Block> = Arc::new(
            (0..frames)
                .map(|i| [if i % 3 == 0 { 2.0 } else { 0.2 }, -1.5])
                .collect(),
        );
        old.push(input.clone(), 0);
        reuse.push(input, 0);
        reuse.consume_into(&mut output);
        assert_eq!(output, old.consume());
        assert_eq!(reuse.telemetry(), old.telemetry());
    }
}
