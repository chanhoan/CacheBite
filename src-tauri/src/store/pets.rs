use crate::domain::is_valid_pet_id;
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
    #[serde(default)]
    pub version: u32,
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
            asset_base_url: normalize_asset_root(&root.to_string_lossy()),
        })
    }

    pub fn should_preserve_installed(&self, id: &str, bundled_version: u32) -> bool {
        let Ok(package) = self.load(id) else {
            return false;
        };
        let bundled_name = match id {
            "cat" => "Cat",
            "corgi" => "Corgi",
            _ => return true,
        };
        // A renamed pet is a user customization — never clobber it.
        if package.manifest.display_name != bundled_name {
            return true;
        }
        // Stock pet whose bundled art was refreshed: force a reinstall so
        // updated frame content (e.g. transparent backgrounds) reaches disk
        // even though filenames are unchanged.
        if package.manifest.version < bundled_version {
            return false;
        }
        // Stock and up to date only when frames already use current naming.
        let prefix = format!("frames/{id}_");
        package
            .manifest
            .animations
            .values()
            .flat_map(paths)
            .all(|path| path.starts_with(&prefix))
    }
}

/// Reads only the `version` field from a manifest on disk, tolerant of any
/// other shape. Returns 0 when the file is missing or unparseable so a
/// versionless legacy manifest never spuriously triggers an upgrade.
pub fn bundled_manifest_version(manifest_path: &Path) -> u32 {
    #[derive(Deserialize)]
    struct VersionProbe {
        #[serde(default)]
        version: u32,
    }
    fs::read(manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<VersionProbe>(&bytes).ok())
        .map(|probe| probe.version)
        .unwrap_or(0)
}

fn normalize_asset_root(raw: &str) -> String {
    let normalized = raw.replace('\\', "/");
    let without_verbatim_prefix = normalized.strip_prefix("//?/").unwrap_or(&normalized);
    format!("{}/", without_verbatim_prefix.trim_end_matches('/'))
}

fn paths(animation: &Animation) -> &[String] {
    match animation {
        Animation::Image { source } => std::slice::from_ref(source),
        Animation::Frames { frames, .. } => frames,
    }
}
fn validate_id(id: &str) -> io::Result<()> {
    if !is_valid_pet_id(id) {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid pet id",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod asset_root_tests {
    #[test]
    fn normalizes_windows_verbatim_paths_for_tauri_asset_urls() {
        assert_eq!(
            super::normalize_asset_root(
                r"\\?\C:\Users\chanhoan\AppData\Roaming\dev.cachebite.app\pets\cat"
            ),
            "C:/Users/chanhoan/AppData/Roaming/dev.cachebite.app/pets/cat/"
        );
    }
}
