# Migration Guide: Typed Keyspaces

This guide helps you migrate from the previous version of Datacake (with hardcoded `u64` keys) to the new typed keyspaces system.

## Overview of Changes

Datacake now supports **per-keyspace typed keys**, allowing each keyspace to use semantically meaningful key types like `String`, `u64`, `Uuid`, or composite types. This is a **breaking change** that affects the public API.

### What Changed

- **Key Type**: Changed from `type Key = u64` to `type Key = Vec<u8>`
- **Storage Trait**: All methods now use `Vec<u8>` or `&[u8]` for keys
- **New API**: Added `TypedKeyspaceHandle<K>` for type-safe keyspace access
- **Storage Backends**: LMDB and SQLite schemas updated to use byte-based keys

### Backward Compatibility

The good news: The underlying serialization format remains compatible. Since `Key` is now `Vec<u8>`, existing `u64` keys can be converted using `.to_le_bytes().to_vec()`.

## Migration Strategies

You have two options for migration:

### Option 1: Minimal Changes (Quick Fix)

Keep using `u64` keys with a helper function. This requires minimal code changes.

**Before:**
```rust
let handle = store.handle();

handle.put(
    "my-keyspace",
    1,  // u64 key
    b"data".to_vec(),
    Consistency::All,
).await?;

let doc = handle.get("my-keyspace", 1).await?;
```

**After:**
```rust
let handle = store.handle();

// Add a helper function
let key = |n: u64| n.to_le_bytes().to_vec();

handle.put(
    "my-keyspace",
    key(1),  // Convert u64 to Vec<u8>
    b"data".to_vec(),
    Consistency::All,
).await?;

let doc = handle.get("my-keyspace", key(1)).await?;
```

### Option 2: Adopt Typed API (Recommended)

Use the new `TypedKeyspaceHandle` for better ergonomics and type safety.

**Before:**
```rust
let handle = store.handle();

handle.put(
    "users",
    1,
    b"Alice".to_vec(),
    Consistency::All,
).await?;
```

**After:**
```rust
// Option A: Continue using u64 with typed API
let users = store.typed_handle::<u64>("users")?;
users.put(1, b"Alice".to_vec(), Consistency::All).await?;

// Option B: Switch to semantic String keys (breaking change for your app)
let users = store.typed_handle::<String>("users")?;
users.put("user_1".to_string(), b"Alice".to_vec(), Consistency::All).await?;
```

## API Changes by Component

### ReplicatedStoreHandle

**Old API:**
```rust
pub async fn get(&self, keyspace: &str, doc_id: u64) -> Result<Option<Document>, S::Error>
pub async fn put(&self, keyspace: &str, doc_id: u64, data: Vec<u8>, ...) -> Result<(), StoreError<S::Error>>
pub async fn del(&self, keyspace: &str, doc_id: u64, ...) -> Result<(), StoreError<S::Error>>
```

**New API:**
```rust
pub async fn get(&self, keyspace: &str, doc_id: Vec<u8>) -> Result<Option<Document>, S::Error>
pub async fn put(&self, keyspace: &str, doc_id: Vec<u8>, data: Vec<u8>, ...) -> Result<(), StoreError<S::Error>>
pub async fn del(&self, keyspace: &str, doc_id: Vec<u8>, ...) -> Result<(), StoreError<S::Error>>

// New typed API
pub fn typed_keyspace<K: DatacakeKey>(&self, keyspace: impl Into<String>) -> Result<TypedKeyspaceHandle<K, S>, TypeMismatchError>
```

### ReplicatedKeyspaceHandle

**Old API:**
```rust
pub async fn get(&self, doc_id: u64) -> Result<Option<Document>, S::Error>
pub async fn put(&self, doc_id: u64, data: Vec<u8>, ...) -> Result<(), StoreError<S::Error>>
```

**New API:**
```rust
pub async fn get(&self, doc_id: Vec<u8>) -> Result<Option<Document>, S::Error>
pub async fn put(&self, doc_id: Vec<u8>, data: Vec<u8>, ...) -> Result<(), StoreError<S::Error>>
```

### Storage Trait

**Old API:**
```rust
async fn get(&self, keyspace: &str, doc_id: u64) -> Result<Option<Document>, Self::Error>;
async fn multi_get(&self, keyspace: &str, doc_ids: impl Iterator<Item = u64> + Send) -> ...;
type MetadataIter: Iterator<Item = (u64, HLCTimestamp, bool)>;
```

**New API:**
```rust
async fn get(&self, keyspace: &str, doc_id: Vec<u8>) -> Result<Option<Document>, Self::Error>;
async fn multi_get(&self, keyspace: &str, doc_ids: impl Iterator<Item = Vec<u8>> + Send) -> ...;
type MetadataIter: Iterator<Item = (Vec<u8>, HLCTimestamp, bool)>;
```

## Storage Backend Migration

### SQLite

The SQLite backend automatically handles the schema migration:

- Column type changed from `INTEGER` to `BLOB` for `doc_id`
- Existing databases need schema migration (run migrations on startup)

### LMDB

The LMDB backend now uses `ByteSlice` instead of `U64<LittleEndian>`:

- Database type changed from `Database<U64<LittleEndian>, ByteSlice>` to `Database<ByteSlice, ByteSlice>`
- Existing databases may need recreation or manual migration

### Custom Storage Implementations

If you have custom `Storage` trait implementations:

