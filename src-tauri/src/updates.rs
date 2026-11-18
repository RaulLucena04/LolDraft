//! Comprobación de actualizaciones de la aplicación.
//!
//! El canal es un `latest.json` publicado junto al instalador: por defecto la
//! última release de GitHub, o una carpeta local o URL que configure el usuario.
//! No hay firma de actualizaciones: si el canal es HTTP, la app abre la descarga
//! en el navegador y es el usuario quien ejecuta el instalador. Nunca descarga y
//! ejecuta un binario de internet por su cuenta.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

pub const MANIFEST_FILE: &str = "latest.json";
/// Canal público por defecto: los assets de la última release de GitHub.
/// `releases/latest/download/<nombre>` sirve cada asset por su nombre.
pub const DEFAULT_CHANNEL: &str =
    "https://github.com/RaulLucena04/LolDraft/releases/latest/download";
/// Un manifiesto es un JSON pequeño; cualquier cosa mayor es sospechosa.
const MAX_MANIFEST_BYTES: usize = 64 * 1024;

/// Contenido de `latest.json`. `installer` es relativo a la carpeta o URL.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateManifest {
    pub version: String,
    pub installer: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// De dónde se leen las actualizaciones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateTarget {
    Local(PathBuf),
    Http(String),
}

impl UpdateTarget {
    pub fn channel(&self) -> &'static str {
        match self {
            Self::Local(_) => "local",
            Self::Http(_) => "url",
        }
    }

    /// Texto para la UI: la carpeta o la URL, sin adornos.
    pub fn describe(&self) -> String {
        match self {
            Self::Local(folder) => folder.display().to_string(),
            Self::Http(url) => url.clone(),
        }
    }

    pub fn manifest_location(&self) -> String {
        match self {
            Self::Local(folder) => folder.join(MANIFEST_FILE).display().to_string(),
            Self::Http(url) => format!("{}/{}", url.trim_end_matches('/'), MANIFEST_FILE),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error(
        "no encuentro `{0}`. Publica una versión con `npm run release` o indica otro origen en la tarjeta de actualizaciones."
    )]
    MissingManifest(String),
    #[error("no se pudo leer la actualización: {0}")]
    Io(String),
    #[error("la respuesta del canal no es válida: {0}")]
    Invalid(String),
    #[error("la versión `{0}` no es válida; usa un formato como 0.2.0")]
    InvalidVersion(String),
    #[error("el manifiesto apunta a un instalador fuera del canal: {0}")]
    OutsideChannel(String),
    #[error("no encuentro el instalador `{0}` en el canal")]
    MissingInstaller(String),
}

