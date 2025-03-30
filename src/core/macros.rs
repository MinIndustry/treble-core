#[macro_export]
macro_rules! note {
    ($( $x:expr )?) => {{
        $( crate::core::keys::Key {
            code: $x,
            ktype: crate::core::keys::KeyType::Note,
            sustain: true,
        } )?
    }};
}
