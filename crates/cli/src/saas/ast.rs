//! Remote SaaS-compiled AST download and signature verification boundary.

use std::path::{Path, PathBuf};

use controlpath_compiler::ast::Artifact;
use controlpath_compiler::catalog::CatalogDocument;
use controlpath_compiler::serialize;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::error::{CliError, CliResult};
use crate::saas::client::RemoteAstArtifact;
use crate::utils::atomic_write::atomic_write;
use controlpath_compiler::{environment_from_ast_path, is_valid_saas_environment_name};

fn invalid_environment_name_error(environment: &str) -> CliError {
    CliError::Message(format!(
        "Invalid environment name for remote AST: '{environment}'"
    ))
}

fn remote_ast_output_path(base_dir: &Path, environment: &str) -> CliResult<PathBuf> {
    if !is_valid_saas_environment_name(environment) {
        return Err(invalid_environment_name_error(environment));
    }

    Ok(base_dir.join(format!(".controlpath/{environment}.ast")))
}

/// Options for verifying downloaded SaaS-compiled AST artifacts.
#[derive(Debug, Clone, Default)]
pub struct RemoteAstOptions {
    pub public_key: Option<Vec<u8>>,
    pub require_signature: bool,
}

/// Build AST verification options from a validated SaaS-mode catalog.
pub fn remote_ast_options_from_catalog(catalog: &CatalogDocument) -> CliResult<RemoteAstOptions> {
    let Some(saas) = catalog.saas.as_ref() else {
        return Ok(RemoteAstOptions::default());
    };

    let public_key = match &saas.ast_public_key {
        Some(encoded) => Some(decode_public_key(encoded)?),
        None => None,
    };

    let options = RemoteAstOptions {
        public_key,
        require_signature: saas.require_ast_signature,
    };
    ensure_signature_config(&options)?;
    Ok(options)
}

fn ensure_signature_config(options: &RemoteAstOptions) -> CliResult<()> {
    if options.require_signature && options.public_key.is_none() {
        return Err(CliError::Message(
            "saas.ast_public_key is required when require_ast_signature is true".to_string(),
        ));
    }
    Ok(())
}

fn decode_public_key(encoded: &str) -> CliResult<Vec<u8>> {
    use base64::Engine;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|e| CliError::Message(format!("Invalid saas.ast_public_key base64: {e}")))?;

    if bytes.len() != 32 {
        return Err(CliError::Message(format!(
            "Invalid saas.ast_public_key length: expected 32 bytes, got {}",
            bytes.len()
        )));
    }

    Ok(bytes)
}

/// Deserialize and optionally verify a remote AST artifact.
pub fn process_remote_ast(bytes: &[u8], options: &RemoteAstOptions) -> CliResult<Artifact> {
    ensure_signature_config(options)?;

    let artifact: Artifact = rmp_serde::from_slice(bytes).map_err(|e| {
        CliError::Message(format!("Failed to deserialize remote AST artifact: {e}"))
    })?;

    if options.require_signature {
        let public_key = options
            .public_key
            .as_ref()
            .expect("checked by ensure_signature_config");
        verify_artifact_signature(artifact, public_key, true)
    } else if let Some(public_key) = &options.public_key {
        verify_artifact_signature(artifact, public_key, false)
    } else {
        Ok(artifact)
    }
}

/// Write downloaded remote AST artifacts into `.controlpath/<env>.ast`.
pub fn write_remote_asts(
    base_dir: &Path,
    artifacts: &[RemoteAstArtifact],
    options: &RemoteAstOptions,
) -> CliResult<Vec<String>> {
    ensure_signature_config(options)?;

    std::fs::create_dir_all(base_dir.join(".controlpath"))
        .map_err(|e| CliError::Message(format!("Failed to create .controlpath directory: {e}")))?;

    let mut written = Vec::new();
    for artifact in artifacts {
        process_remote_ast(&artifact.bytes, options)?;
        let output_path = remote_ast_output_path(base_dir, &artifact.environment)?;
        atomic_write(&output_path, &artifact.bytes).map_err(|e| {
            CliError::Message(format!(
                "Failed to write remote AST for {}: {e}",
                artifact.environment
            ))
        })?;
        written.push(artifact.environment.clone());
    }

    prune_stale_remote_asts(base_dir, &written)?;

    Ok(written)
}

