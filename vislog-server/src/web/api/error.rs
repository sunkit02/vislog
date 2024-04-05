use crate::impl_generic_error_and_display_for_error_type;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {}

impl_generic_error_and_display_for_error_type!(Error);
