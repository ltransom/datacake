//! Type-safe keyspace handles using the `DatacakeKey` trait.
//!
//! This module provides `TypedKeyspaceHandle<K, S>` which enforces compile-time
//! type safety for keyspace operations while maintaining runtime validation to
//! prevent mixing key types for the same keyspace.

use std::marker::PhantomData;

use datacake_crdt::DatacakeKey;
use datacake_node::Consistency;

use crate::core::Document;
use crate::error::StoreError;
use crate::storage::Storage;
use crate::ReplicatedKeyspaceHandle;

/// A type-safe wrapper around a keyspace handle that enforces compile-time
/// type safety for key operations.
///
/// This handle converts keys of type `K` to `Vec<u8>` internally using the
/// `DatacakeKey` trait, providing a more ergonomic and type-safe API.
///
/// # Example
///
/// ```rust,no_run
/// use datacake_crdt::DatacakeKey;
/// use datacake_eventual_consistency::TypedKeyspaceHandle;
/// use datacake_node::Consistency;
///
/// # async fn example(handle: TypedKeyspaceHandle<String, impl datacake_eventual_consistency::Storage>) -> Result<(), Box<dyn std::error::Error>> {
/// // Use String keys directly
/// handle.put("user_123".to_string(), b"user data".to_vec(), Consistency::One).await?;
/// let doc = handle.get("user_123".to_string()).await?;
/// # Ok(())
/// # }
/// ```
pub struct TypedKeyspaceHandle<K, S>
where
    K: DatacakeKey,
    S: Storage,
{
    inner: ReplicatedKeyspaceHandle<S>,
    _phantom: PhantomData<K>,
}

impl<K, S> Clone for TypedKeyspaceHandle<K, S>
where
    K: DatacakeKey,
    S: Storage,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<K, S> TypedKeyspaceHandle<K, S>
where
    K: DatacakeKey,
    S: Storage,
{
    /// Creates a new typed keyspace handle from an untyped handle.
    ///
    /// This is an internal constructor used by the store to create typed handles.
    pub(crate) fn new(inner: ReplicatedKeyspaceHandle<S>) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Retrieves a document from the underlying storage.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use datacake_eventual_consistency::TypedKeyspaceHandle;
    /// # async fn example(handle: TypedKeyspaceHandle<String, impl datacake_eventual_consistency::Storage>) -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = handle.get("user_123".to_string()).await?;
    /// if let Some(doc) = doc {
    ///     println!("Found document: {:?}", doc.data());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, key: K) -> Result<Option<Document>, S::Error> {
        let key_bytes = key.to_bytes();
        self.inner.get(key_bytes).await
    }

    /// Retrieves a set of documents from the underlying storage.
    ///
    /// If a document does not exist with the given ID, it is simply not part
    /// of the returned iterator.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use datacake_eventual_consistency::TypedKeyspaceHandle;
    /// # async fn example(handle: TypedKeyspaceHandle<String, impl datacake_eventual_consistency::Storage>) -> Result<(), Box<dyn std::error::Error>> {
    /// let keys = vec!["user_1".to_string(), "user_2".to_string(), "user_3".to_string()];
    /// let docs = handle.get_many(keys).await?;
    /// for doc in docs {
    ///     println!("Document: {:?}", doc);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_many<I>(&self, keys: I) -> Result<S::DocsIter, S::Error>
    where
        I: IntoIterator<Item = K> + Send,
        I::IntoIter: Send,
    {
        let key_bytes = keys.into_iter().map(|k| k.to_bytes());
        self.inner.get_many(key_bytes).await
    }

    /// Insert or update a single document into the datastore.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use datacake_eventual_consistency::TypedKeyspaceHandle;
    /// # use datacake_node::Consistency;
    /// # async fn example(handle: TypedKeyspaceHandle<String, impl datacake_eventual_consistency::Storage>) -> Result<(), Box<dyn std::error::Error>> {
    /// handle.put(
    ///     "user_123".to_string(),
    ///     b"user data".to_vec(),
    ///     Consistency::All
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn put(
        &self,
        key: K,
        data: Vec<u8>,
        consistency: Consistency,
    ) -> Result<(), StoreError<S::Error>> {
        let key_bytes = key.to_bytes();
        self.inner.put(key_bytes, data, consistency).await
    }

    /// Insert or update multiple documents into the datastore at once.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use datacake_eventual_consistency::TypedKeyspaceHandle;
    /// # use datacake_node::Consistency;
    /// # async fn example(handle: TypedKeyspaceHandle<String, impl datacake_eventual_consistency::Storage>) -> Result<(), Box<dyn std::error::Error>> {
    /// let documents = vec![
    ///     ("user_1".to_string(), b"data1".to_vec()),
    ///     ("user_2".to_string(), b"data2".to_vec()),
    /// ];
    /// handle.put_many(documents, Consistency::All).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn put_many<I>(
        &self,
        documents: I,
        consistency: Consistency,
    ) -> Result<(), StoreError<S::Error>>
    where
        I: IntoIterator<Item = (K, Vec<u8>)> + Send,
        I::IntoIter: Send,
    {
        let docs = documents
            .into_iter()
            .map(|(k, v)| (k.to_bytes(), v));
        self.inner.put_many(docs, consistency).await
    }

    /// Delete a document from the datastore with a given key.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use datacake_eventual_consistency::TypedKeyspaceHandle;
    /// # use datacake_node::Consistency;
    /// # async fn example(handle: TypedKeyspaceHandle<String, impl datacake_eventual_consistency::Storage>) -> Result<(), Box<dyn std::error::Error>> {
    /// handle.del("user_123".to_string(), Consistency::All).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn del(
        &self,
        key: K,
        consistency: Consistency,
    ) -> Result<(), StoreError<S::Error>> {
        let key_bytes = key.to_bytes();
        self.inner.del(key_bytes, consistency).await
    }

    /// Delete multiple documents from the datastore from the set of keys.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use datacake_eventual_consistency::TypedKeyspaceHandle;
    /// # use datacake_node::Consistency;
    /// # async fn example(handle: TypedKeyspaceHandle<String, impl datacake_eventual_consistency::Storage>) -> Result<(), Box<dyn std::error::Error>> {
    /// let keys = vec!["user_1".to_string(), "user_2".to_string()];
    /// handle.del_many(keys, Consistency::All).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn del_many<I>(
        &self,
        keys: I,
        consistency: Consistency,
    ) -> Result<(), StoreError<S::Error>>
    where
        I: IntoIterator<Item = K> + Send,
        I::IntoIter: Send,
    {
        let key_bytes = keys.into_iter().map(|k| k.to_bytes());
        self.inner.del_many(key_bytes, consistency).await
    }
}
