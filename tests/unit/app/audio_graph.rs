use treble::{
    app::audio_graph::AudioGraph,
    instruments::prelude::{Kick, Snare},
};

#[test]
pub fn test_audio_graph_compilation_isnt_destructive() {
    let mut audio_graph = AudioGraph::new();
    audio_graph.add_instrument(Box::from(Kick::new()));
    let maybe_compiled_graph = audio_graph.compile(44100.0);
    assert!(
        maybe_compiled_graph.is_ok(),
        "Audio graph compilation should be working fine"
    );
    let compiled_graph = maybe_compiled_graph.unwrap();
    assert_eq!(
        compiled_graph.sources_len(),
        1,
        "There should be a single source in the compiled graph with a single instrument"
    );
    assert_eq!(
        audio_graph.source_map.iter().len(),
        1,
        "There should be a single entry in the source map of the audio graph"
    );

    audio_graph.add_instrument(Box::from(Snare::new()));
    let maybe_compiled_graph = audio_graph.compile(44100.0);
    assert!(maybe_compiled_graph.is_ok());
    let compiled_graph = maybe_compiled_graph.unwrap();
    assert_eq!(
        compiled_graph.sources_len(),
        2,
        "There should be two sources for a graph with two instruments."
    );
    assert_eq!(
        audio_graph.source_map.iter().len(),
        2,
        "There should be two distinct entries in the source map of the audio graph"
    );
}
