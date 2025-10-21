//! Key type system for Datacake.
//!
//! This module defines the `DatacakeKey` trait and provides implementations
//! for common key types including u64, String, Vec<u8>, Uuid, and composite keys.

use std::fmt::Debug;

#[cfg(feature = "uuid")]
use uuid::Uuid;

/// Maximum allowed size for serialized keys (4KB).
///
/// This limit prevents abuse and ensures reasonable memory usage.
pub const MAX_KEY_SIZE: usize = 4096;

/// Error type for key deserialization failures.
#[derive(Debug, thiserror::Error)]
pub enum KeyDeserializationError {
    #[error("Key size {0} bytes exceeds maximum allowed size of {MAX_KEY_SIZE} bytes")]
    KeyTooLarge(usize),

    #[error("Invalid key format: {0}")]
    InvalidFormat(String),

    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[cfg(feature = "uuid")]
    #[error("UUID parsing error: {0}")]
    UuidError(#[from] uuid::Error),
}

/// Trait defining the interface for key types used in Datacake keyspaces.
///
/// This trait allows each keyspace to use a semantically appropriate key type
/// (e.g., String for user IDs, Uuid for sessions, u64 for performance-critical counters).
///
/// # Example
///
/// ```
/// use datacake_crdt::{DatacakeKey, KeyDeserializationError};
///
/// // String keys for user-facing identifiers
/// let user_key = "user_123".to_string();
/// let bytes = user_key.to_bytes();
/// let restored = String::from_bytes(&bytes).unwrap();
/// assert_eq!(user_key, restored);
/// ```
pub trait DatacakeKey: Clone + std::hash::Hash + Eq + Debug + Send + Sync + 'static {
    /// Serialize the key to bytes for storage and transmission.
    ///
    /// The serialized form must not exceed `MAX_KEY_SIZE` bytes.
    fn to_bytes(&self) -> Vec<u8>;

    /// Deserialize a key from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The byte slice is too large (exceeds `MAX_KEY_SIZE`)
    /// - The byte slice cannot be deserialized to the target type
    fn from_bytes(bytes: &[u8]) -> Result<Self, KeyDeserializationError>;
}

// ========================================
// Implementation for u64 (current default)
// ========================================

impl DatacakeKey for u64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, KeyDeserializationError> {
        if bytes.len() > MAX_KEY_SIZE {
            return Err(KeyDeserializationError::KeyTooLarge(bytes.len()));
        }

        if bytes.len() != 8 {
            return Err(KeyDeserializationError::InvalidFormat(
                format!("Expected 8 bytes for u64, got {}", bytes.len())
            ));
        }

        let mut array = [0u8; 8];
        array.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(array))
    }
}

// ========================================
// Implementation for String
// ========================================

impl DatacakeKey for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, KeyDeserializationError> {
        if bytes.len() > MAX_KEY_SIZE {
            return Err(KeyDeserializationError::KeyTooLarge(bytes.len()));
        }

        String::from_utf8(bytes.to_vec()).map_err(Into::into)
    }
}

// ========================================
// Implementation for Vec<u8>
// ========================================

impl DatacakeKey for Vec<u8> {
    fn to_bytes(&self) -> Vec<u8> {
        self.clone()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, KeyDeserializationError> {
        if bytes.len() > MAX_KEY_SIZE {
            return Err(KeyDeserializationError::KeyTooLarge(bytes.len()));
        }

        Ok(bytes.to_vec())
    }
}

// ========================================
// Implementation for Uuid
// ========================================

#[cfg(feature = "uuid")]
impl DatacakeKey for Uuid {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, KeyDeserializationError> {
        if bytes.len() > MAX_KEY_SIZE {
            return Err(KeyDeserializationError::KeyTooLarge(bytes.len()));
        }

        if bytes.len() != 16 {
            return Err(KeyDeserializationError::InvalidFormat(
                format!("Expected 16 bytes for UUID, got {}", bytes.len())
            ));
        }

