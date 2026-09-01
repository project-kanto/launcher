use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const OP_COPY: u8 = 1;
const OP_INSERT: u8 = 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Android,
    Ios,
}

/// The existing Kanto browser manifest plus optional launcher fields.
///
/// Optional fields keep current production manifests readable while the server rolls out the
/// launcher contract. A versioned launcher manifest requires all of them during validation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameReleaseManifest {
    pub platform: Platform,
    pub input_sha256: String,
    pub input_bytes: u64,
    pub output_sha256: String,
    pub output_bytes: u64,
    pub delta_bytes: u64,
    pub endpoint: String,
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub release_version: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub minimum_launcher_version: Option<String>,
    #[serde(default)]
    pub delta_sha256: Option<String>,
    #[serde(default)]
    pub delta_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ReleaseError {
    #[error("{field} must be a 64-character lowercase SHA-256 digest")]
    InvalidDigest { field: &'static str },
    #[error("{field} must be greater than zero")]
    EmptySize { field: &'static str },
    #[error("launcher manifest schema {0} is not supported")]
    UnsupportedSchema(u32),
    #[error("launcher manifest schema 1 is missing {0}")]
    MissingField(&'static str),
    #[error("source URL must use HTTPS")]
    InsecureSourceUrl,
    #[error("{artifact} size mismatch: got {actual}, expected {expected}")]
    SizeMismatch {
        artifact: &'static str,
        actual: u64,
        expected: u64,
    },
    #[error("{0} SHA-256 verification failed")]
    DigestMismatch(&'static str),
    #[error("delta operation {operation} is truncated at byte {offset}")]
    Truncated {
        operation: &'static str,
        offset: usize,
    },
    #[error("unknown delta operation {operation} at byte {offset}")]
    UnknownOperation { operation: u8, offset: usize },
    #[error("delta copy range {offset}+{length} exceeds the source")]
    CopyOutOfRange { offset: u64, length: u64 },
    #[error("patched output is too large for this platform")]
    OutputTooLarge,
}

impl GameReleaseManifest {
    pub fn validate(&self) -> Result<(), ReleaseError> {
        validate_digest("input_sha256", &self.input_sha256)?;
        validate_digest("output_sha256", &self.output_sha256)?;
        validate_size("input_bytes", self.input_bytes)?;
        validate_size("output_bytes", self.output_bytes)?;
        validate_size("delta_bytes", self.delta_bytes)?;

        match self.schema_version {
            None => Ok(()),
            Some(1) => {
                required("release_version", self.release_version.as_deref())?;
                required("published_at", self.published_at.as_deref())?;
                required(
                    "minimum_launcher_version",
                    self.minimum_launcher_version.as_deref(),
                )?;
                let delta_sha = self
                    .delta_sha256
                    .as_deref()
                    .ok_or(ReleaseError::MissingField("delta_sha256"))?;
                validate_digest("delta_sha256", delta_sha)?;
                required("delta_url", self.delta_url.as_deref())?;
                if let Some(source_url) = self.source_url.as_deref() {
                    let parsed =
                        url::Url::parse(source_url).map_err(|_| ReleaseError::InsecureSourceUrl)?;
                    if parsed.scheme() != "https"
                        || parsed.host_str().is_none()
                        || !parsed.username().is_empty()
                        || parsed.password().is_some()
                    {
                        return Err(ReleaseError::InsecureSourceUrl);
                    }
                }
                Ok(())
            }
            Some(version) => Err(ReleaseError::UnsupportedSchema(version)),
        }
    }
}

/// Apply and verify one exact Kanto release.
///
/// ponytail: the supported clients are about 60 MB, so one in-memory output keeps the parser small;
/// switch to a temporary-file writer only if future exact profiles grow materially.
pub fn apply_verified(
    source: &[u8],
    delta: &[u8],
    manifest: &GameReleaseManifest,
) -> Result<Vec<u8>, ReleaseError> {
    manifest.validate()?;
    verify_size("source", source, manifest.input_bytes)?;
    verify_digest("source", source, &manifest.input_sha256)?;
    verify_size("delta", delta, manifest.delta_bytes)?;
    if let Some(expected) = &manifest.delta_sha256 {
        verify_digest("delta", delta, expected)?;
    }

    let max_output =
        usize::try_from(manifest.output_bytes).map_err(|_| ReleaseError::OutputTooLarge)?;
    let output = apply_delta_with_limit(source, delta, max_output)?;
    verify_size("output", &output, manifest.output_bytes)?;
    verify_digest("output", &output, &manifest.output_sha256)?;
    Ok(output)
}

pub fn apply_delta(source: &[u8], delta: &[u8]) -> Result<Vec<u8>, ReleaseError> {
    apply_delta_with_limit(source, delta, usize::MAX)
}

fn apply_delta_with_limit(
    source: &[u8],
    delta: &[u8],
    max_output: usize,
) -> Result<Vec<u8>, ReleaseError> {
    let mut output = Vec::new();
    let mut cursor = 0usize;

    while cursor < delta.len() {
        let operation_offset = cursor;
        let operation = delta[cursor];
        cursor += 1;

        match operation {
            OP_COPY => {
                let offset = read_u64(delta, &mut cursor, "copy", operation_offset)?;
                let length = read_u64(delta, &mut cursor, "copy", operation_offset)?;
                let start = usize::try_from(offset)
                    .map_err(|_| ReleaseError::CopyOutOfRange { offset, length })?;
                let length_usize = usize::try_from(length)
                    .map_err(|_| ReleaseError::CopyOutOfRange { offset, length })?;
                let end = start
                    .checked_add(length_usize)
                    .filter(|end| *end <= source.len())
                    .ok_or(ReleaseError::CopyOutOfRange { offset, length })?;
                if output
                    .len()
                    .checked_add(length_usize)
                    .is_none_or(|size| size > max_output)
                {
                    return Err(ReleaseError::OutputTooLarge);
                }
                output
                    .try_reserve(length_usize)
                    .map_err(|_| ReleaseError::OutputTooLarge)?;
                output.extend_from_slice(&source[start..end]);
            }
            OP_INSERT => {
                let length = read_u64(delta, &mut cursor, "insert", operation_offset)?;
                let length_usize =
                    usize::try_from(length).map_err(|_| ReleaseError::OutputTooLarge)?;
                let end = cursor
                    .checked_add(length_usize)
                    .filter(|end| *end <= delta.len())
                    .ok_or(ReleaseError::Truncated {
                        operation: "insert",
                        offset: operation_offset,
                    })?;
                if output
                    .len()
                    .checked_add(length_usize)
                    .is_none_or(|size| size > max_output)
                {
                    return Err(ReleaseError::OutputTooLarge);
                }
                output
                    .try_reserve(length_usize)
                    .map_err(|_| ReleaseError::OutputTooLarge)?;
                output.extend_from_slice(&delta[cursor..end]);
                cursor = end;
            }
            operation => {
                return Err(ReleaseError::UnknownOperation {
                    operation,
                    offset: operation_offset,
                });
            }
        }
    }

    Ok(output)
}

pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn validate_digest(field: &'static str, digest: &str) -> Result<(), ReleaseError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ReleaseError::InvalidDigest { field });
    }
    Ok(())
}

fn validate_size(field: &'static str, size: u64) -> Result<(), ReleaseError> {
    if size == 0 {
        return Err(ReleaseError::EmptySize { field });
    }
    Ok(())
}

fn required(field: &'static str, value: Option<&str>) -> Result<(), ReleaseError> {
    if value.is_none_or(str::is_empty) {
        return Err(ReleaseError::MissingField(field));
    }
    Ok(())
}

fn verify_size(artifact: &'static str, bytes: &[u8], expected: u64) -> Result<(), ReleaseError> {
    let actual = u64::try_from(bytes.len()).map_err(|_| ReleaseError::OutputTooLarge)?;
    if actual != expected {
        return Err(ReleaseError::SizeMismatch {
            artifact,
            actual,
            expected,
        });
    }
    Ok(())
}

fn verify_digest(artifact: &'static str, bytes: &[u8], expected: &str) -> Result<(), ReleaseError> {
    if sha256(bytes) != expected {
        return Err(ReleaseError::DigestMismatch(artifact));
    }
    Ok(())
}

fn read_u64(
    bytes: &[u8],
    cursor: &mut usize,
    operation: &'static str,
    operation_offset: usize,
) -> Result<u64, ReleaseError> {
    let end = cursor
        .checked_add(8)
        .filter(|end| *end <= bytes.len())
        .ok_or(ReleaseError::Truncated {
            operation,
            offset: operation_offset,
        })?;
    let value = u64::from_le_bytes(
        bytes[*cursor..end]
            .try_into()
            .expect("the bounds check guarantees eight bytes"),
    );
    *cursor = end;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert(bytes: &[u8]) -> Vec<u8> {
        let mut delta = vec![OP_INSERT];
        delta.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        delta.extend_from_slice(bytes);
        delta
    }

    fn copy(offset: u64, length: u64) -> Vec<u8> {
        let mut delta = vec![OP_COPY];
        delta.extend_from_slice(&offset.to_le_bytes());
        delta.extend_from_slice(&length.to_le_bytes());
        delta
    }

    fn manifest(source: &[u8], delta: &[u8], output: &[u8]) -> GameReleaseManifest {
        GameReleaseManifest {
            platform: Platform::Ios,
            input_sha256: sha256(source),
            input_bytes: source.len() as u64,
            output_sha256: sha256(output),
            output_bytes: output.len() as u64,
            delta_bytes: delta.len() as u64,
            endpoint: "https://api.kanto.ac".into(),
            schema_version: Some(1),
            release_version: Some("0.1.0".into()),
            published_at: Some("2026-09-01T00:00:00Z".into()),
            minimum_launcher_version: Some("0.1.0".into()),
            delta_sha256: Some(sha256(delta)),
            delta_url: Some("/api/launcher/v1/game/ios/delta".into()),
            source_url: Some("https://archive.example/client.ipa".into()),
        }
    }

    #[test]
    fn applies_and_verifies_the_copy_insert_format() {
        let source = b"hello world";
        let mut delta = copy(0, 6);
        delta.extend(insert(b"brave "));
        delta.extend(copy(6, 5));
        let expected = b"hello brave world";

        let result = apply_verified(source, &delta, &manifest(source, &delta, expected)).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn reads_the_current_unversioned_browser_manifest() {
        let wire = r#"{
            "platform":"android",
            "input_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "input_bytes":63658806,
            "output_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "output_bytes":63660000,
            "delta_bytes":1000,
            "endpoint":"https://api.kanto.ac"
        }"#;

        let parsed: GameReleaseManifest = serde_json::from_str(wire).unwrap();

        assert_eq!(parsed.platform, Platform::Android);
        assert_eq!(parsed.schema_version, None);
        assert_eq!(parsed.validate(), Ok(()));
    }

    #[test]
    fn refuses_a_source_that_does_not_match_the_manifest() {
        let source = b"hello world";
        let delta = insert(b"replacement");
        let expected = b"replacement";
        let release = manifest(source, &delta, expected);

        let error = apply_verified(b"wrong source", &delta, &release).unwrap_err();

        assert!(matches!(
            error,
            ReleaseError::SizeMismatch {
                artifact: "source",
                ..
            }
        ));
    }

    #[test]
    fn refuses_output_larger_than_the_manifest() {
        let source = b"source";
        let delta = insert(b"too large");
        let mut release = manifest(source, &delta, b"too large");
        release.output_bytes = 1;

        assert_eq!(
            apply_verified(source, &delta, &release),
            Err(ReleaseError::OutputTooLarge)
        );
    }

    #[test]
    fn accepts_manual_source_fallback() {
        let source = b"source";
        let delta = insert(b"output");
        let mut release = manifest(source, &delta, b"output");
        release.source_url = None;

        assert_eq!(release.validate(), Ok(()));
    }

    #[test]
    fn refuses_unsafe_source_urls() {
        let source = b"source";
        let delta = insert(b"output");
        for source_url in [
            "http://example.com/game.ipa",
            "https://user:pass@example.com/game.ipa",
        ] {
            let mut release = manifest(source, &delta, b"output");
            release.source_url = Some(source_url.into());
            assert_eq!(release.validate(), Err(ReleaseError::InsecureSourceUrl));
        }
    }

    #[test]
    fn refuses_truncated_delta_operations() {
        assert_eq!(
            apply_delta(b"source", &[OP_COPY, 1, 2]),
            Err(ReleaseError::Truncated {
                operation: "copy",
                offset: 0,
            })
        );
    }
}
