use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fluxtube_network::ResilientHttpClient;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum UpdaterError {
    #[error("Network error: {0}")]
    Network(#[from] fluxtube_network::NetworkError),

    #[error("Invalid version string: {0}")]
    InvalidVersion(String),

    #[error("Untrusted release signature: {0}")]
    SignatureVerificationFailed(String),

    #[error("Artifact SHA-256 checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Release artifact is unsigned; unsigned updates are prohibited")]
    UnsignedUpdateRejected,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Invalid update URL: {0}")]
    InvalidUrl(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
}

impl SemVer {
    pub fn parse(raw: &str) -> Result<Self, UpdaterError> {
        let trimmed = raw.trim().trim_start_matches('v');
        let (core, pre) = match trimmed.split_once('-') {
            Some((c, p)) => (c, Some(p.to_string())),
            None => (trimmed, None),
        };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return Err(UpdaterError::InvalidVersion(raw.to_string()));
        }
        Ok(Self {
            major: parts[0]
                .parse()
                .map_err(|_| UpdaterError::InvalidVersion(raw.to_string()))?,
            minor: parts[1]
                .parse()
                .map_err(|_| UpdaterError::InvalidVersion(raw.to_string()))?,
            patch: parts[2]
                .parse()
                .map_err(|_| UpdaterError::InvalidVersion(raw.to_string()))?,
            pre_release: pre,
        })
    }

    pub fn is_prerelease(&self) -> bool {
        self.pre_release.is_some()
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch)) {
            Ordering::Equal => match (&self.pre_release, &other.pre_release) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(b),
            },
            non_eq => non_eq,
        }
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    pub channel: UpdateChannel,
    pub release_notes: String,
    pub published_at: String,
    pub asset_url: String,
    pub sha256_hex: String,
    pub ed25519_signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckOutcome {
    UpToDate { current_version: String },
    UpdateAvailable(ReleaseManifest),
}

/// Computes lowercase hex SHA-256 digest of a byte buffer.
pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verifies both SHA-256 digest and Ed25519 cryptographic signature of an update artifact.
pub fn verify_artifact(
    bytes: &[u8],
    expected_sha256_hex: &str,
    ed25519_signature_hex: &str,
    public_key_bytes: &[u8; 32],
) -> Result<(), UpdaterError> {
    if ed25519_signature_hex.trim().is_empty() {
        return Err(UpdaterError::UnsignedUpdateRejected);
    }

    let actual_hash = compute_sha256_hex(bytes);
    if !actual_hash.eq_ignore_ascii_case(expected_sha256_hex.trim()) {
        return Err(UpdaterError::ChecksumMismatch {
            expected: expected_sha256_hex.to_string(),
            actual: actual_hash,
        });
    }

    let sig_bytes = hex::decode(ed25519_signature_hex.trim())
        .map_err(|e| UpdaterError::SignatureVerificationFailed(format!("Invalid hex signature: {}", e)))?;
    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| {
        UpdaterError::SignatureVerificationFailed("Ed25519 signature must be 64 bytes".to_string())
    })?;
    let signature = Signature::from_bytes(&sig_array);

    let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
        .map_err(|e| UpdaterError::SignatureVerificationFailed(format!("Invalid public key: {}", e)))?;

    verifying_key
        .verify(bytes, &signature)
        .map_err(|e| UpdaterError::SignatureVerificationFailed(format!("Verification failed: {}", e)))?;

    Ok(())
}

/// Safe atomic staging and rollback manager for verified update payloads.
pub struct StagedUpdate {
    staging_path: PathBuf,
    backup_path: Option<PathBuf>,
}