fn prune_stale_remote_asts(base_dir: &Path, active_envs: &[String]) -> CliResult<()> {
    use std::collections::BTreeSet;

    let controlpath_dir = base_dir.join(".controlpath");
    if !controlpath_dir.is_dir() {
        return Ok(());
    }

    let active: BTreeSet<&str> = active_envs.iter().map(String::as_str).collect();
    for entry in std::fs::read_dir(&controlpath_dir)
        .map_err(|e| CliError::Message(format!("Failed to read .controlpath directory: {e}")))?
    {
        let entry =
            entry.map_err(|e| CliError::Message(format!("Failed to read directory entry: {e}")))?;
        let path = entry.path();
        // Skip invalid `*.ast` junk (e.g. `..ast`); discovery also ignores them — manual cleanup only.
        let Some(env) = environment_from_ast_path(&path) else {
            continue;
        };
        if !active.contains(env.as_str()) {
            std::fs::remove_file(&path).map_err(|e| {
                CliError::Message(format!("Failed to remove stale remote AST {env}: {e}"))
            })?;
        }
    }

    Ok(())
}

fn verify_artifact_signature(
    artifact: Artifact,
    public_key: &[u8],
    require_signature: bool,
) -> CliResult<Artifact> {
    let signature_bytes = match &artifact.signature {
        Some(sig) => sig.as_slice(),
        None => {
            if require_signature {
                return Err(CliError::Message(
                    "Signature required but not present in artifact".to_string(),
                ));
            }
            return Ok(artifact);
        }
    };

    if signature_bytes.len() != 64 {
        return Err(CliError::Message(format!(
            "Invalid signature length: expected 64 bytes, got {}",
            signature_bytes.len()
        )));
    }

    if public_key.len() != 32 {
        return Err(CliError::Message(format!(
            "Invalid public key length: expected 32 bytes, got {}",
            public_key.len()
        )));
    }

    let verifying_key = VerifyingKey::from_bytes(
        public_key
            .try_into()
            .map_err(|_| CliError::Message("Invalid public key format".to_string()))?,
    )
    .map_err(|e| CliError::Message(format!("Invalid public key: {e}")))?;

    let mut unsigned = artifact.clone();
    unsigned.signature = None;
    let message_bytes = serialize(&unsigned).map_err(CliError::from)?;

    let signature = Signature::from_bytes(
        signature_bytes
            .try_into()
            .map_err(|_| CliError::Message("Invalid signature format".to_string()))?,
    );

    verifying_key
        .verify(&message_bytes, &signature)
        .map_err(|_| CliError::Message("Signature verification failed".to_string()))?;

    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use controlpath_compiler::ast::Artifact;
    use ed25519_dalek::{Signer, SigningKey};

    fn sample_artifact() -> Artifact {
        Artifact {
            version: "1.0".to_string(),
            environment: "production".to_string(),
            string_table: vec!["flag_a".to_string()],
            flags: vec![vec![]],
            flag_names: vec![0],
            segments: None,
            signature: None,
        }
    }

    fn sign_artifact(artifact: &Artifact, signing_key: &SigningKey) -> Artifact {
        let mut unsigned = artifact.clone();
        unsigned.signature = None;
        let message = serialize(&unsigned).unwrap();
        let signature = signing_key.sign(&message).to_bytes().to_vec();
        let mut signed = unsigned;
        signed.signature = Some(signature);
        signed
    }

    #[test]
    fn accepts_unsigned_artifact_when_signature_not_required() {
        let bytes = serialize(&sample_artifact()).unwrap();
        let artifact = process_remote_ast(&bytes, &RemoteAstOptions::default()).unwrap();
        assert_eq!(artifact.environment, "production");
    }

    #[test]
    fn rejects_unsigned_artifact_when_signature_required() {
        let bytes = serialize(&sample_artifact()).unwrap();
        let err = process_remote_ast(
            &bytes,
            &RemoteAstOptions {
                require_signature: true,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("ast_public_key is required"));
    }

    #[test]
    fn rejects_bogus_signature_when_signature_required() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let mut signed = sign_artifact(&sample_artifact(), &signing_key);
        signed.signature = Some([0u8; 64].to_vec());
        let bytes = serialize(&signed).unwrap();

        let err = process_remote_ast(
            &bytes,
            &RemoteAstOptions {
                public_key: Some(verifying_key.to_bytes().to_vec()),
                require_signature: true,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("Signature verification failed"));
    }

    #[test]
    fn removes_stale_remote_ast_files() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        std::fs::create_dir_all(base.join(".controlpath")).unwrap();
        std::fs::write(base.join(".controlpath/staging.ast"), b"old").unwrap();
        std::fs::write(base.join(".controlpath/saas-fake-state.json"), b"{}").unwrap();

        let bytes = serialize(&sample_artifact()).unwrap();
        write_remote_asts(
            base,
            &[RemoteAstArtifact {
                environment: "production".to_string(),
                bytes,
            }],
            &RemoteAstOptions::default(),
        )
        .unwrap();

        assert!(base.join(".controlpath/production.ast").exists());
        assert!(!base.join(".controlpath/staging.ast").exists());
        assert!(base.join(".controlpath/saas-fake-state.json").exists());
    }

    #[test]
    fn verifies_signed_artifact_with_public_key() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let signed = sign_artifact(&sample_artifact(), &signing_key);
        let bytes = serialize(&signed).unwrap();

        let artifact = process_remote_ast(
            &bytes,
            &RemoteAstOptions {
                public_key: Some(verifying_key.to_bytes().to_vec()),
                require_signature: true,
            },
        )
        .unwrap();

        assert_eq!(artifact.environment, "production");
    }

    #[test]
    fn remote_ast_options_from_catalog_reads_saas_config() {
        use controlpath_compiler::catalog::{
            CatalogDocument, CatalogIdentity, CatalogMode, SaasConfig,
        };
        use ed25519_dalek::SigningKey;

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            signing_key.verifying_key().to_bytes(),
        );

        let catalog = CatalogDocument {
            catalog: CatalogIdentity {
                id: "svc".to_string(),
                namespace: None,
            },
            mode: CatalogMode::Saas,
            saas: Some(SaasConfig {
                project: "acme/svc".to_string(),
                api_url: None,
                cdn_url: None,
                ast_public_key: Some(encoded),
                require_ast_signature: true,
            }),
            imports: Default::default(),
            flags: Default::default(),
            environments: Default::default(),
            segments: Default::default(),
            kill_switches: Default::default(),
            artifacts: Default::default(),
        };

        let options = remote_ast_options_from_catalog(&catalog).unwrap();
        assert!(options.require_signature);
        assert_eq!(options.public_key.as_ref().map(|k| k.len()), Some(32));
    }

    #[test]
    fn rejects_unsafe_environment_names() {
        let bytes = serialize(&sample_artifact()).unwrap();
        for environment in ["../outside", ".", ".."] {
            let err = write_remote_asts(
                Path::new("/tmp/test"),
                &[RemoteAstArtifact {
                    environment: environment.to_string(),
                    bytes: bytes.clone(),
                }],
                &RemoteAstOptions::default(),
            )
            .unwrap_err();

            assert!(
                err.to_string().contains("Invalid environment name"),
                "expected rejection for {environment:?}, got {err}"
            );
        }
    }

    #[test]
    fn rejects_invalid_signature() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let other_key = SigningKey::from_bytes(&[9u8; 32]);
        let mut signed = sign_artifact(&sample_artifact(), &signing_key);
        signed.signature = Some([0u8; 64].to_vec());
        let bytes = serialize(&signed).unwrap();

        let err = process_remote_ast(
            &bytes,
            &RemoteAstOptions {
                public_key: Some(other_key.verifying_key().to_bytes().to_vec()),
                require_signature: true,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("Signature verification failed"));
    }
}
