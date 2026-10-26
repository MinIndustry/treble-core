use treble::patch::{ModuleRegistry, PatchSession, parse_document};
#[test]
fn prepared_filter_handles_are_resolved_by_module_identity() {
    let document = parse_document(include_str!("../examples/fixtures/rack.trbl")).unwrap();
    let registry = ModuleRegistry::builtins();
    let patch = registry.prepare(&document, 48000.0, 128).unwrap();
    let session = PatchSession::new(registry, document.clone()).unwrap();
    let edit = session.snapshot().prepare(48000.0, 128).unwrap();
    for module in &document.instrument.patch.modules {
        let handle = patch.filter_index(&module.id);
        assert_eq!(handle, edit.filter_index(&module.id));
        if module.type_id == "treble.oscillator" {
            assert!(handle.is_none());
        } else if matches!(
            module.type_id.as_str(),
            "treble.low_pass" | "treble.output" | "treble.delay"
        ) {
            assert!(handle.is_some());
        }
    }
    assert!(patch.filter_index("missing").is_none());
}
