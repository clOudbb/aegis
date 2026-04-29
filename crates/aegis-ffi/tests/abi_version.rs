//! ABI version contract tests for the Aegis FFI facade.

use aegis_ffi::{
    AEGIS_ABI_MAJOR, AEGIS_ABI_MINOR, AEGIS_ABI_PATCH, AegisAbiVersion, aegis_abi_version,
};

#[test]
fn abi_version_struct_reports_its_size() {
    let version = AegisAbiVersion::CURRENT;

    assert_eq!(version.size, core::mem::size_of::<AegisAbiVersion>());
}

#[test]
fn exported_abi_version_reports_major_version() {
    let version = aegis_abi_version();

    assert_eq!(version.major, AEGIS_ABI_MAJOR);
}

#[test]
fn exported_abi_version_reports_minor_version() {
    let version = aegis_abi_version();

    assert_eq!(version.minor, AEGIS_ABI_MINOR);
}

#[test]
fn exported_abi_version_reports_patch_version() {
    let version = aegis_abi_version();

    assert_eq!(version.patch, AEGIS_ABI_PATCH);
}

#[test]
fn exported_abi_version_reports_core_api_version() {
    let version = aegis_abi_version();

    assert_eq!(version.core_api_version, aegis_core::CORE_API_VERSION);
}