pub fn is_http(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Resuelve el canal configurado. Sin origen explícito se usa el canal público
/// de GitHub; una ruta local sirve para probar una compilación antes de
/// publicarla.
pub fn resolve_target(configured: Option<&str>) -> UpdateTarget {
    match configured.map(str::trim).filter(|value| !value.is_empty()) {
        Some(source) if is_http(source) => {
            UpdateTarget::Http(source.trim_end_matches('/').to_string())
        }
        Some(source) => UpdateTarget::Local(PathBuf::from(source)),
        None => UpdateTarget::Http(DEFAULT_CHANNEL.to_string()),
    }
}

/// Compara versiones numéricas (`0.2.0`, `v0.2.0`, `0.2.0-beta`).
///
/// Las etiquetas de preestreno se ignoran: solo se comparan los números, así
/// que `0.2.0-beta` no se considera más nueva que `0.2.0`. La app no publica
/// preestrenos.
pub fn is_newer(latest: &str, current: &str) -> Result<bool, UpdateError> {
    let mut latest = parse_version(latest)?;
    let mut current = parse_version(current)?;
    let width = latest.len().max(current.len());
    latest.resize(width, 0);
    current.resize(width, 0);
    Ok(latest > current)
}

fn parse_version(value: &str) -> Result<Vec<u64>, UpdateError> {
    let core = value.trim().trim_start_matches(['v', 'V']);
    let core = core.split(['-', '+']).next().unwrap_or_default();
    if core.is_empty() {
        return Err(UpdateError::InvalidVersion(value.to_string()));
    }
    core.split('.')
        .map(|part| {
            part.trim()
                .parse::<u64>()
                .map_err(|_| UpdateError::InvalidVersion(value.to_string()))
        })
        .collect()
}

/// Lee el manifiesto del canal. HTTP solo acepta JSON pequeño y con timeout.
pub async fn load(target: &UpdateTarget) -> Result<UpdateManifest, UpdateError> {
    match target {
        UpdateTarget::Local(folder) => {
            let path = folder.join(MANIFEST_FILE);
            let text = std::fs::read_to_string(&path).map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    UpdateError::MissingManifest(path.display().to_string())
                } else {
                    UpdateError::Io(error.to_string())
                }
            })?;
            parse_manifest(&text)
        }
        UpdateTarget::Http(_) => {
            let url = target.manifest_location();
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|error| UpdateError::Io(error.to_string()))?;
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|error| UpdateError::Io(error.to_string()))?;
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(UpdateError::MissingManifest(url));
            }
            if !response.status().is_success() {
                return Err(UpdateError::Io(format!(
                    "el canal respondió con estado {}",
                    response.status().as_u16()
                )));
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|error| UpdateError::Io(error.to_string()))?;
            if bytes.len() > MAX_MANIFEST_BYTES {
                return Err(UpdateError::Invalid(
                    "el manifiesto es demasiado grande".to_string(),
                ));
            }
            let text = String::from_utf8(bytes.to_vec())
                .map_err(|_| UpdateError::Invalid("el manifiesto no es UTF-8".to_string()))?;
            parse_manifest(&text)
        }
    }
}

fn parse_manifest(text: &str) -> Result<UpdateManifest, UpdateError> {
    let manifest: UpdateManifest =
        serde_json::from_str(text).map_err(|error| UpdateError::Invalid(error.to_string()))?;
    if manifest.version.trim().is_empty() || manifest.installer.trim().is_empty() {
        return Err(UpdateError::Invalid(
            "faltan `version` o `installer`".to_string(),
        ));
    }
    Ok(manifest)
}

/// Dónde está el instalador de este manifiesto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installer {
    Local(PathBuf),
    Url(String),
}

impl Installer {
    pub fn describe(&self) -> String {
        match self {
            Self::Local(path) => path.display().to_string(),
            Self::Url(url) => url.clone(),
        }
    }
}

/// Resuelve el instalador rechazando rutas absolutas, `..` y nombres con
/// caracteres que un manifiesto no necesita, para que no pueda apuntar a un
/// ejecutable cualquiera del sistema.
pub fn installer_for(
    target: &UpdateTarget,
    manifest: &UpdateManifest,
) -> Result<Installer, UpdateError> {
    if !is_safe_installer(&manifest.installer) {
        return Err(UpdateError::OutsideChannel(manifest.installer.clone()));
    }

    match target {
        UpdateTarget::Local(folder) => {
            let path = folder.join(&manifest.installer);
            if !path.is_file() {
                return Err(UpdateError::MissingInstaller(manifest.installer.clone()));
            }
            Ok(Installer::Local(path))
        }
        UpdateTarget::Http(base) => Ok(Installer::Url(format!(
            "{}/{}",
            base.trim_end_matches('/'),
            manifest.installer.trim_start_matches('/')
        ))),
    }
}

