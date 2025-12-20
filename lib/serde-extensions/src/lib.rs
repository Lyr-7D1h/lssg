use serde::Deserialize;
pub use serde_extensions_derive::*;
pub use serde_value;

pub trait Overwrite {
    /// Overwrite existing fields in a struct
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>;
}

// Blanket implementations for common types that should just be replaced, not merged
macro_rules! impl_overwrite_replace {
    ($($t:ty),*) => {
        $(
            impl Overwrite for $t {
                fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    *self = Deserialize::deserialize(d)?;
                    Ok(())
                }
            }
        )*
    };
}

// Implement Overwrite for common primitive and standard library types
impl_overwrite_replace!(
    bool, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, String, char, usize, isize
);

// Implement for Option<T> where T: Overwrite
impl<T> Overwrite for Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        *self = Deserialize::deserialize(d)?;
        Ok(())
    }
}

// Implement for Vec<T> where T: Overwrite
impl<T> Overwrite for Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        *self = Deserialize::deserialize(d)?;
        Ok(())
    }
}

// Implement for types from common crates
impl Overwrite for std::path::PathBuf {
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        *self = Deserialize::deserialize(d)?;
        Ok(())
    }
}

impl<K, V> Overwrite for std::collections::HashMap<K, V>
where
    K: for<'de> Deserialize<'de> + std::hash::Hash + Eq,
    V: for<'de> Deserialize<'de>,
{
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        *self = Deserialize::deserialize(d)?;
        Ok(())
    }
}
