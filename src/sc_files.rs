//! Certificate and key files of a BACnet/SC site, in the forms integrators and vendor tools hand
//! them out (docs/concept-sc.md): PEM or DER, a PEM file with certificate and key together, a key
//! with a password (encrypted PKCS#8), or one PKCS#12 file with all of it. Read into DER; the
//! password opens the file and is not kept.

use pkcs8::EncryptedPrivateKeyInfoRef;
use pkcs8::der::{Decode, Document};
use rustls::pki_types::pem::{PemObject, SectionKind};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// Larger than any certificate file; checked before anything is decoded.
pub const MAX_FILE_BYTES: usize = 1024 * 1024;

const PEM_BEGIN: &[u8] = b"-----BEGIN ";
const ENCRYPTED_KEY_BEGIN: &str = "-----BEGIN ENCRYPTED PRIVATE KEY-----";
const ENCRYPTED_KEY_END: &str = "-----END ENCRYPTED PRIVATE KEY-----";
/// The old OpenSSL form of an encrypted key, which no current tool needs.
const OLD_ENCRYPTED_PEM: &[u8] = b"Proc-Type: 4,ENCRYPTED";

/// What one file holds.
#[derive(Debug, Default)]
pub struct Contents {
    /// Certificates in the order of the file; from PKCS#12 the one of the key first, then its chain.
    pub certificates: Vec<CertificateDer<'static>>,
    /// Self-signed certificates a PKCS#12 file brings beside the chain: the site's CA.
    pub authorities: Vec<CertificateDer<'static>>,
    pub key: Option<PrivateKeyDer<'static>>,
}

/// Why a file gives nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreadable {
    TooLarge,
    /// A PKCS#12 file or an encrypted key, chosen without its password.
    NeedsPassword,
    WrongPassword,
    /// Encrypted the old OpenSSL way (`Proc-Type: 4,ENCRYPTED`).
    OldEncryptedPem,
    /// No certificate and no key in it.
    Nothing,
}

impl Unreadable {
    /// The code the interface translates by.
    pub fn code(self) -> &'static str {
        match self {
            Unreadable::TooLarge => "too_large",
            Unreadable::NeedsPassword => "needs_password",
            Unreadable::WrongPassword => "wrong_password",
            Unreadable::OldEncryptedPem => "old_encrypted_pem",
            Unreadable::Nothing => "nothing",
        }
    }
}

/// Reads a file of certificates, a key, or both; `password` for a PKCS#12 file or an encrypted key.
pub fn read(bytes: &[u8], password: Option<&str>) -> Result<Contents, Unreadable> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(Unreadable::TooLarge);
    }
    let contents = if bytes.windows(PEM_BEGIN.len()).any(|part| part == PEM_BEGIN) {
        read_pem(bytes, password)?
    } else {
        read_der(bytes, password)?
    };
    if contents.certificates.is_empty() && contents.authorities.is_empty() && contents.key.is_none()
    {
        return Err(Unreadable::Nothing);
    }
    Ok(contents)
}

fn read_pem(bytes: &[u8], password: Option<&str>) -> Result<Contents, Unreadable> {
    if bytes
        .windows(OLD_ENCRYPTED_PEM.len())
        .any(|part| part == OLD_ENCRYPTED_PEM)
    {
        return Err(Unreadable::OldEncryptedPem);
    }
    let mut contents = Contents::default();
    for section in <(SectionKind, Vec<u8>)>::pem_slice_iter(bytes) {
        let (kind, der) = section.map_err(|_| Unreadable::Nothing)?;
        match kind {
            SectionKind::Certificate => contents.certificates.push(CertificateDer::from(der)),
            SectionKind::PrivateKey => contents.key = Some(PrivateKeyDer::Pkcs8(der.into())),
            SectionKind::RsaPrivateKey => contents.key = Some(PrivateKeyDer::Pkcs1(der.into())),
            SectionKind::EcPrivateKey => contents.key = Some(PrivateKeyDer::Sec1(der.into())),
            _ => {}
        }
    }
    // not a section the PEM reader knows
    if let Some(encrypted) = encrypted_key_der(bytes) {
        contents.key = Some(decrypt_key(&encrypted, password)?);
    }
    Ok(contents)
}

