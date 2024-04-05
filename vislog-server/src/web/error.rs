#[macro_export]
macro_rules! impl_generic_error_and_display_for_error_type {
    ($name:ident) => {
        impl std::error::Error for $name {}

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{self:?}")
            }
        }
    };
}
