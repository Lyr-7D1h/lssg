use serde::Deserialize;
pub use serde_extensions_derive::*;
pub use serde_value;

/// A trait for selectively overwriting fields in a struct from a deserializer.
///
/// The `Overwrite` trait provides a mechanism to update existing values in a struct
/// by deserializing only the fields present in the input data, leaving other fields
/// unchanged. This is particularly useful for configuration merging, partial updates,
/// and layered settings where you want to apply defaults first and then override
/// specific values.
///
/// # Behavior
///
/// - For primitive types and most standard library types, `overwrite` completely
///   replaces the existing value with the deserialized value.
/// - For structs that derive `Overwrite` (using `#[derive(Overwrite)]`), only the
///   fields present in the input are updated, while absent fields retain their
///   current values.
///
/// # Examples
///
/// ```rust
/// use serde::Deserialize;
/// use serde_extensions::Overwrite;
///
/// #[derive(Deserialize, Overwrite)]
/// struct Config {
///     host: String,
///     port: u16,
///     debug: bool,
/// }
///
/// let mut config = Config {
///     host: "localhost".to_string(),
///     port: 8080,
///     debug: false,
/// };
///
/// // Overwrite only the port, leaving host and debug unchanged
/// let partial = r#"{"port": 3000}"#;
/// config.overwrite(&mut serde_json::Deserializer::from_str(partial)).unwrap();
///
/// assert_eq!(config.host, "localhost");
/// assert_eq!(config.port, 3000);
/// assert_eq!(config.debug, false);
/// ```
///
/// # Implementing for Custom Types
///
/// For structs, you can derive `Overwrite` automatically:
///
/// ```rust
/// use serde_extensions::Overwrite;
///
/// #[derive(Overwrite)]
/// struct MyStruct {
///     field1: String,
///     field2: i32,
/// }
/// ```
///
/// For other types, implement the trait manually. Types that should be completely
/// replaced (rather than merged) should deserialize a new value and assign it:
///
/// ```rust
/// use serde::{Deserialize, Deserializer};
/// use serde_extensions::Overwrite;
///
/// struct CustomType(i32);
///
/// impl Overwrite for CustomType {
///     fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
///     where
///         D: Deserializer<'de>,
///     {
///         *self = CustomType::deserialize(d)?;
///         Ok(())
///     }
/// }
/// ```
pub trait Overwrite {
    /// Overwrites the current value with data from a deserializer.
    ///
    /// For primitive types and collections, this completely replaces the value.
    /// For structs with derived `Overwrite`, only fields present in the deserializer
    /// are updated.
    ///
    /// # Arguments
    ///
    /// * `d` - A deserializer containing the data to overwrite with
    ///
    /// # Errors
    ///
    /// Returns a deserialization error if the input data is invalid for the type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use serde_extensions::Overwrite;
    ///
    /// let mut value = 42;
    /// value.overwrite(&mut serde_json::Deserializer::from_str("100")).unwrap();
    /// assert_eq!(value, 100);
    /// ```
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
