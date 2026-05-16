use lru_tokens::LruTokens;

#[test]
fn empty_cache_is_zero() {
    let c: LruTokens<&str, ()> = LruTokens::new(100);
    assert_eq!(c.weight(), 0);
    assert!(c.is_empty());
}

#[test]
fn put_and_get_round_trip() {
    let mut c: LruTokens<&str, String> = LruTokens::new(1000);
    c.put("a", "hello".into(), 50);
    assert_eq!(c.weight(), 50);
    assert_eq!(c.get(&"a").map(|s| s.as_str()), Some("hello"));
}

#[test]
fn eviction_drops_lru() {
    let mut c: LruTokens<&str, ()> = LruTokens::new(100);
    c.put("a", (), 60);
    c.put("b", (), 50);
    // 60 + 50 = 110 > 100; LRU is "a" (older); should be evicted.
    assert_eq!(c.weight(), 50);
    assert!(c.peek(&"a").is_none());
    assert!(c.peek(&"b").is_some());
}

#[test]
fn get_bumps_recency() {
    let mut c: LruTokens<&str, ()> = LruTokens::new(100);
    c.put("a", (), 40);
    c.put("b", (), 40);
    let _ = c.get(&"a"); // bump a's recency; b is now LRU
    c.put("c", (), 40); // 40+40+40=120 > 100; evict LRU (b)
    assert!(c.peek(&"a").is_some(), "a should survive");
    assert!(c.peek(&"b").is_none(), "b should be evicted");
    assert!(c.peek(&"c").is_some(), "c should be present");
}

#[test]
fn peek_does_not_bump_recency() {
    let mut c: LruTokens<&str, ()> = LruTokens::new(100);
    c.put("a", (), 40);
    c.put("b", (), 40);
    let _ = c.peek(&"a"); // does NOT bump
    c.put("c", (), 40);
    // a is still LRU; should be evicted.
    assert!(c.peek(&"a").is_none());
    assert!(c.peek(&"b").is_some());
}

#[test]
fn oversize_entry_kept_alone() {
    let mut c: LruTokens<&str, ()> = LruTokens::new(100);
    c.put("a", (), 50);
    c.put("huge", (), 5000); // larger than capacity
    // Eviction loop won't drop the last entry, so we end with just "huge".
    assert_eq!(c.len(), 1);
    assert!(c.peek(&"huge").is_some());
    assert_eq!(c.weight(), 5000);
}

#[test]
fn replace_same_key_updates_weight() {
    let mut c: LruTokens<&str, &'static str> = LruTokens::new(100);
    c.put("a", "first", 30);
    c.put("a", "second", 70);
    assert_eq!(c.weight(), 70);
    assert_eq!(c.peek(&"a"), Some(&"second"));
}

#[test]
fn remove_frees_weight() {
    let mut c: LruTokens<&str, ()> = LruTokens::new(100);
    c.put("a", (), 30);
    c.put("b", (), 30);
    assert!(c.remove(&"a").is_some());
    assert_eq!(c.weight(), 30);
    assert!(c.remove(&"missing").is_none());
}

#[test]
fn clear_resets() {
    let mut c: LruTokens<&str, ()> = LruTokens::new(100);
    c.put("a", (), 30);
    c.clear();
    assert!(c.is_empty());
    assert_eq!(c.weight(), 0);
}
