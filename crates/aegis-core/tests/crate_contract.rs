//! Integration tests for the public aegis-core crate contract.

use aegis_core::CORE_API_VERSION;

#[test]
fn exposes_core_api_version() {
    assert_eq!(CORE_API_VERSION, 1);
}
