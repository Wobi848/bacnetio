//! The certificates of a BACnet/SC site as the search settings show them (docs/concept-sc.md):
//! three lines, the site's CA, bacnetio's own certificate and its private key, each filled from a
//! file and said in words: valid until, expired, not signed by this CA, does not match.

use bacnet_transport::sc_tls::ScNodeTlsConfig;
use bacnet_transport::sc_tls::inspection::{
    ScDirectIssuer, ScExpiry, ScInputRole, ScInspectedCertificate, ScKeyConsistency, ScValidity,
    inspect_sc_pem,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};

use crate::sc_files::{self, Contents, Unreadable};

const DAY_S: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Line {
    Ca,
    Own,
    Key,
}

/// One line: empty, filled from a file, or a file that gave nothing for it.
#[derive(Debug, Default)]
enum Slot<T> {
    #[default]
    Empty,
    Filled {
        file: String,
        value: T,
    },
    Failed {
        file: String,
        reason: &'static str,
        /// Kept while a password is asked for, to open the file with it; never on disk.
        waiting: Option<Vec<u8>>,
    },
}

impl<T> Slot<T> {
    fn value(&self) -> Option<&T> {
        match self {
            Slot::Filled { value, .. } => Some(value),
            _ => None,
        }
    }

    fn file(&self) -> Option<String> {
        match self {
            Slot::Empty => None,
            Slot::Filled { file, .. } | Slot::Failed { file, .. } => Some(file.clone()),
        }
    }
}

/// The three lines.
#[derive(Debug, Default)]
pub struct Certificates {
    ca: Slot<Vec<CertificateDer<'static>>>,
    own: Slot<Vec<CertificateDer<'static>>>,
    key: Slot<PrivateKeyDer<'static>>,
    /// The key came with a password: kept on disk only when the user asks.
    key_had_password: bool,
}

/// One line in words: the file, a state the interface translates, and the dates it speaks of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineView {
    pub file: Option<String>,
    /// `missing`, `valid`, `expires_soon`, `expired`, `not_yet_valid`, `not_from_ca`, `matches`,
    /// `mismatch`, `unusable`, `chosen`, or why the file gave nothing (`needs_password`,
    /// `wrong_password`, `old_encrypted_pem`, `too_large`, `nothing`, `no_key`, `no_certificate`).
    pub state: &'static str,
    /// Unix seconds.
    pub from: Option<i64>,
    pub until: Option<i64>,
    /// Days left, for `expires_soon`.
    pub days: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct View {
    pub ca: LineView,
    pub own: LineView,
    pub key: LineView,
    /// All three filled and the key fits: bacnetio can try the hub.
    pub ready: bool,
}

impl Certificates {
    /// Takes a file chosen for `line`. A file with more in it fills the other lines too: a PKCS#12
    /// file all three, a PEM file with certificate and key both of those.
    pub fn choose(&mut self, line: Line, file: &str, bytes: Vec<u8>, password: Option<&str>) {
        match sc_files::read(&bytes, password) {
            Ok(contents) => {
                if contents.key.is_some() {
                    self.key_had_password = password.is_some();
                }
                self.place(line, file, contents);
            }
            Err(reason) => {
                let waiting = matches!(
                    reason,
                    Unreadable::NeedsPassword | Unreadable::WrongPassword
                )
                .then_some(bytes);
                self.fail(line, file, reason.code(), waiting);
            }
        }
    }

    /// Opens the file waiting on `line` with its password. `false` when none waits.
    pub fn open_with(&mut self, line: Line, password: &str) -> bool {
        let waiting = match line {
            Line::Ca => take_waiting(&mut self.ca),
            Line::Own => take_waiting(&mut self.own),
            Line::Key => take_waiting(&mut self.key),
        };
        let Some((file, bytes)) = waiting else {
            return false;
        };
        self.choose(line, &file, bytes, Some(password));
        true
    }

