pub use serde_extensions_derive::*;

pub trait Overwrite {
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>;
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::Overwrite;
    use toml::Value;

    #[derive(Debug, Overwrite)]
    struct Test {
        a: PathBuf,
        b: String,
        c: Vec<String>,
        d: u32,
        e: HashMap<String, String>,
    }

    const TEST_TOML: &'static str = r#"
a = "/test/asdf"
c = ["c", "c"]
[e]
test="asdf"
"#;

    #[test]
    pub fn overwrite_works() {
        let mut e = HashMap::new();
        e.insert("test", "fdsa");
        let mut test = Test {
            a: PathBuf::default(),
            b: "b".into(),
            c: vec!["asdf".into()],
            d: 0,
            e: HashMap::new(),
        };
        let value: Value = toml::from_str(TEST_TOML).unwrap();
        test.overwrite(value).unwrap();

        assert_eq!(
            format!("{test:?}"),
            r#"Test { a: "/test/asdf", b: "b", c: ["c", "c"], d: 0, e: {"test": "asdf"} }"#
        )
    }
}
