//! # lru-tokens
//!
//! LRU cache where eviction is **weighted by token count** (or any other
//! size unit you supply), not entry count.
//!
//! Prompt caches are usually bounded by tokens, not by entries — a few
//! 100k-token system prompts can dominate the budget that would
//! otherwise hold thousands of small entries. This crate inverts the
//! usual `LruCache<K, V>` policy: each entry carries a `weight` (you
//! say what it means — tokens, bytes, dollars), and the cache evicts
//! the least-recently-used entries until the cumulative weight fits.
//!
//! ## Example
//!
//! ```
//! use lru_tokens::LruTokens;
//!
//! let mut cache: LruTokens<&str, String> = LruTokens::new(1_000);
//!
//! cache.put("system-prompt-a", "...".into(), 800);
//! cache.put("system-prompt-b", "...".into(), 300); // total 1100 > 1000
//! // Inserting `b` evicted `a` (the LRU) to bring total under 1000.
//! assert!(cache.get(&"system-prompt-a").is_none());
//! assert!(cache.get(&"system-prompt-b").is_some());
//! assert_eq!(cache.weight(), 300);
//! ```

#![deny(missing_docs)]

use std::collections::HashMap;
use std::hash::Hash;

/// A single cache entry's bookkeeping.
struct Entry<V> {
    value: V,
    weight: u64,
    /// Monotonic timestamp of last access; bigger = more recent.
    last_access: u64,
}

/// LRU cache bounded by cumulative weight.
pub struct LruTokens<K, V> {
    capacity: u64,
    weight: u64,
    tick: u64,
    map: HashMap<K, Entry<V>>,
}

impl<K, V> LruTokens<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Build a cache that holds entries totalling at most `capacity`
    /// units of weight.
    pub fn new(capacity: u64) -> Self {
        Self {
            capacity,
            weight: 0,
            tick: 0,
            map: HashMap::new(),
        }
    }

    /// Capacity in weight units.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Current cumulative weight.
    pub fn weight(&self) -> u64 {
        self.weight
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when no entries are cached.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Insert or update an entry. If the new weight pushes the total
    /// over capacity, the least-recently-used entries are dropped
    /// until the total fits (or only the new entry remains).
    ///
    /// If `weight` itself exceeds `capacity`, the cache will end up
    /// holding just this one entry with `self.weight() == weight`.
    pub fn put(&mut self, key: K, value: V, weight: u64) {
        // Replace existing entry (free up its old weight first).
        if let Some(old) = self.map.remove(&key) {
            self.weight -= old.weight;
        }
        self.tick += 1;
        self.map.insert(
            key.clone(),
            Entry {
                value,
                weight,
                last_access: self.tick,
            },
        );
        self.weight += weight;
        self.evict_until_fits();
    }

    /// Look up an entry; bumps recency on hit.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.tick += 1;
        let tick = self.tick;
        let entry = self.map.get_mut(key)?;
        entry.last_access = tick;
        Some(&entry.value)
    }

    /// Look up without bumping recency. For peek-only inspection.
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|e| &e.value)
    }

    /// Remove an entry. Returns its value if present.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let entry = self.map.remove(key)?;
        self.weight -= entry.weight;
        Some(entry.value)
    }

    /// Drop everything.
    pub fn clear(&mut self) {
        self.map.clear();
        self.weight = 0;
    }

    fn evict_until_fits(&mut self) {
        while self.weight > self.capacity && self.map.len() > 1 {
            // Find the entry with the smallest last_access (LRU).
            let lru_key = self
                .map
                .iter()
                .min_by_key(|(_, e)| e.last_access)
                .map(|(k, _)| k.clone());
            if let Some(k) = lru_key {
                if let Some(e) = self.map.remove(&k) {
                    self.weight -= e.weight;
                }
            } else {
                break;
            }
        }
    }
}
