use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

const MAX_MANIFEST_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PetPackage {
    pub manifest: PetManifest,
    pub asset_base_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PetManifest {
    pub id: String,
    pub display_name: String,
    pub default_size: BTreeMap<String, u32>,
    pub animations: BTreeMap<String, Animation>,
    #[serde(default)]
    pub states: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Animation {
    #[serde(rename = "image")]
    Image { source: String },
    #[serde(rename = "frames", rename_all = "camelCase")]
    Frames {
        frames: Vec<String>,
        frame_duration_ms: u32,
    },
}

pub struct PetPackageRepository {
    pets_root: PathBuf,
}
impl PetPackageRepository {
    pub fn new(app_data: impl AsRef<Path>) -> Self {
        Self {
            pets_root: app_data.as_ref().join("pets"),
        }
    }
    pub fn load(&self, id: &str) -> io::Result<PetPackage> {
        validate_id(id)?;
        let pets = fs::canonicalize(&self.pets_root)?;
        let root = fs::canonicalize(self.pets_root.join(id))?;
        if !root.starts_with(&pets) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "pet package escapes root",
            ));
        }
        let bytes = fs::read(root.join("manifest.json"))?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "pet manifest too large",
            ));
        }
        let manifest: PetManifest = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid pet manifest"))?;
        if manifest.id != id || !manifest.animations.contains_key("idle") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid pet manifest identity",
            ));
        }
        for path in manifest.animations.values().flat_map(paths) {
            if path.starts_with('/') || path.contains("..") || path.contains('\\') {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "invalid asset path",
                ));
            }
            let asset = fs::canonicalize(root.join(path))?;
            if !asset.starts_with(&root) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "asset escapes package",
                ));
            }
        }
        Ok(PetPackage {
            manifest,
            asset_base_url: format!("asset://localhost/pets/{id}/"),
        })
    }
}

fn paths(animation: &Animation) -> Vec<&str> {
    match animation {
        Animation::Image { source } => vec![source],
        Animation::Frames { frames, .. } => frames.iter().map(String::as_str).collect(),
    }
}
fn validate_id(id: &str) -> io::Result<()> {
    let bytes = id.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_lowercase()
        || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
        || bytes
            .iter()
            .any(|byte| !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'-')
    {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid pet id",
        ))
    } else {
        Ok(())
    }
}
