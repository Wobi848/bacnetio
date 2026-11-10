//! bacnetio's own identity on a BACnet/SC network (docs/concept-sc.md): a VMAC and a device UUID,
//! made once at random and kept in the data folder, so the hub knows bacnetio again after a
//! restart. The standard asks for a device UUID that never changes.

use std::fs;
use std::io;
use std::path::Path;

use bacnet_transport::sc::generate_random48_vmac;
use serde::{Deserialize, Serialize};

/// Below the data folder; the certificates and the key join it later.
pub const SC_DIR: &str = "sc";
const IDENTITY_FILE: &str = "identity.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub vmac: [u8; 6],
    pub uuid: [u8; 16],
}

impl Identity {
    /// A fresh identity: a Random-48 VMAC (Clause H.7.3) and a random UUID (RFC 4122 version 4).
    pub fn new() -> io::Result<Identity> {
        let vmac = generate_random48_vmac().map_err(io::Error::other)?;
        let mut uuid = [0; 16];
        getrandom::fill(&mut uuid).map_err(io::Error::other)?;
        uuid[6] = (uuid[6] & 0x0F) | 0x40;
        uuid[8] = (uuid[8] & 0x3F) | 0x80;
        Ok(Identity { vmac, uuid })
    }

    /// The identity kept in `data_dir`, made and kept there the first time.
    pub fn kept_in(data_dir: &Path) -> io::Result<Identity> {
        let path = data_dir.join(SC_DIR).join(IDENTITY_FILE);
        match fs::read(&path) {
            Ok(json) => serde_json::from_slice(&json).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let identity = Identity::new()?;
                fs::create_dir_all(data_dir.join(SC_DIR))?;
                let temporary = path.with_extension("json.new");
                fs::write(&temporary, serde_json::to_vec_pretty(&identity)?)?;
                fs::rename(temporary, &path)?;
                Ok(identity)
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bacnetio-sc-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn made_once_and_the_same_after_a_restart() {
        let dir = temporary("identity");
        let first = Identity::kept_in(&dir).unwrap();
        assert_eq!(Identity::kept_in(&dir).unwrap(), first);
        // a Random-48 VMAC and a version 4 UUID
        assert_eq!(first.vmac[0] & 0x0F, 0x02);
        assert_eq!(first.uuid[6] >> 4, 4);
        assert_eq!(first.uuid[8] >> 6, 0b10);
        assert_ne!(Identity::new().unwrap(), first);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_broken_file_is_an_error_and_not_replaced() {
        let dir = temporary("broken");
        fs::create_dir_all(dir.join(SC_DIR)).unwrap();
        fs::write(dir.join(SC_DIR).join(IDENTITY_FILE), "{").unwrap();
        assert!(Identity::kept_in(&dir).is_err());
        assert_eq!(
            fs::read(dir.join(SC_DIR).join(IDENTITY_FILE)).unwrap(),
            b"{"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