/// The DER of an `ENCRYPTED PRIVATE KEY` section, if the file has one.
fn encrypted_key_der(bytes: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let start = text.find(ENCRYPTED_KEY_BEGIN)?;
    let end = text[start..].find(ENCRYPTED_KEY_END)? + start + ENCRYPTED_KEY_END.len();
    let (_, document) = Document::from_pem(&text[start..end]).ok()?;
    Some(document.as_bytes().to_vec())
}

fn decrypt_key(der: &[u8], password: Option<&str>) -> Result<PrivateKeyDer<'static>, Unreadable> {
    let encrypted = EncryptedPrivateKeyInfoRef::from_der(der).map_err(|_| Unreadable::Nothing)?;
    let password = password.ok_or(Unreadable::NeedsPassword)?;
    let key = encrypted
        .decrypt(password)
        .map_err(|_| Unreadable::WrongPassword)?;
    Ok(PrivateKeyDer::Pkcs8(key.as_bytes().to_vec().into()))
}

fn read_der(bytes: &[u8], password: Option<&str>) -> Result<Contents, Unreadable> {
    if p12_keystore::Certificate::from_der(bytes).is_ok() {
        return Ok(Contents {
            certificates: vec![CertificateDer::from(bytes.to_vec())],
            ..Contents::default()
        });
    }
    if EncryptedPrivateKeyInfoRef::from_der(bytes).is_ok() {
        return Ok(Contents {
            key: Some(decrypt_key(bytes, password)?),
            ..Contents::default()
        });
    }
    match read_pkcs12(bytes, password) {
        Err(Unreadable::Nothing) => {}
        read => return read,
    }
    let key = PrivateKeyDer::try_from(bytes.to_vec()).map_err(|_| Unreadable::Nothing)?;
    Ok(Contents {
        key: Some(key),
        ..Contents::default()
    })
}

