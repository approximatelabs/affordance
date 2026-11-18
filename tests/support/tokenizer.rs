//! Integration-test wrapper around the same fixture the library's unit tests use.
#![allow(dead_code)] // Each integration-test binary uses a different subset of the fixture.
#[path = "../../src/test_support.rs"]
mod fixture;
use affordance::emit::Encoder;
pub use fixture::path;

pub fn encoder() -> Encoder {
    Encoder::from_file(path().to_str().unwrap()).unwrap()
}
