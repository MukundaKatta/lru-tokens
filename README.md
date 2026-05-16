# lru-tokens

[![crates.io](https://img.shields.io/crates/v/lru-tokens.svg)](https://crates.io/crates/lru-tokens)
[![docs.rs](https://img.shields.io/docsrs/lru-tokens)](https://docs.rs/lru-tokens)

LRU cache where eviction is **weighted by token count** (or any other
size unit), not entry count. Zero deps.

## Why

A prompt cache holding a few 100k-token system prompts and thousands of
small ones doesn't fit in a `LruCache<K, V>` bounded by entry count —
the large entries dominate memory but cost only one slot. This cache
inverts the policy: each entry carries a `weight`, and eviction drops
the least-recently-used entries until the cumulative weight fits.

## Usage

```rust
use lru_tokens::LruTokens;

let mut cache: LruTokens<&str, String> = LruTokens::new(100_000); // 100k tokens

cache.put("system-prompt-v1", "...".into(), 80_000);
cache.put("system-prompt-v2", "...".into(), 30_000); // total 110k > 100k
// v1 (LRU) is evicted to make room.
assert!(cache.get(&"system-prompt-v1").is_none());
assert_eq!(cache.weight(), 30_000);
```

The unit of `weight` is whatever you want — tokens, bytes, or dollars
(scaled to ints).

## License

MIT or Apache-2.0.
