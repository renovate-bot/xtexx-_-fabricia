pub mod branch;

/// Git object ID.
///
/// Currently only SHA-1 is supported.
pub type GitOid = [u8; 20];