        Uuid::from_slice(bytes).map_err(Into::into)
    }
}

// ========================================
// Implementation for composite key (String, String)
// ========================================

impl DatacakeKey for (String, String) {
    fn to_bytes(&self) -> Vec<u8> {
        // Format: [len1: u32 | data1 | len2: u32 | data2]
        let bytes1 = self.0.as_bytes();
        let bytes2 = self.1.as_bytes();

        let len1 = bytes1.len() as u32;
        let len2 = bytes2.len() as u32;

        let mut result = Vec::with_capacity(8 + bytes1.len() + bytes2.len());
        result.extend_from_slice(&len1.to_le_bytes());
        result.extend_from_slice(bytes1);
        result.extend_from_slice(&len2.to_le_bytes());
        result.extend_from_slice(bytes2);

        result
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, KeyDeserializationError> {
        if bytes.len() > MAX_KEY_SIZE {
            return Err(KeyDeserializationError::KeyTooLarge(bytes.len()));
        }

        if bytes.len() < 8 {
            return Err(KeyDeserializationError::InvalidFormat(
                "Composite key too short".to_string()
            ));
        }

        // Read first length
        let mut len1_bytes = [0u8; 4];
        len1_bytes.copy_from_slice(&bytes[0..4]);
        let len1 = u32::from_le_bytes(len1_bytes) as usize;

        if bytes.len() < 8 + len1 {
            return Err(KeyDeserializationError::InvalidFormat(
                "Invalid first component length".to_string()
            ));
        }

        // Read first string
        let str1 = String::from_utf8(bytes[4..4 + len1].to_vec())?;

        // Read second length
        let mut len2_bytes = [0u8; 4];
        len2_bytes.copy_from_slice(&bytes[4 + len1..8 + len1]);
        let len2 = u32::from_le_bytes(len2_bytes) as usize;

        if bytes.len() != 8 + len1 + len2 {
            return Err(KeyDeserializationError::InvalidFormat(
                "Invalid second component length".to_string()
            ));
        }

        // Read second string
        let str2 = String::from_utf8(bytes[8 + len1..].to_vec())?;

        Ok((str1, str2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u64_key_roundtrip() {
        let key = 42u64;
        let bytes = key.to_bytes();
        let restored = u64::from_bytes(&bytes).unwrap();
        assert_eq!(key, restored);
    }

    #[test]
    fn test_string_key_roundtrip() {
        let key = "user_123".to_string();
        let bytes = key.to_bytes();
        let restored = String::from_bytes(&bytes).unwrap();
        assert_eq!(key, restored);
    }

    #[test]
    fn test_vec_key_roundtrip() {
        let key = vec![1u8, 2, 3, 4, 5];
        let bytes = key.to_bytes();
        let restored = Vec::<u8>::from_bytes(&bytes).unwrap();
        assert_eq!(key, restored);
    }

    #[cfg(feature = "uuid")]
    #[test]
    fn test_uuid_key_roundtrip() {
        let key = Uuid::new_v4();
        let bytes = DatacakeKey::to_bytes(&key);
        let restored = <Uuid as DatacakeKey>::from_bytes(&bytes).unwrap();
        assert_eq!(key, restored);
    }

    #[test]
    fn test_composite_key_roundtrip() {
        let key = ("tenant_123".to_string(), "resource_456".to_string());
        let bytes = key.to_bytes();
        let restored = <(String, String)>::from_bytes(&bytes).unwrap();
        assert_eq!(key, restored);
    }

    #[test]
    fn test_key_size_limit() {
        let large_key = "x".repeat(MAX_KEY_SIZE + 1);
        let bytes = large_key.as_bytes();
        let result = String::from_bytes(bytes);
        assert!(matches!(result, Err(KeyDeserializationError::KeyTooLarge(_))));
    }

    #[test]
    fn test_invalid_u64_size() {
        let bytes = vec![1u8, 2, 3]; // Wrong size
        let result = u64::from_bytes(&bytes);
        assert!(matches!(result, Err(KeyDeserializationError::InvalidFormat(_))));
    }

    #[test]
    fn test_invalid_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = String::from_bytes(&invalid_utf8);
        assert!(matches!(result, Err(KeyDeserializationError::Utf8Error(_))));
    }
}