    fn place(&mut self, line: Line, file: &str, contents: Contents) {
        let Contents {
            certificates,
            authorities,
            key,
        } = contents;
        if line == Line::Key && key.is_none() {
            return self.fail(line, file, "no_key", None);
        }
        if line != Line::Key && certificates.is_empty() && authorities.is_empty() {
            return self.fail(line, file, "no_certificate", None);
        }
        let has_key = key.is_some();
        if let Some(key) = key {
            self.key = filled(file, key);
        }
        if !authorities.is_empty() {
            self.ca = filled(file, authorities);
        }
        if !certificates.is_empty() {
            // beside a key the certificates are bacnetio's own, wherever the file was chosen
            if line == Line::Ca && !has_key {
                self.ca = filled(file, certificates);
            } else {
                self.own = filled(file, certificates);
            }
        }
    }

    fn fail(&mut self, line: Line, file: &str, reason: &'static str, waiting: Option<Vec<u8>>) {
        match line {
            Line::Ca => self.ca = failed(file, reason, waiting),
            Line::Own => self.own = failed(file, reason, waiting),
            Line::Key => self.key = failed(file, reason, waiting),
        }
    }

    /// The lines in words at `now` (Unix seconds).
    pub fn view(&self, now: i64) -> View {
        let inspection = self.inspect(now);
        let ca = line_view(&self.ca, |_| match &inspection {
            Some(found) => certificate_view(
                found
                    .certificates
                    .iter()
                    .find(|certificate| certificate.role == ca_role(self.own.value().is_some())),
                now,
            ),
            None => ("chosen", None, None, None),
        });
        let own = line_view(&self.own, |_| {
            let Some(found) = &inspection else {
                return ("chosen", None, None, None);
            };
            if found.direct_issuer == ScDirectIssuer::NotDirect {
                let (_, from, until, _) = certificate_view(found.certificates.first(), now);
                return ("not_from_ca", from, until, None);
            }
            certificate_view(found.certificates.first(), now)
        });
        let key = line_view(&self.key, |_| {
            let consistency = inspection.as_ref().map(|found| found.key_consistency);
            let state = match consistency {
                Some(ScKeyConsistency::Match) => "matches",
                Some(ScKeyConsistency::Mismatch) => "mismatch",
                Some(ScKeyConsistency::Unusable) => "unusable",
                _ => "chosen",
            };
            (state, None, None, None)
        });
        let ready =
            self.ca.value().is_some() && self.own.value().is_some() && key.state == "matches";
        View {
            ca,
            own,
            key,
            ready,
        }
    }

    /// Checked by bacnet-transport, which takes PEM; `None` while neither certificate is there.
    fn inspect(&self, now: i64) -> Option<bacnet_transport::sc_tls::inspection::ScPemInspection> {
        let ca = self
            .ca
            .value()
            .map(|certificates| certificates_pem(certificates));
        let Some(own) = self.own.value() else {
            return inspect_sc_pem(ca?.as_bytes(), None, None, Some(now)).ok();
        };
        let key = self.key.value().map(key_pem);
        inspect_sc_pem(
            certificates_pem(own).as_bytes(),
            ca.as_deref().map(str::as_bytes),
            key.as_deref().map(str::as_bytes),
            Some(now),
        )
        .ok()
    }

    /// The line as PEM with the name of its file, to keep it; `None` while it is not filled.
    pub fn pem(&self, line: Line) -> Option<(String, Vec<u8>)> {
        let (file, pem) = match line {
            Line::Ca => (self.ca.file()?, certificates_pem(self.ca.value()?)),
            Line::Own => (self.own.file()?, certificates_pem(self.own.value()?)),
            Line::Key => (self.key.file()?, key_pem(self.key.value()?)),
        };
        Some((file, pem.into_bytes()))
    }

    pub fn key_had_password(&self) -> bool {
        self.key_had_password
    }