impl StagedUpdate {
    /// Stages verified bytes to a temporary file and atomically swaps with `target_path`,
    /// preserving a rollback backup if replacement fails.
    pub fn apply_verified(
        bytes: &[u8],
        expected_sha256_hex: &str,
        ed25519_signature_hex: &str,
        public_key_bytes: &[u8; 32],
        target_path: &Path,
    ) -> Result<Self, UpdaterError> {
        verify_artifact(
            bytes,
            expected_sha256_hex,
            ed25519_signature_hex,
            public_key_bytes,
        )?;

        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;

        let staging_path = parent.join(format!(
            ".{}.staging",
            target_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("fluxtube")
        ));
        std::fs::write(&staging_path, bytes)?;

        let backup_path = if target_path.exists() {
            let bak = parent.join(format!(
                ".{}.bak",
                target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("fluxtube")
            ));
            std::fs::copy(target_path, &bak)?;
            Some(bak)
        } else {
            None
        };

        if let Err(e) = std::fs::rename(&staging_path, target_path) {
            // Rollback if backup exists
            if let Some(ref bak) = backup_path {
                let _ = std::fs::copy(bak, target_path);
            }
            let _ = std::fs::remove_file(&staging_path);
            return Err(UpdaterError::Io(e));
        }

        Ok(Self {
            staging_path,
            backup_path,
        })
    }

    pub fn rollback(&self, target_path: &Path) -> Result<(), UpdaterError> {
        if let Some(ref bak) = self.backup_path {
            if bak.exists() {
                std::fs::copy(bak, target_path)?;
            }
        }
        let _ = std::fs::remove_file(&self.staging_path);
        Ok(())
    }

    pub fn cleanup_backup(&self) {
        if let Some(ref bak) = self.backup_path {
            let _ = std::fs::remove_file(bak);
        }
    }
}

/// Client for checking and downloading cryptographically signed releases.
pub struct UpdaterClient {
    http: ResilientHttpClient,
    manifest_url: String,
    public_key: [u8; 32],
}

impl UpdaterClient {
    pub fn new(
        http: ResilientHttpClient,
        manifest_url: impl Into<String>,
        public_key: [u8; 32],
    ) -> Self {
        Self {
            http,
            manifest_url: manifest_url.into(),
            public_key,
        }
    }

    pub async fn check_for_updates(
        &self,
        current_version: &str,
        channel: UpdateChannel,
    ) -> Result<UpdateCheckOutcome, UpdaterError> {
        let current = SemVer::parse(current_version)?;
        let releases: Vec<ReleaseManifest> = self.http.get_json(&self.manifest_url).await?;

        let mut best: Option<(SemVer, ReleaseManifest)> = None;
        for rel in releases {
            if channel == UpdateChannel::Stable && rel.channel != UpdateChannel::Stable {
                continue;
            }
            if rel.ed25519_signature_hex.trim().is_empty() {
                // Skip unsigned releases automatically
                continue;
            }
            if let Ok(ver) = SemVer::parse(&rel.version) {
                if channel == UpdateChannel::Stable && ver.is_prerelease() {
                    continue;
                }
                if ver > current {
                    let is_newer = best.as_ref().map(|(b, _)| &ver > b).unwrap_or(true);
                    if is_newer {
                        best = Some((ver, rel));
                    }
                }
            }
        }

        match best {
            Some((_, manifest)) => Ok(UpdateCheckOutcome::UpdateAvailable(manifest)),
            None => Ok(UpdateCheckOutcome::UpToDate {
                current_version: current_version.to_string(),
            }),
        }
    }

