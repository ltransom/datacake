
use rkyv::{Archive, Serialize};

use crate::rkyv_tooling::DatacakeSerializer;
use crate::Status;

use http_body_util::BodyExt;

/// A wrapper type around different body types
pub enum Body {
    Incoming(hyper::body::Incoming),
    Full(http_body_util::Full<bytes::Bytes>),
}

impl Body {
    /// Creates a new body from an incoming body.
    pub fn new(inner: hyper::body::Incoming) -> Self {
        Self::Incoming(inner)
    }

    /// Creates a new body from bytes.
    pub fn from_bytes(bytes: impl Into<bytes::Bytes>) -> Self {
        Self::Full(http_body_util::Full::new(bytes.into()))
    }

    /// Collects the body into bytes
    pub async fn collect(self) -> Result<bytes::Bytes, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Body::Incoming(body) => {
                let collected = body.collect().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                Ok(collected.to_bytes())
            }
            Body::Full(body) => {
                let collected = body.collect().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                Ok(collected.to_bytes())
            }
        }
    }

    /// Converts to bytes for HTTP responses
    pub fn into_http_body(self) -> http_body_util::Full<bytes::Bytes> {
        match self {
            Body::Full(body) => body,
            Body::Incoming(_) => {
                // For immediate conversion, we return empty body
                // In practice, this should be collected first
                http_body_util::Full::new(bytes::Bytes::new())
            }
        }
    }
}

impl From<hyper::body::Incoming> for Body {
    fn from(value: hyper::body::Incoming) -> Self {
        Self::new(value)
    }
}

impl From<Vec<u8>> for Body {
    fn from(value: Vec<u8>) -> Self {
        Self::from_bytes(value)
    }
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        Self::from_bytes(value)
    }
}

impl From<&'static str> for Body {
    fn from(value: &'static str) -> Self {
        Self::from_bytes(value)
    }
}


/// The serializer trait converting replies into hyper bodies.
///
/// This is a light abstraction to allow users to be able to
/// stream data across the RPC system which may not fit in memory.
///
/// Any types which implement [TryAsBody] will implement this type.
pub trait TryIntoBody {
    /// Try convert the reply into a body or return an error
    /// status.
    fn try_into_body(self) -> Result<Body, Status>;
}

/// The serializer trait for converting replies into hyper bodies
/// using a reference to self.
///
/// This will work for most implementations but if you want to stream
/// hyper bodies for example, you cannot implement this trait.
pub trait TryAsBody {
    /// Try convert the reply into a body or return an error
    /// status.
    fn try_as_body(&self) -> Result<Body, Status>;
}

impl<T> TryAsBody for T
where
    T: Archive + Serialize<DatacakeSerializer>,
{
    #[inline]
    fn try_as_body(&self) -> Result<Body, Status> {
        crate::rkyv_tooling::to_view_bytes(self)
            .map(|v| Body::from(v.to_vec()))
            .map_err(|e| Status::internal(e.to_string()))
    }
}

impl<T> TryIntoBody for T
where
    T: TryAsBody,
{
    #[inline]
    fn try_into_body(self) -> Result<Body, Status> {
        <Self as TryAsBody>::try_as_body(&self)
    }
}

impl TryIntoBody for Body {
    #[inline]
    fn try_into_body(self) -> Result<Body, Status> {
        Ok(self)
    }
}