    /// What the connection to the hub needs, once all three lines are filled.
    pub fn tls_config(&self) -> Result<ScNodeTlsConfig, String> {
        let (Some(ca), Some(own), Some(key)) =
            (self.ca.value(), self.own.value(), self.key.value())
        else {
            return Err("choose the site CA, bacnetio's certificate and its key first".to_string());
        };
        ScNodeTlsConfig::from_der(ca.clone(), own.clone(), key.clone_key())
            .map_err(|error| format!("the certificates do not make a connection: {error}"))
    }
}

/// Without bacnetio's own certificate the CA is checked in its place.
fn ca_role(own_chosen: bool) -> ScInputRole {
    if own_chosen {
        ScInputRole::PeerTrust
    } else {
        ScInputRole::Identity
    }
}

fn filled<T>(file: &str, value: T) -> Slot<T> {
    Slot::Filled {
        file: file.to_string(),
        value,
    }
}

fn failed<T>(file: &str, reason: &'static str, waiting: Option<Vec<u8>>) -> Slot<T> {
    Slot::Failed {
        file: file.to_string(),
        reason,
        waiting,
    }
}

fn take_waiting<T>(slot: &mut Slot<T>) -> Option<(String, Vec<u8>)> {
    match slot {
        Slot::Failed { file, waiting, .. } => waiting.take().map(|bytes| (file.clone(), bytes)),
        _ => None,
    }
}

