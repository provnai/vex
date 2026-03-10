// DEPRECATED: This test has been migrated to the 'attest-rs' crate
// to resolve the circular dependency loop (VEX -> Attest -> VEX)
// and establish a proper Directed Acyclic Graph (DAG) for publication.
// See: attest/attest-rs/tests/singularity_high_fid.rs

/*
use attest_rs::runtime::hashing::AuthoritySegment;
use attest_rs::runtime::intent::Intent;
use attest_rs::runtime::keystore_provider::{KeyProvider, TpmKeyProvider};
use attest_rs::runtime::vep::{VepBuildInput, VepBuilder};
use ed25519_dalek::SigningKey;
use serde_json::json;
use std::sync::Arc;
use vex_core::vep::VepPacket;
use vex_hardware::tpm::create_identity_provider;

#[tokio::test]
async fn test_singularity_high_fidelity_tpm() {
    // ... test logic migrated ...
}
*/