1. Update all method signatures to use `Vec<u8>` or `&[u8]` for keys
2. Update `MetadataIter` type to yield `(Vec<u8>, HLCTimestamp, bool)`
3. Update internal storage to use byte keys
4. Run the storage test suite to verify correctness:
   ```rust
   datacake_eventual_consistency::storage::test_suite::run_test_suite(your_storage).await;
   ```

## Using the Typed API

### Creating Typed Handles

```rust
use datacake::eventual_consistency::TypedKeyspaceHandle;
use datacake::crdt::DatacakeKey;

// From EventuallyConsistentStore
let users = store.typed_handle::<String>("users")?;

// From ReplicatedStoreHandle
let counters = handle.typed_keyspace::<u64>("counters")?;
```

### Type Safety

The typed API enforces type consistency at runtime:

```rust
// First access with String keys
let users = store.typed_handle::<String>("users")?;
users.put("user_1".to_string(), b"Alice".to_vec(), Consistency::All).await?;

// Later access with same type - OK
let users2 = store.typed_handle::<String>("users")?;

// Try to access with different type - ERROR!
let fail = store.typed_handle::<u64>("users")?;
// Returns: Err(TypeMismatchError { keyspace: "users", expected: "alloc::string::String", actual: "u64" })
```

### Supported Key Types

Out of the box:
- `u64` - Numeric keys
- `String` - Text-based keys
- `Vec<u8>` - Raw byte keys
- `(String, String)` - Composite keys
- `uuid::Uuid` - UUID keys (requires `uuid` feature)

Custom types can implement the `DatacakeKey` trait:

```rust
use datacake::crdt::DatacakeKey;

#[derive(Clone)]
struct CustomKey {
    prefix: String,
    id: u64,
}

impl DatacakeKey for CustomKey {
    fn to_bytes(&self) -> Vec<u8> {
        // Implement serialization
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.prefix.as_bytes());
        bytes.push(0); // Separator
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, datacake::crdt::KeyDeserializationError> {
        // Implement deserialization
        // ...
    }
}
```

## Common Migration Patterns

### Pattern 1: Batch Operations

**Before:**
```rust
let keys = vec![1, 2, 3];
let docs = handle.get_many("keyspace", keys).await?;
```

**After:**
```rust
// Option A: Helper function
let key = |n: u64| n.to_le_bytes().to_vec();
let keys = vec![key(1), key(2), key(3)];
let docs = handle.get_many("keyspace", keys).await?;

// Option B: Typed API
let kspace = handle.typed_keyspace::<u64>("keyspace")?;
let docs = kspace.get_many(vec![1, 2, 3]).await?;
```

### Pattern 2: Iteration

**Before:**
```rust
for (id, ts, is_tombstone) in storage.iter_metadata("keyspace").await? {
    // id is u64
    println!("Document {}: {}", id, ts);
}
```

**After:**
```rust
// Helper to convert bytes back to u64
fn bytes_to_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().unwrap())
}

for (id, ts, is_tombstone) in storage.iter_metadata("keyspace").await? {
    // id is Vec<u8>
    let id_num = bytes_to_u64(&id);
    println!("Document {}: {}", id_num, ts);
}
```

### Pattern 3: Tests

**Before:**
```rust
#[tokio::test]
async fn test_storage() {
    store.put("test", 1, b"data".to_vec(), Consistency::All).await.unwrap();
    let doc = store.get("test", 1).await.unwrap();
    assert!(doc.is_some());
}
```

**After:**
```rust
// Add helper at module level
fn key(n: u64) -> Vec<u8> {
    n.to_le_bytes().to_vec()
}

#[tokio::test]
async fn test_storage() {
    store.put("test", key(1), b"data".to_vec(), Consistency::All).await.unwrap();
    let doc = store.get("test", key(1)).await.unwrap();
    assert!(doc.is_some());
}
```

## FAQ

### Q: Will this break my existing data?

A: No, existing data remains compatible. The serialization format for `u64` keys (little-endian bytes) is preserved when converted to `Vec<u8>`.

### Q: Do I need to migrate my storage backend?

A: SQLite and LMDB users should review the schema changes. Custom implementations need to update their `Storage` trait implementation.

### Q: Can I use different key types in different keyspaces?

A: Yes! That's the whole point. Each keyspace can have its own key type, enforced at runtime.

### Q: What's the performance impact?

A: Minimal. The typed API is a zero-cost abstraction at runtime. The underlying storage still uses efficient byte representations.

### Q: Can I mix typed and untyped APIs?

A: Yes, but be careful. The untyped API always uses `Vec<u8>`, while the typed API converts your key type to `Vec<u8>`. As long as the byte representation matches, they're compatible.

### Q: How do I know which key type a keyspace uses?

A: The system tracks this internally. If you try to access a keyspace with the wrong type, you'll get a `TypeMismatchError`.

## Getting Help

If you encounter issues during migration:

1. Check the [examples](https://github.com/lnx-search/datacake/tree/main/examples) for reference implementations
2. Run the storage test suite to verify your custom implementations
3. Review the API documentation for `TypedKeyspaceHandle`
4. Open an issue on GitHub if you find bugs or need clarification

## Summary

**Quick Migration Checklist:**

- [ ] Add helper function: `let key = |n: u64| n.to_le_bytes().to_vec();`
- [ ] Replace all numeric key arguments with `key(n)`
- [ ] Update custom `Storage` implementations to use `Vec<u8>`
- [ ] Consider migrating to typed API for better ergonomics
- [ ] Test thoroughly with your data
- [ ] Update your documentation

Welcome to type-safe keyspaces in Datacake!
