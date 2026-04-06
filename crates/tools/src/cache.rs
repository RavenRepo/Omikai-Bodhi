//! Tool result caching for performance optimization.
//!
//! Provides an LRU cache for read-only tool results with TTL-based expiration.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::ToolResult;

/// Configuration for which tools can be cached and their TTLs.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// TTL settings per tool
    pub tool_ttls: HashMap<String, Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let mut tool_ttls = HashMap::new();
        // Cacheable read-only tools with their TTLs
        tool_ttls.insert("glob".to_string(), Duration::from_secs(30));
        tool_ttls.insert("grep".to_string(), Duration::from_secs(30));
        tool_ttls.insert("file_read".to_string(), Duration::from_secs(10));

        Self {
            max_entries: 100,
            tool_ttls,
        }
    }
}

impl CacheConfig {
    /// Check if a tool is cacheable.
    pub fn is_cacheable(&self, tool_name: &str) -> bool {
        self.tool_ttls.contains_key(tool_name)
    }

    /// Get the TTL for a tool.
    pub fn get_ttl(&self, tool_name: &str) -> Option<Duration> {
        self.tool_ttls.get(tool_name).copied()
    }
}

/// A cache key combining tool name and input.
#[derive(Debug, Clone, Eq)]
pub struct CacheKey {
    pub tool_name: String,
    pub input_hash: u64,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.tool_name == other.tool_name && self.input_hash == other.input_hash
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tool_name.hash(state);
        self.input_hash.hash(state);
    }
}

impl CacheKey {
    /// Create a new cache key from tool name and input.
    pub fn new(tool_name: &str, input: &serde_json::Value) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        input.to_string().hash(&mut hasher);

        Self {
            tool_name: tool_name.to_string(),
            input_hash: hasher.finish(),
        }
    }
}

/// A cached tool result with expiration time.
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub result: ToolResult,
    pub cached_at: Instant,
    pub expires_at: Instant,
}

impl CachedResult {
    /// Create a new cached result.
    pub fn new(result: ToolResult, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            result,
            cached_at: now,
            expires_at: now + ttl,
        }
    }

    /// Check if the cached result has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    /// Get the age of the cached result.
    pub fn age(&self) -> Duration {
        Instant::now().duration_since(self.cached_at)
    }
}

/// Thread-safe LRU cache for tool results.
pub struct ToolCache {
    config: CacheConfig,
    cache: RwLock<HashMap<CacheKey, CachedResult>>,
    /// Tracks access order for LRU eviction
    access_order: RwLock<Vec<CacheKey>>,
    /// Statistics
    hits: RwLock<u64>,
    misses: RwLock<u64>,
}

