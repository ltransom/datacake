use std::net::SocketAddr;
use anyhow::Result;
use tokio::time::Instant;
use datacake_cluster::{ClusterOptions, ConnectionConfig, Consistency, DatacakeCluster, DCAwareSelector};
use datacake_sqlite::SqliteStorage;

static KEYSPACE: &str = "sqlite-store";

#[tokio::test]
async fn test_basic_sqlite_cluster() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = SqliteStorage::open_in_memory().await?;

    let addr = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let cluster = DatacakeCluster::connect(
        "node-1",
        connection_cfg,
        store,
        DCAwareSelector::default(),
        ClusterOptions::default(),
    )
    .await?;

    let handle = cluster.handle();

    handle
        .put(KEYSPACE, 1, b"Hello, world".to_vec(), Consistency::All)
        .await
        .expect("Put value.");

    let doc = handle
        .get(KEYSPACE, 1)
        .await
        .expect("Get value.")
        .expect("Document should not be none");
    assert_eq!(doc.id, 1);
    assert_eq!(doc.data.as_ref(), b"Hello, world");

    handle
        .del(KEYSPACE, 1, Consistency::All)
        .await
        .expect("Del value.");
    let doc = handle.get(KEYSPACE, 1).await.expect("Get value.");
    assert!(doc.is_none(), "No document should not exist!");

    handle
        .del(KEYSPACE, 2, Consistency::All)
        .await
        .expect("Del value which doesnt exist locally.");
    let doc = handle.get(KEYSPACE, 2).await.expect("Get value.");
    assert!(doc.is_none(), "No document should not exist!");

    cluster.shutdown().await;

    Ok(())
}


#[tokio::test]
async fn test_insert_many_entries() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let store = SqliteStorage::open("my-demo.db").await?;

    let addr = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
    let connection_cfg = ConnectionConfig::new(addr, addr, Vec::<String>::new());

    let cluster = DatacakeCluster::connect(
        "node-1",
        connection_cfg,
        store,
        DCAwareSelector::default(),
        ClusterOptions::default(),
    )
    .await?;

    let handle = cluster.handle();

    let mut docs = Vec::new();
    for i in 0..1_000_000 {
        docs.push((i, b"Hello world, this is a message!" as &'static [u8]));
    }

    let start = Instant::now();
    handle.put_many("my-demo-ks", docs, Consistency::None).await?;
    tracing::info!("Took: {:?}", start.elapsed());

    let keys = 0..1_000_000;
    let start = Instant::now();
    let _docs = handle
        .get_many("my-demo-ks", keys)
        .await?
        .collect::<Vec<_>>();
    tracing::info!("Took: {:?}", start.elapsed());

    cluster.shutdown().await;

    Ok(())
}