//! Declarative `btreemap!` macro (macro_rules!)

#[macro_export]
macro_rules! btreemap {
    // порожня мапа
    () => {{
        ::std::collections::BTreeMap::new()
    }};
    ( $( $k:expr => $v:expr ),+ $(,)? ) => {{
        let mut __map = ::std::collections::BTreeMap::new();
        $(
            // insert повертає Option старого значення — ігноруємо (останнє перемагає)
            __map.insert($k, $v);
        )+
        __map
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn empty() {
        let m: BTreeMap::<u8, u8> = btreemap!();
        assert!(m.is_empty());
    }

    #[test]
    fn basic_and_trailing() {
        let m = btreemap! {
            3 => "c",
            1 => "a",
            2 => "b",
        };
        assert_eq!(m.len(), 3);
        let keys: Vec<_> = m.keys().copied().collect();
        assert_eq!(keys, vec![1, 2, 3]); // BTreeMap відсортований за ключем
    }

    #[test]
    fn expressions_and_overwrite() {
        let m = btreemap! {
            "he".to_string() + "llo" => 1 + 1,
            "hello".to_string() => 3,
        };
        assert_eq!(m.get("hello"), Some(&3)); // останнє значення перезаписує
    }

    #[test]
    fn non_copy_values() {
        #[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
        struct K(i32);
        let v1 = String::from("x");
        let v2 = String::from("y");
        let m = btreemap! {
            K(2) => v2,
            K(1) => v1,
        };
        assert_eq!(m.get(&K(1)).unwrap(), "x");
        assert_eq!(m.get(&K(2)).unwrap(), "y");
    }
}
