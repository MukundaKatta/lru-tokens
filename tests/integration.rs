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
    // `huge` is larger than capacity; the eviction loop won't drop the last
    // entry, so we end with just "huge".
    c.put("huge", (), 5000);
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

#[test]
fn capacity_is_reported() {
    let c: LruTokens<&str, ()> = LruTokens::new(4242);
    assert_eq!(c.capacity(), 4242);
}

#[test]
fn weight_saturates_instead_of_overflowing() {
    // Cumulative weight must never panic or wrap when it would exceed u64::MAX.
    let mut c: LruTokens<u64, ()> = LruTokens::new(u64::MAX);
    c.put(1, (), u64::MAX);
    c.put(2, (), 10); // u64::MAX + 10 would overflow without saturation
    assert_eq!(c.weight(), u64::MAX);
}

#[test]
fn put_existing_key_bumps_recency() {
    // Re-inserting an existing key must refresh its recency so it survives
    // a later eviction that an older, untouched key does not.
    let mut c: LruTokens<&str, i32> = LruTokens::new(100);
    c.put("a", 1, 40);
    c.put("b", 2, 40);
    c.put("a", 11, 40); // update "a"; "b" is now the LRU
    c.put("c", 3, 40); // 40 + 40 + 40 = 120 > 100; evict LRU ("b")
    assert_eq!(
        c.peek(&"a"),
        Some(&11),
        "updated 'a' should survive and hold new value"
    );
    assert!(c.peek(&"b").is_none(), "'b' should be evicted as the LRU");
    assert!(c.peek(&"c").is_some(), "'c' should be present");
}

#[test]
fn zero_capacity_keeps_only_newest() {
    // With capacity 0 every entry is oversize; the eviction loop never drops
    // the last remaining entry, so the cache holds exactly the most recent one.
    let mut c: LruTokens<&str, ()> = LruTokens::new(0);
    c.put("a", (), 5);
    assert_eq!(c.len(), 1);
    assert!(c.peek(&"a").is_some());

    c.put("b", (), 5);
    assert_eq!(c.len(), 1, "inserting 'b' should evict 'a'");
    assert!(c.peek(&"a").is_none());
    assert!(c.peek(&"b").is_some());
}

#[test]
fn zero_weight_entries_never_evicted() {
    // Weightless entries never push cumulative weight over a positive capacity,
    // so none of them are evicted regardless of count.
    let mut c: LruTokens<i32, ()> = LruTokens::new(10);
    for i in 0..1000 {
        c.put(i, (), 0);
    }
    assert_eq!(c.len(), 1000);
    assert_eq!(c.weight(), 0);
}

#[test]
fn remove_then_reinsert_tracks_weight() {
    // Weight bookkeeping must stay consistent across remove + reinsert cycles.
    let mut c: LruTokens<&str, ()> = LruTokens::new(100);
    c.put("a", (), 30);
    c.put("b", (), 30);
    assert_eq!(c.weight(), 60);
    assert!(c.remove(&"a").is_some());
    assert_eq!(c.weight(), 30);
    c.put("a", (), 10);
    assert_eq!(c.weight(), 40);
    assert_eq!(c.len(), 2);
}
