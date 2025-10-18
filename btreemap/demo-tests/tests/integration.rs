use std::collections::BTreeMap;

// Імпортуємо обидва макроси з різними псевдонімами,
// щоб чітко перевірити обидві реалізації
use btreemap_macro_rules::btreemap as btreemap_rules;
use btreemap_proc::btreemap as btreemap_proc;

#[test]
fn rules_empty_and_sorted() {
    let m: BTreeMap::<i32, &str> = btreemap_rules!();
    assert!(m.is_empty());

    let m = btreemap_rules! {
        3 => "c",
        1 => "a",
        2 => "b",
    };
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), vec![1,2,3]);
}

#[test]
fn rules_overwrite_and_exprs() {
    let m = btreemap_rules! {
        "x".to_string() => 10,
        "x".to_string() => 20,
        format!("y{}", 1) => 30 - 5,
    };
    assert_eq!(m.get("x"), Some(&20));
    assert_eq!(m.get("y1"), Some(&25));
}

#[test]
fn proc_empty_and_sorted() {
    let m: BTreeMap::<i32, i32> = btreemap_proc!();
    assert!(m.is_empty());

    let m = btreemap_proc! {
        9 => 0,
        1 => 0,
        5 => 0,
    };
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), vec![1,5,9]);
}

#[test]
fn proc_non_copy_values_and_trailing_comma() {
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    struct K(i32);

    let m = btreemap_proc! {
        K(2) => String::from("b"),
        K(1) => String::from("a"),
    };
    assert_eq!(m.get(&K(1)).unwrap(), "a");
    assert_eq!(m.get(&K(2)).unwrap(), "b");
}

#[test]
fn parity_between_impls() {
    let a = btreemap_rules! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
    };
    let b = btreemap_proc! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
    };
    assert_eq!(a, b);
}
