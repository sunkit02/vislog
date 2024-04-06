use std::{collections::HashMap, hash::Hash};

pub mod courses;
pub mod json_providers;
pub mod programs;

struct ProviderCache<K, T, E>
where
    K: Hash,
    E: std::error::Error,
{
    items: HashMap<K, T>,
    errors: Vec<E>,
}
