pub mod structs;
pub mod traits;

use structs::MetaFilter;

#[macro_export]
macro_rules! filters {
    ( $( $x:ident ),* ) => {
        {
            let mut temp_vec = Vec::new();
            $(
                temp_vec.push(Box::new($x::default()) as Box<dyn Filter>);
            )*
            temp_vec
        }
    };
}

pub fn get_filters() -> Vec<MetaFilter> {
    vec![
        crate::core::filters::prelude::GainFilter_META().clone(),
        crate::core::filters::prelude::Clipper_META().clone(),
        crate::core::filters::prelude::CombinatorFilter_META().clone(),
    ]
}