/// Acepta un nombre (`setup.exe`) o una subcarpeta (`v0.2/setup.exe`) con
/// solo letras, números, punto, guion y guion bajo.
fn is_safe_installer(installer: &str) -> bool {
    let mut segments = 0usize;
    for component in Path::new(installer.trim()).components() {
        let Component::Normal(name) = component else {
            return false;
        };
        let Some(name) = name.to_str() else {
            return false;
        };
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        {
            return false;
        }
        segments += 1;
    }
    (1..=2).contains(&segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(installer: &str) -> UpdateManifest {
        UpdateManifest {
            version: "0.2.0".to_string(),
            installer: installer.to_string(),
            notes: None,
            sha256: None,
        }
    }

    #[test]
    fn compares_numeric_versions() {
        assert!(is_newer("0.2.0", "0.1.0").unwrap());
        assert!(is_newer("v0.10.0", "0.9.9").unwrap());
        assert!(is_newer("1.0", "0.9.9").unwrap());
        assert!(is_newer("0.1.0.1", "0.1.0").unwrap());
        assert!(!is_newer("0.1.0", "0.1.0").unwrap());
        assert!(!is_newer("0.1.0", "0.2.0").unwrap());
        assert!(!is_newer("0.2.0-beta", "0.2.0").unwrap());
    }

    #[test]
    fn rejects_invalid_versions() {
        assert!(matches!(
            is_newer("", "0.1.0"),
            Err(UpdateError::InvalidVersion(_))
        ));
        assert!(matches!(
            is_newer("0.1.x", "0.1.0"),
            Err(UpdateError::InvalidVersion(_))
        ));
    }

    #[test]
    fn defaults_to_the_public_github_channel() {
        let target = resolve_target(None);
        assert_eq!(target, UpdateTarget::Http(DEFAULT_CHANNEL.to_string()));
        assert_eq!(target.channel(), "url");
        assert_eq!(
            target.manifest_location(),
            "https://github.com/RaulLucena04/LolDraft/releases/latest/download/latest.json"
        );

        let blank = resolve_target(Some("   "));
        assert_eq!(blank, target);
    }

    #[test]
    fn resolves_http_and_folder_sources() {
        let url = resolve_target(Some("https://example.test/updates/"));
        assert_eq!(
            url,
            UpdateTarget::Http("https://example.test/updates".to_string())
        );
        assert_eq!(
            url.manifest_location(),
            "https://example.test/updates/latest.json"
        );

        let folder = resolve_target(Some(r"D:\updates"));
        assert_eq!(folder, UpdateTarget::Local(PathBuf::from(r"D:\updates")));
        assert_eq!(folder.channel(), "local");
    }

    #[test]
    fn refuses_installers_outside_the_channel() {
        let local = UpdateTarget::Local(PathBuf::from("C:/updates"));
        for bad in [
            "C:/Windows/notepad.exe",
            "../setup.exe",
            "/setup.exe",
            "a/b/c/d.exe",
            "setup & calc.exe",
        ] {
            assert!(matches!(
                installer_for(&local, &manifest(bad)),
                Err(UpdateError::OutsideChannel(_))
            ));
        }
    }

    #[test]
    fn accepts_plain_installer_names_and_one_subfolder() {
        assert!(is_safe_installer("LolDraft_0.2.0_x64-setup.exe"));
        assert!(is_safe_installer("v0.2/LolDraft_x64-setup.exe"));
        assert!(!is_safe_installer(""));
        assert!(!is_safe_installer("sub/dir/file.exe"));
    }

    #[test]
    fn reports_a_missing_installer() {
        let local = UpdateTarget::Local(PathBuf::from("C:/updates"));
        assert!(matches!(
            installer_for(&local, &manifest("LolDraft_0.2.0_x64-setup.exe")),
            Err(UpdateError::MissingInstaller(_))
        ));
    }

    #[test]
    fn builds_the_installer_url_for_http_channels() {
        let target = UpdateTarget::Http("https://example.test/u".to_string());
        let installer = installer_for(&target, &manifest("setup.exe")).unwrap();
        assert_eq!(
            installer,
            Installer::Url("https://example.test/u/setup.exe".to_string())
        );
    }

    #[test]
    fn reads_a_local_manifest() {
        let dir = std::env::temp_dir().join(format!(
            "loldraft-updates-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(MANIFEST_FILE),
            r#"{"version":"0.3.0","installer":"setup.exe","notes":"prueba"}"#,
        )
        .unwrap();

        let target = UpdateTarget::Local(dir.clone());
        let loaded =
            tauri::async_runtime::block_on(load(&target)).expect("manifiesto local válido");
        assert_eq!(loaded.version, "0.3.0");
        assert_eq!(loaded.notes.as_deref(), Some("prueba"));

        std::fs::write(dir.join("setup.exe"), b"stub").unwrap();
        assert_eq!(
            installer_for(&target, &loaded).unwrap(),
            Installer::Local(dir.join("setup.exe"))
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