type Words = (&'static str, Option<i64>, Option<i64>, Option<i64>);

fn line_view<T>(slot: &Slot<T>, filled: impl FnOnce(&T) -> Words) -> LineView {
    let (state, from, until, days) = match slot {
        Slot::Empty => ("missing", None, None, None),
        Slot::Failed { reason, .. } => (*reason, None, None, None),
        Slot::Filled { value, .. } => filled(value),
    };
    LineView {
        file: slot.file(),
        state,
        from,
        until,
        days,
    }
}

fn certificate_view(certificate: Option<&ScInspectedCertificate>, now: i64) -> Words {
    let Some(certificate) = certificate else {
        return ("nothing", None, None, None);
    };
    let Some(metadata) = &certificate.metadata else {
        return ("nothing", None, None, None);
    };
    let (from, until) = (metadata.not_before_unix(), metadata.not_after_unix());
    let state = match (certificate.validity, certificate.expiry) {
        (ScValidity::Expired, _) => "expired",
        (ScValidity::NotYetValid, _) => "not_yet_valid",
        (ScValidity::WithinInterval, ScExpiry::Warning | ScExpiry::Alarm) => "expires_soon",
        (ScValidity::WithinInterval, _) => "valid",
        _ => "nothing",
    };
    let days = (state == "expires_soon").then(|| (until - now) / DAY_S);
    (state, Some(from), Some(until), days)
}

fn certificates_pem(certificates: &[CertificateDer<'_>]) -> String {
    certificates
        .iter()
        .map(|certificate| pem("CERTIFICATE", certificate.as_ref()))
        .collect()
}

fn key_pem(key: &PrivateKeyDer<'_>) -> String {
    let label = match key {
        PrivateKeyDer::Pkcs1(_) => "RSA PRIVATE KEY",
        PrivateKeyDer::Sec1(_) => "EC PRIVATE KEY",
        _ => "PRIVATE KEY",
    };
    pem(label, key.secret_der())
}

fn pem(label: &str, der: &[u8]) -> String {
    let text = STANDARD.encode(der);
    let lines: Vec<&str> = text
        .as_bytes()
        .chunks(64)
        .map(|line| std::str::from_utf8(line).expect("base64 is ASCII"))
        .collect();
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
        lines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sc_files::tests::{Made, made};

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn chosen(made: &Made) -> Certificates {
        let mut lines = Certificates::default();
        lines.choose(Line::Ca, "ca.pem", made.ca_pem.clone().into_bytes(), None);
        lines.choose(Line::Own, "bacnetio.crt", made.own_der.clone(), None);
        lines.choose(
            Line::Key,
            "bacnetio.key",
            made.key_pem.clone().into_bytes(),
            None,
        );
        lines
    }

    #[test]
    fn three_files_that_belong_together() {
        let empty = Certificates::default().view(now());
        assert_eq!(empty.ca.state, "missing");
        assert!(!empty.ready);

        let made = made();
        let lines = chosen(&made);
        let view = lines.view(now());
        assert_eq!(
            (view.ca.state, view.own.state, view.key.state),
            ("valid", "valid", "matches")
        );
        assert_eq!(view.own.file.as_deref(), Some("bacnetio.crt"));
        assert!(view.own.until.unwrap() > now());
        assert!(view.ready);
        assert!(lines.tls_config().is_ok());
    }

    #[test]
    fn a_certificate_of_another_ca_or_a_key_of_another_certificate() {
        let (made, other) = (made(), made());
        let mut lines = chosen(&made);
        lines.choose(Line::Own, "other.pem", other.own_pem.into_bytes(), None);
        let view = lines.view(now());
        assert_eq!(
            (view.own.state, view.key.state),
            ("not_from_ca", "mismatch")
        );
        assert!(!view.ready);

        let mut lines = chosen(&made);
        lines.choose(Line::Key, "other.key", other.key_pem.into_bytes(), None);
        assert_eq!(lines.view(now()).key.state, "mismatch");
        // a certificate where the key belongs
        lines.choose(Line::Key, "ca.pem", made.ca_pem.into_bytes(), None);
        assert_eq!(lines.view(now()).key.state, "no_key");
        assert!(lines.tls_config().is_err());
    }

    #[test]
    fn running_out_and_run_out() {
        let made = made();
        let lines = chosen(&made);
        let until = lines.view(now()).own.until.unwrap();
        let soon = lines.view(until - 10 * DAY_S);
        assert_eq!((soon.own.state, soon.own.days), ("expires_soon", Some(10)));
        assert_eq!(lines.view(until + DAY_S).own.state, "expired");
    }

    #[test]
    fn one_file_fills_the_lines_it_holds() {
        let made = made();
        let mut lines = Certificates::default();
        lines.choose(
            Line::Own,
            "bacnetio.pem",
            format!("{}{}", made.own_pem, made.key_pem).into_bytes(),
            None,
        );
        let view = lines.view(now());
        assert_eq!(
            (view.own.state, view.key.state, view.key.file.as_deref()),
            ("valid", "matches", Some("bacnetio.pem"))
        );

        let own = p12_keystore::Certificate::from_der(&made.own_der).unwrap();
        let ca = p12_keystore::Certificate::from_der(&made.ca_der).unwrap();
        let key = p12_keystore::PrivateKey::from_der(&made.key_der).unwrap();
        let mut store = p12_keystore::KeyStore::new();
        store.add_entry(
            "bacnetio",
            p12_keystore::KeyStoreEntry::PrivateKeyChain(p12_keystore::PrivateKeyChain::new(
                b"1".to_vec(),
                key,
                [own, ca],
            )),
        );
        let p12 = store.writer("geheim").write().unwrap();
        let mut lines = Certificates::default();
        lines.choose(Line::Ca, "site.p12", p12, None);
        assert_eq!(lines.view(now()).ca.state, "needs_password");
        assert!(lines.open_with(Line::Ca, "falsch"));
        assert_eq!(lines.view(now()).ca.state, "wrong_password");
        assert!(lines.open_with(Line::Ca, "geheim"));
        let view = lines.view(now());
        assert_eq!(
            (view.ca.state, view.own.state, view.key.state),
            ("valid", "valid", "matches")
        );
        assert_eq!(view.key.file.as_deref(), Some("site.p12"));
        assert!(!lines.open_with(Line::Ca, "geheim"));
        assert!(lines.key_had_password());

        // kept as PEM, read back as a chosen file
        let mut again = Certificates::default();
        for line in [Line::Ca, Line::Own, Line::Key] {
            let (file, pem) = lines.pem(line).unwrap();
            again.choose(line, &file, pem, None);
        }
        assert_eq!(again.view(now()), view);
        assert!(!again.key_had_password());
    }
}
