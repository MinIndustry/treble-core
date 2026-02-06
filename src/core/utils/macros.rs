//! Macros are used to define common patterns and structures within the application.
//! They can be used to simplify code and improve readability.

#[macro_export]
macro_rules! note {
    ($( $x:expr )?) => {{
        $( $crate::core::keys::Key {
            code: $x,
            ktype: $crate::core::keys::KeyType::Note,
            sustain: true,
        } )?
    }};
}
