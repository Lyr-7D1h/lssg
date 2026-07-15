#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::*;

    fn get_hash<T: Hash>(t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_input_variants_are_unique_in_hashmap() {
        let local = Input::Local {
            path: PathBuf::from("/some/path/to/file.md"),
        };
        let external = Input::External {
            url: Url::parse("https://example.com/some/path/to/file.md").unwrap(),
        };

        // Same logical path should still produce distinct enum variants
        assert_ne!(local, external);
        assert_ne!(get_hash(&local), get_hash(&external));

        // Both should coexist as separate keys in a HashMap
        let mut map = HashMap::new();
        map.insert(local.clone(), "local_value");
        map.insert(external.clone(), "external_value");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&local), Some(&"local_value"));
        assert_eq!(map.get(&external), Some(&"external_value"));
    }
}
