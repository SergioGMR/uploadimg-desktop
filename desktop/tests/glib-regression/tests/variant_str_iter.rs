use glib::{prelude::*, Variant};

fn array(values: &[&str]) -> Variant {
    Variant::array_from_iter::<String>(values.iter().map(|s| s.to_string().to_variant()))
}

#[test]
fn next_returns_exact_utf8_strings() {
    let v = array(&["", "alpha", "á🐈", "omega"]);
    let mut it = v.array_iter_str().unwrap();
    assert_eq!(it.len(), 4);
    for value in ["", "alpha", "á🐈", "omega"] {
        assert_eq!(it.next(), Some(value));
    }
    assert_eq!(it.next(), None);
    assert_eq!(it.next_back(), None);
}

#[test]
fn next_back_returns_exact_utf8_strings() {
    let v = array(&["", "alpha", "á🐈", "omega"]);
    let mut it = v.array_iter_str().unwrap();
    for value in ["omega", "á🐈", "alpha", ""] {
        assert_eq!(it.next_back(), Some(value));
    }
    assert_eq!(it.next_back(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn nth_returns_exact_string_and_preserves_remaining_length() {
    let v = array(&["zero", "one", "two", "three", "four"]);
    let mut it = v.array_iter_str().unwrap();
    assert_eq!(it.nth(1), Some("one"));
    assert_eq!(it.len(), 3);
    assert_eq!(it.nth(1), Some("three"));
    assert_eq!(it.len(), 1);
    assert_eq!(it.nth(0), Some("four"));
    assert_eq!(it.nth(0), None);
}

#[test]
fn nth_back_returns_exact_string_and_preserves_remaining_length() {
    let v = array(&["zero", "one", "two", "three", "four"]);
    let mut it = v.array_iter_str().unwrap();
    assert_eq!(it.nth_back(1), Some("three"));
    assert_eq!(it.len(), 3);
    assert_eq!(it.nth_back(1), Some("one"));
    assert_eq!(it.len(), 1);
    assert_eq!(it.nth_back(0), Some("zero"));
    assert_eq!(it.nth_back(0), None);
}

#[test]
fn last_returns_last_remaining_string() {
    let v = array(&["zero", "one", "two", "three"]);
    assert_eq!(v.array_iter_str().unwrap().last(), Some("three"));
    let mut it = v.array_iter_str().unwrap();
    assert_eq!(it.next(), Some("zero"));
    assert_eq!(it.next_back(), Some("three"));
    assert_eq!(it.last(), Some("two"));
}

#[test]
fn empty_and_out_of_bounds_remain_fused() {
    let v = array(&[]);
    let mut it = v.array_iter_str().unwrap();
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);
    assert_eq!(it.next_back(), None);
    assert_eq!(it.nth(usize::MAX), None);
    assert_eq!(it.nth_back(usize::MAX), None);
    assert_eq!(it.last(), None);
    let v = array(&["zero", "one"]);
    let mut it = v.array_iter_str().unwrap();
    assert_eq!(it.nth(usize::MAX), None);
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);
    assert_eq!(it.next_back(), None);
    let mut it = v.array_iter_str().unwrap();
    assert_eq!(it.nth_back(usize::MAX), None);
    assert_eq!(it.len(), 0);
    assert_eq!(it.next(), None);
    assert_eq!(it.next_back(), None);
}

#[test]
fn mixed_iteration_matches_standard_iterator() {
    for len in 0..=33 {
        let owned: Vec<String> = (0..len).map(|i| format!("item-{i}-á🐈")).collect();
        let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
        let v = array(&refs);
        let mut actual = v.array_iter_str().unwrap();
        let mut expected = refs.iter().copied();
        for step in 0..len + 6 {
            assert_eq!(actual.len(), expected.len());
            assert_eq!(actual.size_hint(), expected.size_hint());
            let (got, want) = match step % 4 {
                0 => (actual.next(), expected.next()),
                1 => (actual.next_back(), expected.next_back()),
                2 => (actual.nth(1), expected.nth(1)),
                _ => (actual.nth_back(1), expected.nth_back(1)),
            };
            assert_eq!(got, want, "len={len}, step={step}");
        }
        assert_eq!(actual.last(), expected.last());
    }
}

// The following cases retain the assertions of upstream 0.18.5's relevant
// variant_iter.rs tests, exercising the public API without uncached dev deps.
#[test]
fn upstream_variant_iter_array() {
    let v = Variant::array_from_iter::<String>([
        "foo".to_string().to_variant(),
        "bar".to_string().to_variant(),
    ]);
    let vec: Vec<String> = v.iter().map(|i| i.get().unwrap()).collect();
    let a = vec!["foo".to_string(), "bar".to_string()];
    assert_eq!(&vec, &a);
    let vec: Vec<_> = v.array_iter_str().unwrap().collect();
    assert_eq!(&vec, &a);
}

#[test]
fn upstream_variant_str_iter_nth() {
    let v = Variant::array_from_iter::<String>([
        "0".to_string().to_variant(), "1".to_string().to_variant(),
        "2".to_string().to_variant(), "3".to_string().to_variant(),
        "4".to_string().to_variant(), "5".to_string().to_variant(),
    ]);
    let mut iter = v.array_iter_str().unwrap();
    assert_eq!(iter.len(), 6);
    assert_eq!(iter.nth(1), Some("1"));
    assert_eq!(iter.len(), 4);
    assert_eq!(iter.next(), Some("2"));
    assert_eq!(iter.nth_back(2), Some("3"));
    assert_eq!(iter.len(), 0);
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next_back(), None);
}

#[test]
fn upstream_variant_str_iter_count() {
    let v = Variant::array_from_iter::<String>([
        "0".to_string().to_variant(), "1".to_string().to_variant(),
        "2".to_string().to_variant(),
    ]);
    let iter = v.array_iter_str().unwrap();
    assert_eq!(iter.len(), 3);
    assert_eq!(iter.count(), 3);
}

#[test]
fn upstream_variant_str_iter_last() {
    let v = Variant::array_from_iter::<String>([
        "0".to_string().to_variant(), "1".to_string().to_variant(),
        "2".to_string().to_variant(),
    ]);
    let iter = v.array_iter_str().unwrap();
    assert_eq!(iter.len(), 3);
    assert_eq!(iter.last(), Some("2"));
}
