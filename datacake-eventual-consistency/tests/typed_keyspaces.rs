//! Integration tests for typed keyspace handles.

use datacake_eventual_consistency::test_utils::MemStore;
use datacake_eventual_consistency::{
    EventuallyConsistentStoreExtension,
    TypeMismatchError,
};
use datacake_node::{
    ConnectionConfig,
    Consistency,
    DCAwareSelector,
    DatacakeNodeBuilder,
};

static KEYSPACE_USERS: &str = "users";
static KEYSPACE_COUNTERS: &str = "counters";
static KEYSPACE_SESSIONS: &str = "sessions";

#[tokio::test]
async fn test_typed_handle_with_string_keys() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = MemStore::default();
    let addr = test_helper::get_unused_addr();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let node = DatacakeNodeBuilder::<DCAwareSelector>::new(1, connection_cfg)
        .connect()
        .await?;
    let store = node
        .add_extension(EventuallyConsistentStoreExtension::new(store))
        .await?;

    let users = store.typed_handle::<String>(KEYSPACE_USERS)?;

    // Test put
    users
        .put("user_123".to_string(), b"Alice".to_vec(), Consistency::All)
        .await?;

    // Test get
    let doc = users.get("user_123".to_string()).await?;
    assert!(doc.is_some());
    let doc = doc.unwrap();
    assert_eq!(doc.data(), b"Alice");

    // Test update
    users
        .put("user_123".to_string(), b"Alice Smith".to_vec(), Consistency::All)
        .await?;

    let doc = users.get("user_123".to_string()).await?;
    assert!(doc.is_some());
    assert_eq!(doc.unwrap().data(), b"Alice Smith");

    // Test delete
    users.del("user_123".to_string(), Consistency::All).await?;

    let doc = users.get("user_123".to_string()).await?;
    assert!(doc.is_none());

    node.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_typed_handle_with_u64_keys() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = MemStore::default();
    let addr = test_helper::get_unused_addr();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let node = DatacakeNodeBuilder::<DCAwareSelector>::new(1, connection_cfg)
        .connect()
        .await?;
    let store = node
        .add_extension(EventuallyConsistentStoreExtension::new(store))
        .await?;

    let counters = store.typed_handle::<u64>(KEYSPACE_COUNTERS)?;

    // Test put
    counters.put(42, b"counter_value".to_vec(), Consistency::All).await?;

    // Test get
    let doc = counters.get(42).await?;
    assert!(doc.is_some());
    let doc = doc.unwrap();
    assert_eq!(doc.data(), b"counter_value");

    // Test multi_put
    let documents = vec![
        (1, b"one".to_vec()),
        (2, b"two".to_vec()),
        (3, b"three".to_vec()),
    ];
    counters.put_many(documents, Consistency::All).await?;

    // Test get_many
    let docs: Vec<_> = counters.get_many(vec![1, 2, 3]).await?.collect();
    assert_eq!(docs.len(), 3);

    // Test del_many
    counters.del_many(vec![1, 2, 3], Consistency::All).await?;

    let doc = counters.get(1).await?;
    assert!(doc.is_none());

    node.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_type_mismatch_error() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = MemStore::default();
    let addr = test_helper::get_unused_addr();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let node = DatacakeNodeBuilder::<DCAwareSelector>::new(1, connection_cfg)
        .connect()
        .await?;
    let store = node
        .add_extension(EventuallyConsistentStoreExtension::new(store))
        .await?;

    // First access with String
    let _users = store.typed_handle::<String>(KEYSPACE_USERS)?;

    // Try to access with u64 should fail
    let result = store.typed_handle::<u64>(KEYSPACE_USERS);
    assert!(result.is_err());

    match result {
        Err(TypeMismatchError::KeyTypeMismatch { keyspace, expected, actual }) => {
            assert_eq!(keyspace, KEYSPACE_USERS);
            assert!(expected.contains("String"));
            assert!(actual.contains("u64"));
        }
        Ok(_) => panic!("Expected type mismatch error but got Ok"),
    }

    node.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_mixed_keyspace_types() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = MemStore::default();
    let addr = test_helper::get_unused_addr();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let node = DatacakeNodeBuilder::<DCAwareSelector>::new(1, connection_cfg)
        .connect()
        .await?;
    let store = node
        .add_extension(EventuallyConsistentStoreExtension::new(store))
        .await?;

    // Create handles with different key types for different keyspaces
    let users = store.typed_handle::<String>(KEYSPACE_USERS)?;
    let counters = store.typed_handle::<u64>(KEYSPACE_COUNTERS)?;
    let sessions = store.typed_handle::<String>(KEYSPACE_SESSIONS)?;

    // Use all handles
    users.put("user_1".to_string(), b"Alice".to_vec(), Consistency::All).await?;
    counters.put(42, b"counter".to_vec(), Consistency::All).await?;
    sessions.put("session_abc".to_string(), b"data".to_vec(), Consistency::All).await?;

    // Verify each can be accessed
    assert!(users.get("user_1".to_string()).await?.is_some());
    assert!(counters.get(42).await?.is_some());
    assert!(sessions.get("session_abc".to_string()).await?.is_some());

    // Verify type safety is maintained
    let _users2 = store.typed_handle::<String>(KEYSPACE_USERS)?; // OK - same type
    assert!(store.typed_handle::<u64>(KEYSPACE_USERS).is_err()); // Error - different type

    node.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_typed_handle_from_replicated_store_handle() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = MemStore::default();
    let addr = test_helper::get_unused_addr();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let node = DatacakeNodeBuilder::<DCAwareSelector>::new(1, connection_cfg)
        .connect()
        .await?;
    let store = node
        .add_extension(EventuallyConsistentStoreExtension::new(store))
        .await?;

    // Get the replicated store handle
    let handle = store.handle();

    // Create typed handles from the replicated handle
    let users = handle.typed_keyspace::<String>(KEYSPACE_USERS)?;
    let counters = handle.typed_keyspace::<u64>(KEYSPACE_COUNTERS)?;

    // Use the handles
    users.put("user_1".to_string(), b"Alice".to_vec(), Consistency::All).await?;
    counters.put(1, b"one".to_vec(), Consistency::All).await?;

    // Verify
    assert!(users.get("user_1".to_string()).await?.is_some());
    assert!(counters.get(1).await?.is_some());

    node.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_composite_key_type() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = MemStore::default();
    let addr = test_helper::get_unused_addr();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let node = DatacakeNodeBuilder::<DCAwareSelector>::new(1, connection_cfg)
        .connect()
        .await?;
    let store = node
        .add_extension(EventuallyConsistentStoreExtension::new(store))
        .await?;

    // Create handle with composite key type
    let tenants = store.typed_handle::<(String, String)>("tenant_resources")?;

    // Use composite keys (tenant_id, resource_id)
    let key1 = ("tenant_abc".to_string(), "resource_123".to_string());
    let key2 = ("tenant_abc".to_string(), "resource_456".to_string());
    let key3 = ("tenant_xyz".to_string(), "resource_123".to_string());

    tenants.put(key1.clone(), b"data1".to_vec(), Consistency::All).await?;
    tenants.put(key2.clone(), b"data2".to_vec(), Consistency::All).await?;
    tenants.put(key3.clone(), b"data3".to_vec(), Consistency::All).await?;

    // Verify retrieval
    assert!(tenants.get(key1).await?.is_some());
    assert!(tenants.get(key2).await?.is_some());
    assert!(tenants.get(key3).await?.is_some());

    node.shutdown().await;
    Ok(())
}