impl ToolCache {
    /// Create a new cache with default configuration.
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache with custom configuration.
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            config,
            cache: RwLock::new(HashMap::new()),
            access_order: RwLock::new(Vec::new()),
            hits: RwLock::new(0),
            misses: RwLock::new(0),
        }
    }

    /// Check if a tool's results can be cached.
    pub fn is_cacheable(&self, tool_name: &str) -> bool {
        self.config.is_cacheable(tool_name)
    }

    /// Get a cached result if available and not expired.
    pub fn get(&self, tool_name: &str, input: &serde_json::Value) -> Option<ToolResult> {
        if !self.is_cacheable(tool_name) {
            return None;
        }

        let key = CacheKey::new(tool_name, input);

        let cache = self.cache.read().ok()?;
        if let Some(cached) = cache.get(&key) {
            if !cached.is_expired() {
                // Update access order
                if let Ok(mut order) = self.access_order.write() {
                    order.retain(|k| k != &key);
                    order.push(key);
                }

                // Update stats
                if let Ok(mut hits) = self.hits.write() {
                    *hits += 1;
                }

                return Some(cached.result.clone());
            }
        }

        // Update stats
        if let Ok(mut misses) = self.misses.write() {
            *misses += 1;
        }

        None
    }

    /// Store a result in the cache.
    pub fn put(&self, tool_name: &str, input: &serde_json::Value, result: ToolResult) {
        let Some(ttl) = self.config.get_ttl(tool_name) else {
            return; // Not cacheable
        };

        let key = CacheKey::new(tool_name, input);

        // Evict if at capacity
        self.evict_if_needed();

        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key.clone(), CachedResult::new(result, ttl));
        }

        if let Ok(mut order) = self.access_order.write() {
            order.retain(|k| k != &key);
            order.push(key);
        }
    }

    /// Evict least recently used entries if at capacity.
    fn evict_if_needed(&self) {
        let Ok(cache) = self.cache.read() else {
            return;
        };

        if cache.len() < self.config.max_entries {
            return;
        }
        drop(cache);

        // Remove expired entries first
        self.remove_expired();

        // If still at capacity, evict LRU
        let Ok(cache) = self.cache.read() else {
            return;
        };

        if cache.len() >= self.config.max_entries {
            drop(cache);

            if let Ok(mut order) = self.access_order.write() {
                if let Some(key) = order.first().cloned() {
                    order.remove(0);
                    if let Ok(mut cache) = self.cache.write() {
                        cache.remove(&key);
                    }
                }
            }
        }
    }

    /// Remove all expired entries.
    pub fn remove_expired(&self) {
        let Ok(mut cache) = self.cache.write() else {
            return;
        };

        let expired_keys: Vec<CacheKey> = cache
            .iter()
            .filter(|(_, v)| v.is_expired())
            .map(|(k, _)| k.clone())
            .collect();

        for key in &expired_keys {
            cache.remove(key);
        }

        if let Ok(mut order) = self.access_order.write() {
            order.retain(|k| !expired_keys.contains(k));
        }
    }

    /// Invalidate cache entries for a specific tool.
    pub fn invalidate_tool(&self, tool_name: &str) {
        let Ok(mut cache) = self.cache.write() else {
            return;
        };

        let keys_to_remove: Vec<CacheKey> = cache
            .keys()
            .filter(|k| k.tool_name == tool_name)
            .cloned()
            .collect();

        for key in &keys_to_remove {
            cache.remove(key);
        }

        if let Ok(mut order) = self.access_order.write() {
            order.retain(|k| !keys_to_remove.contains(k));
        }
    }

    /// Invalidate all cache entries (e.g., after file write).
    pub fn invalidate_all(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        if let Ok(mut order) = self.access_order.write() {
            order.clear();
        }
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.read().map(|h| *h).unwrap_or(0);
        let misses = self.misses.read().map(|m| *m).unwrap_or(0);
        let size = self.cache.read().map(|c| c.len()).unwrap_or(0);

        CacheStats {
            hits,
            misses,
            size,
            hit_rate: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Clear the cache and reset stats.
    pub fn clear(&self) {
        self.invalidate_all();
        if let Ok(mut hits) = self.hits.write() {
            *hits = 0;
        }
        if let Ok(mut misses) = self.misses.write() {
            *misses = 0;
        }
    }
}

impl Default for ToolCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_equality() {
        let input = serde_json::json!({"pattern": "*.rs"});
        let key1 = CacheKey::new("glob", &input);
        let key2 = CacheKey::new("glob", &input);
        assert_eq!(key1, key2);

        let key3 = CacheKey::new("grep", &input);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_put_get() {
        let cache = ToolCache::new();
        let input = serde_json::json!({"pattern": "*.rs"});
        let result = ToolResult::success("file1.rs\nfile2.rs");

        cache.put("glob", &input, result.clone());

        let cached = cache.get("glob", &input);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().output, result.output);
    }

    #[test]
    fn test_non_cacheable_tool() {
        let cache = ToolCache::new();
        let input = serde_json::json!({"command": "ls"});
        let result = ToolResult::success("output");

        cache.put("bash", &input, result);

        let cached = cache.get("bash", &input);
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ToolCache::new();
        let input = serde_json::json!({"pattern": "*.rs"});
        let result = ToolResult::success("output");

        cache.put("glob", &input, result);

        // First get - hit
        cache.get("glob", &input);
        // Second get - hit
        cache.get("glob", &input);
        // Miss - different input
        cache.get("glob", &serde_json::json!({"pattern": "*.py"}));

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);
    }

    #[test]
    fn test_invalidate_tool() {
        let cache = ToolCache::new();
        let input1 = serde_json::json!({"pattern": "*.rs"});
        let input2 = serde_json::json!({"pattern": "*.py"});

        cache.put("glob", &input1, ToolResult::success("1"));
        cache.put("glob", &input2, ToolResult::success("2"));

        cache.invalidate_tool("glob");

        assert!(cache.get("glob", &input1).is_none());
        assert!(cache.get("glob", &input2).is_none());
    }

    #[test]
    fn test_invalidate_all() {
        let cache = ToolCache::new();
        let input = serde_json::json!({"pattern": "*.rs"});

        cache.put("glob", &input, ToolResult::success("1"));
        cache.put("grep", &input, ToolResult::success("2"));

        cache.invalidate_all();

        assert!(cache.get("glob", &input).is_none());
        assert!(cache.get("grep", &input).is_none());
    }
}
