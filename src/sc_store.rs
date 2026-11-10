//! An SC site's files kept in bacnetio's data folder (docs/concept-sc.md, *Keys and
//! certificates*): the certificates as they are, they are not secret; the private key readable only
//! for the user, on Windows encrypted for that user by the system (DPAPI). As PEM, so that
//! [`crate::sc_certificates::Certificates::choose`] reads them back like a chosen file.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::sc_certificates::Line;
use crate::sc_identity::SC_DIR;

/// Which file each line came from, to show its name again.
const NAMES_FILE: &str = "files.json";
/// The hub last connected to, offered again.
const HUB_FILE: &str = "hub.txt";

fn file_of(line: Line) -> &'static str {
    match line {
        Line::Ca => "ca.pem",
        Line::Own => "own.pem",
        // encrypted on Windows, so not PEM there
        Line::Key if cfg!(windows) => "key.protected",
        Line::Key => "key.pem",
    }
}

const LINES: [Line; 3] = [Line::Ca, Line::Own, Line::Key];

/// Keeps `pem`, chosen from the file `name`, for `line`.
pub fn keep(data_dir: &Path, line: Line, name: &str, pem: &[u8]) -> io::Result<()> {
    let dir = data_dir.join(SC_DIR);
    fs::create_dir_all(&dir)?;
    match line {
        Line::Key => write_protected(&dir.join(file_of(line)), pem)?,
        _ => write_replacing(&dir.join(file_of(line)), pem)?,
    }
    let mut names = names(&dir)?;
    names.insert(line_key(line).to_string(), name.to_string());
    write_replacing(&dir.join(NAMES_FILE), &serde_json::to_vec_pretty(&names)?)
}

/// What is kept, line by line, with the name of the file each came from.
pub fn kept(data_dir: &Path) -> io::Result<Vec<(Line, String, Vec<u8>)>> {
    let dir = data_dir.join(SC_DIR);
    let names = names(&dir)?;
    let mut found = Vec::new();
    for line in LINES {
        let path = dir.join(file_of(line));
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let bytes = match line {
            Line::Key => unprotect(bytes)?,
            _ => bytes,
        };
        let name = names.get(line_key(line)).cloned().unwrap_or_default();
        found.push((line, name, bytes));
    }
    Ok(found)
}

/// Keeps the address of the hub the files connected to.
pub fn keep_hub(data_dir: &Path, hub: &str) -> io::Result<()> {
    let dir = data_dir.join(SC_DIR);
    fs::create_dir_all(&dir)?;
    write_replacing(&dir.join(HUB_FILE), hub.as_bytes())
}

/// The hub kept last, if any.
pub fn kept_hub(data_dir: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(data_dir.join(SC_DIR).join(HUB_FILE)) {
        Ok(hub) => Ok(Some(hub.trim().to_string()).filter(|hub| !hub.is_empty())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Whether a key is kept.
pub fn key_kept(data_dir: &Path) -> bool {
    data_dir.join(SC_DIR).join(file_of(Line::Key)).exists()
}

/// Deletes the kept key; the certificates stay.
pub fn forget_key(data_dir: &Path) -> io::Result<()> {
    match fs::remove_file(data_dir.join(SC_DIR).join(file_of(Line::Key))) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn line_key(line: Line) -> &'static str {
    match line {
        Line::Ca => "ca",
        Line::Own => "own",
        Line::Key => "key",
    }
}

fn names(dir: &Path) -> io::Result<BTreeMap<String, String>> {
    match fs::read(dir.join(NAMES_FILE)) {
        Ok(json) => serde_json::from_slice(&json).map_err(io::Error::other),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(error) => Err(error),
    }
}

fn temporary(path: &Path) -> PathBuf {
    path.with_extension("new")
}

fn write_replacing(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = temporary(path);
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

/// Readable only for the user who runs bacnetio, from the moment the file exists.
#[cfg(unix)]
fn write_protected(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let temporary = temporary(path);
    let _ = fs::remove_file(&temporary);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?
        .write_all(bytes)?;
    fs::rename(temporary, path)
}

#[cfg(unix)]
fn unprotect(bytes: Vec<u8>) -> io::Result<Vec<u8>> {
    Ok(bytes)
}

#[cfg(windows)]
fn write_protected(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_replacing(path, &dpapi::protect(bytes)?)
}

#[cfg(windows)]
fn unprotect(bytes: Vec<u8>) -> io::Result<Vec<u8>> {
    dpapi::unprotect(&bytes)
}

/// The system's data protection: only the same user on the same computer opens it again.
#[cfg(windows)]
mod dpapi {
    use std::io;
    use std::ptr;

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };

    pub fn protect(bytes: &[u8]) -> io::Result<Vec<u8>> {
        run(bytes, |input, output| unsafe {
            CryptProtectData(
                input,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                output,
            )
        })
    }

    pub fn unprotect(bytes: &[u8]) -> io::Result<Vec<u8>> {
        run(bytes, |input, output| unsafe {
            CryptUnprotectData(
                input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                output,
            )
        })
    }

    fn run(
        bytes: &[u8],
        call: impl FnOnce(*const CRYPT_INTEGER_BLOB, *mut CRYPT_INTEGER_BLOB) -> i32,
    ) -> io::Result<Vec<u8>> {
        let length = u32::try_from(bytes.len()).map_err(io::Error::other)?;
        let input = CRYPT_INTEGER_BLOB {
            cbData: length,
            pbData: bytes.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        if call(&input, &mut output) == 0 {
            return Err(io::Error::last_os_error());
        }
        if output.pbData.is_null() || output.cbData == 0 {
            return Ok(Vec::new());
        }
        // SAFETY: on success the system hands over a buffer of cbData bytes, freed with LocalFree
        let result =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        unsafe { LocalFree(output.pbData.cast()) };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bacnetio-sc-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn kept_and_read_back_with_their_names() {
        let dir = temporary_dir("kept");
        assert!(kept(&dir).unwrap().is_empty());
        keep(&dir, Line::Ca, "ca.pem", b"CA").unwrap();
        keep(&dir, Line::Key, "bacnetio.key", b"KEY").unwrap();
        assert_eq!(
            kept(&dir).unwrap(),
            vec![
                (Line::Ca, "ca.pem".to_string(), b"CA".to_vec()),
                (Line::Key, "bacnetio.key".to_string(), b"KEY".to_vec()),
            ]
        );
        // the key is not on the disk as it came
        let on_disk = fs::read(dir.join(SC_DIR).join(file_of(Line::Key))).unwrap();
        #[cfg(windows)]
        assert_ne!(on_disk, b"KEY");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join(SC_DIR).join(file_of(Line::Key)))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
            assert_eq!(on_disk, b"KEY");
        }
        assert_eq!(kept_hub(&dir).unwrap(), None);
        keep_hub(&dir, "wss://hub.example:443/").unwrap();
        assert_eq!(
            kept_hub(&dir).unwrap().as_deref(),
            Some("wss://hub.example:443/")
        );
        forget_key(&dir).unwrap();
        forget_key(&dir).unwrap();
        assert_eq!(kept(&dir).unwrap().len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