    pub async fn download_and_verify(
        &self,
        manifest: &ReleaseManifest,
    ) -> Result<Vec<u8>, UpdaterError> {
        let parsed = url::Url::parse(&manifest.asset_url)
            .map_err(|e| UpdaterError::InvalidUrl(e.to_string()))?;
        if parsed.scheme() != "https" && parsed.scheme() != "http" {
            return Err(UpdaterError::InvalidUrl(format!(
                "Disallowed update URL scheme: {}",
                parsed.scheme()
            )));
        }

        let bytes = self.http.get_bytes(&manifest.asset_url).await?;
        verify_artifact(
            &bytes,
            &manifest.sha256_hex,
            &manifest.ed25519_signature_hex,
            &self.public_key,
        )?;
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use tempfile::tempdir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_semver_ordering_and_channel_rules() {
        let v1 = SemVer::parse("0.1.0").unwrap();
        let v2_beta = SemVer::parse("0.2.0-beta.1").unwrap();
        let v2 = SemVer::parse("0.2.0").unwrap();
        assert!(v2 > v2_beta);
        assert!(v2_beta > v1);
        assert!(v2_beta.is_prerelease());
        assert!(!v2.is_prerelease());
    }

    #[test]
    fn test_ed25519_and_sha256_verification_and_rollback() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let payload = b"fluxtube-binary-v0.2.0";
        let sha256 = compute_sha256_hex(payload);
        let sig = signing_key.sign(payload);
        let sig_hex = hex::encode(sig.to_bytes());

        assert!(verify_artifact(payload, &sha256, &sig_hex, &verifying_key).is_ok());

        // Reject unsigned
        assert!(matches!(
            verify_artifact(payload, &sha256, "", &verifying_key),
            Err(UpdaterError::UnsignedUpdateRejected)
        ));

        // Reject tampered payload
        assert!(matches!(
            verify_artifact(b"tampered", &sha256, &sig_hex, &verifying_key),
            Err(UpdaterError::ChecksumMismatch { .. })
        ));

        // Atomic staging + rollback
        let dir = tempdir().unwrap();
        let target = dir.path().join("fluxtube.exe");
        std::fs::write(&target, b"old-binary").unwrap();

        let staged = StagedUpdate::apply_verified(
            payload,
            &sha256,
            &sig_hex,
            &verifying_key,
            &target,
        )
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), payload);

        staged.rollback(&target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"old-binary");
    }

    #[tokio::test]
    async fn test_updater_client_channel_filtering_and_download() {
        let server = MockServer::start().await;
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let payload = b"signed-update-zip";
        let sha256 = compute_sha256_hex(payload);
        let sig_hex = hex::encode(signing_key.sign(payload).to_bytes());

        Mock::given(method("GET"))
            .and(path("/releases.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "version": "0.2.0-beta.1",
                    "channel": "beta",
                    "release_notes": "Beta build",
                    "published_at": "2026-11-01T00:00:00Z",
                    "asset_url": format!("{}/fluxtube-0.2.0-beta.1.bin", server.uri()),
                    "sha256_hex": sha256,
                    "ed25519_signature_hex": sig_hex
                },
                {
                    "version": "0.1.5",
                    "channel": "stable",
                    "release_notes": "Stable patch",
                    "published_at": "2026-10-15T00:00:00Z",
                    "asset_url": format!("{}/fluxtube-0.1.5.bin", server.uri()),
                    "sha256_hex": sha256,
                    "ed25519_signature_hex": sig_hex
                }
            ])))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/fluxtube-0.1.5.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.as_slice()))
            .mount(&server)
            .await;

        let http = ResilientHttpClient::builder().build().unwrap();
        let client = UpdaterClient::new(http, format!("{}/releases.json", server.uri()), verifying_key);

        // Stable channel skips beta 0.2.0-beta.1 and selects 0.1.5
        let stable_check = client
            .check_for_updates("0.1.0", UpdateChannel::Stable)
            .await
            .unwrap();
        match stable_check {
            UpdateCheckOutcome::UpdateAvailable(m) => {
                assert_eq!(m.version, "0.1.5");
                let downloaded = client.download_and_verify(&m).await.unwrap();
                assert_eq!(downloaded, payload);
            }
            _ => panic!("Expected UpdateAvailable"),
        }

        // Beta channel picks 0.2.0-beta.1
        let beta_check = client
            .check_for_updates("0.1.0", UpdateChannel::Beta)
            .await
            .unwrap();
        match beta_check {
            UpdateCheckOutcome::UpdateAvailable(m) => assert_eq!(m.version, "0.2.0-beta.1"),
            _ => panic!("Expected Beta update"),
        }
    }
}