/// A PKCS#12 file: the key, its certificate with the chain, and the CA beside them. Many files
/// have an empty password; that is tried when none is given.
fn read_pkcs12(bytes: &[u8], password: Option<&str>) -> Result<Contents, Unreadable> {
    use p12_keystore::error::Error;
    let archive = match p12_keystore::Pkcs12Archive::from_pkcs12(bytes, password.unwrap_or("")) {
        Ok(archive) => archive,
        Err(Error::DerError(_) | Error::InvalidVersion | Error::UnsupportedContentType) => {
            return Err(Unreadable::Nothing);
        }
        Err(_) if password.is_none() => return Err(Unreadable::NeedsPassword),
        Err(_) => return Err(Unreadable::WrongPassword),
    };
    let key = archive.keys.first();
    let own = key.and_then(|key| {
        let id = key.local_key_id.as_ref()?;
        archive
            .certs
            .iter()
            .position(|bag| bag.local_key_id.as_deref() == Some(id.0.as_slice()))
    });
    let mut contents = Contents {
        key: key.map(|key| PrivateKeyDer::Pkcs8(key.key.as_der().to_vec().into())),
        ..Contents::default()
    };
    if let Some(own) = own {
        contents.certificates.push(CertificateDer::from(
            archive.certs[own].cert.as_der().to_vec(),
        ));
    }
    for (index, bag) in archive.certs.iter().enumerate() {
        if Some(index) == own {
            continue;
        }
        let der = CertificateDer::from(bag.cert.as_der().to_vec());
        if bag.cert.subject() == bag.cert.issuer() {
            contents.authorities.push(der);
        } else {
            contents.certificates.push(der);
        }
    }
    Ok(contents)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rcgen::{CertificateParams, Issuer, KeyPair};

    /// A CA and a certificate for bacnetio signed by it, with its key, as files would hold them.
    pub(crate) struct Made {
        pub ca_pem: String,
        pub ca_der: Vec<u8>,
        pub own_pem: String,
        pub own_der: Vec<u8>,
        pub key_pem: String,
        pub key_der: Vec<u8>,
    }

    pub(crate) fn made() -> Made {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "Site CA");
        let ca_key = KeyPair::generate().unwrap();
        let ca = params.self_signed(&ca_key).unwrap();
        let issuer = Issuer::new(params, ca_key);
        let mut own_params = CertificateParams::new(vec!["bacnetio".into()]).unwrap();
        own_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "bacnetio");
        let key = KeyPair::generate().unwrap();
        let own = own_params.signed_by(&key, &issuer).unwrap();
        Made {
            ca_pem: ca.pem(),
            ca_der: ca.der().to_vec(),
            own_pem: own.pem(),
            own_der: own.der().to_vec(),
            key_pem: key.serialize_pem(),
            key_der: key.serialize_der(),
        }
    }

    #[test]
    fn certificates_and_keys_as_pem_or_der() {
        let made = made();
        let ca = read(made.ca_pem.as_bytes(), None).unwrap();
        assert_eq!(ca.certificates[0].as_ref(), made.ca_der);
        assert!(ca.key.is_none());
        let ca = read(&made.ca_der, None).unwrap();
        assert_eq!(ca.certificates[0].as_ref(), made.ca_der);
        let key = read(made.key_pem.as_bytes(), None).unwrap();
        assert_eq!(key.key.unwrap().secret_der(), made.key_der);
        let key = read(&made.key_der, None).unwrap();
        assert_eq!(key.key.unwrap().secret_der(), made.key_der);
        // certificate and key in one PEM file
        let both = read(format!("{}{}", made.own_pem, made.key_pem).as_bytes(), None).unwrap();
        assert_eq!(both.certificates[0].as_ref(), made.own_der);
        assert!(both.key.is_some());
    }

    #[test]
    fn an_encrypted_key_needs_its_password() {
        let made = made();
        let info = pkcs8::PrivateKeyInfoRef::from_der(&made.key_der).unwrap();
        let encrypted = info.encrypt("geheim").unwrap();
        let pem = encrypted
            .to_pem("ENCRYPTED PRIVATE KEY", pkcs8::LineEnding::LF)
            .unwrap();
        assert_eq!(
            read(pem.as_bytes(), None).unwrap_err(),
            Unreadable::NeedsPassword
        );
        assert_eq!(
            read(pem.as_bytes(), Some("falsch")).unwrap_err(),
            Unreadable::WrongPassword
        );
        let key = read(pem.as_bytes(), Some("geheim")).unwrap().key.unwrap();
        assert_eq!(key.secret_der(), made.key_der);
        let key = read(encrypted.as_bytes(), Some("geheim"))
            .unwrap()
            .key
            .unwrap();
        assert_eq!(key.secret_der(), made.key_der);
    }

    #[test]
    fn a_pkcs12_file_holds_key_certificate_and_ca() {
        let made = made();
        let own = p12_keystore::Certificate::from_der(&made.own_der).unwrap();
        let ca = p12_keystore::Certificate::from_der(&made.ca_der).unwrap();
        let key = p12_keystore::PrivateKey::from_der(&made.key_der).unwrap();
        let chain = p12_keystore::PrivateKeyChain::new(b"1".to_vec(), key, [own, ca]);
        let mut store = p12_keystore::KeyStore::new();
        store.add_entry(
            "bacnetio",
            p12_keystore::KeyStoreEntry::PrivateKeyChain(chain),
        );
        let file = store.writer("geheim").write().unwrap();
        // the old form older Windows and many vendor tools still write
        let old = store
            .writer("geheim")
            .encryption_algorithm(p12_keystore::EncryptionAlgorithm::PbeWithShaAnd3KeyTripleDesCbc)
            .mac_algorithm(p12_keystore::MacAlgorithm::HmacSha1)
            .write()
            .unwrap();
        let old = read(&old, Some("geheim")).unwrap();
        assert_eq!(old.key.unwrap().secret_der(), made.key_der);

        assert_eq!(read(&file, None).unwrap_err(), Unreadable::NeedsPassword);
        assert_eq!(
            read(&file, Some("falsch")).unwrap_err(),
            Unreadable::WrongPassword
        );
        let contents = read(&file, Some("geheim")).unwrap();
        assert_eq!(contents.certificates.len(), 1);
        assert_eq!(contents.certificates[0].as_ref(), made.own_der);
        assert_eq!(contents.authorities[0].as_ref(), made.ca_der);
        assert_eq!(contents.key.unwrap().secret_der(), made.key_der);
    }

    #[test]
    fn what_is_no_certificate_says_so() {
        assert_eq!(read(b"hello", None).unwrap_err(), Unreadable::Nothing);
        assert_eq!(
            read(
                b"-----BEGIN PUBLIC KEY-----\n-----END PUBLIC KEY-----\n",
                None
            )
            .unwrap_err(),
            Unreadable::Nothing
        );
        let old = b"-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\n";
        assert_eq!(read(old, None).unwrap_err(), Unreadable::OldEncryptedPem);
        assert_eq!(
            read(&vec![0; MAX_FILE_BYTES + 1], None).unwrap_err(),
            Unreadable::TooLarge
        );
    }
}
