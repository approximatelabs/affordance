//! Optional host integrations composed through the existing executor boundary.
//! Protocol libraries live in TSX; these adapters supply external I/O.
#[cfg(feature = "http")]
pub mod http;
