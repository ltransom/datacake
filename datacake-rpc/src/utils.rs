use rkyv::AlignedVec;
use crate::body::Body;

pub async fn to_aligned(
    body: Body,
) -> Result<AlignedVec, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = body.collect().await?;
    let mut vec = AlignedVec::with_capacity(bytes.len());
    vec.extend_from_slice(&bytes);
    Ok(vec)
}
