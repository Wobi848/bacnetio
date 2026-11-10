//! A simulated BACnet/SC network to try SC without a plant (docs/concept-sc.md):
//! `cargo run --example simulated_sc -- <folder> [port]`.
//!
//! It makes a site CA and certificates at every start, starts a hub on `127.0.0.1` and a device
//! "SC-AHU" (5001) on it, and writes into the folder what an integrator would hand out: `ca.pem`,
//! `bacnetio.pem` and `bacnetio.key`, the same in one `bacnetio.p12`, and the key once more with a
//! password in `bacnetio-locked.key`. The password of both is `demo`. The hub's address is
//! `wss://localhost:<port>/`; nothing of it enters the repository.

use std::path::{Path, PathBuf};
use std::time::Duration;

use bacnet_objects::analog::{AnalogInputObject, AnalogOutputObject, AnalogValueObject};
use bacnet_objects::binary::BinaryOutputObject;
use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
use bacnet_objects::traits::BACnetObject;
use bacnet_server::server::BACnetServer;
use bacnet_transport::sc_hub::{ScHub, ScHubTlsConfig};
use bacnet_transport::sc_tls::ScNodeTlsConfig;
use bacnet_types::enums::{EngineeringUnits, ObjectType};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use rcgen::{Certificate, CertificateParams, Issuer, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

const PORT: u16 = 47443;
const PASSWORD: &str = "demo";
const MOVE_EVERY: Duration = Duration::from_secs(5);

struct Issued {
    certificate: Certificate,
    key: KeyPair,
}

impl Issued {
    fn der(&self) -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
        (
            vec![self.certificate.der().clone()],
            PrivatePkcs8KeyDer::from(self.key.serialize_der()).into(),
        )
    }
}

fn issue(issuer: &Issuer<'_, KeyPair>, name: &str) -> Issued {
    let mut params = CertificateParams::new(vec!["localhost".to_string()]).unwrap();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, name);
    let key = KeyPair::generate().unwrap();
    let certificate = params.signed_by(&key, issuer).unwrap();
    Issued { certificate, key }
}

fn objects() -> Vec<Box<dyn BACnetObject>> {
    let celsius = EngineeringUnits::DEGREES_CELSIUS;
    let mut supply = AnalogInputObject::new(1, "Supply air temp", celsius).unwrap();
    supply.set_present_value(21.0);
    let setpoint = AnalogValueObject::new(1, "Supply air setpoint", celsius).unwrap();
    let mut valve = AnalogOutputObject::new(1, "Heating valve", EngineeringUnits::PERCENT).unwrap();
    valve.set_relinquish_default(30.0).unwrap();
    let fan = BinaryOutputObject::new(1, "Supply fan enable").unwrap();
    vec![
        Box::new(supply),
        Box::new(setpoint),
        Box::new(valve),
        Box::new(fan),
    ]
}

/// What an integrator hands out for bacnetio, in the forms bacnetio reads.
fn write_files(folder: &Path, ca: &Certificate, own: &Issued) {
    std::fs::create_dir_all(folder).expect("cannot make the folder");
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(folder.join(name), bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
    };
    write("ca.pem", ca.pem().as_bytes());
    write("bacnetio.pem", own.certificate.pem().as_bytes());
    write("bacnetio.key", own.key.serialize_pem().as_bytes());
    let key_der = own.key.serialize_der();
    let info = pkcs8::PrivateKeyInfoRef::try_from(key_der.as_slice()).unwrap();
    let locked = info
        .encrypt(PASSWORD)
        .unwrap()
        .to_pem("ENCRYPTED PRIVATE KEY", pkcs8::LineEnding::LF)
        .unwrap();
    write("bacnetio-locked.key", locked.as_bytes());
    let chain = p12_keystore::PrivateKeyChain::new(
        b"bacnetio".to_vec(),
        p12_keystore::PrivateKey::from_der(&own.key.serialize_der()).unwrap(),
        [
            p12_keystore::Certificate::from_der(own.certificate.der()).unwrap(),
            p12_keystore::Certificate::from_der(ca.der()).unwrap(),
        ],
    );
    let mut store = p12_keystore::KeyStore::new();
    store.add_entry(
        "bacnetio",
        p12_keystore::KeyStoreEntry::PrivateKeyChain(chain),
    );
    write("bacnetio.p12", &store.writer(PASSWORD).write().unwrap());
}

#[tokio::main]
async fn main() {
    let folder: PathBuf = std::env::args()
        .nth(1)
        .expect("give a folder for the certificate files")
        .into();
    let port: u16 = std::env::args()
        .nth(2)
        .map(|text| text.parse().expect("the port is a number"))
        .unwrap_or(PORT);

    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Simulated site CA");
    let ca_key = KeyPair::generate().unwrap();
    let ca = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);
    let (hub, device, own) = (
        issue(&issuer, "Simulated hub"),
        issue(&issuer, "SC-AHU"),
        issue(&issuer, "bacnetio"),
    );
    write_files(&folder, &ca, &own);

    let (chain, key) = hub.der();
    let hub_tls = ScHubTlsConfig::from_der(vec![ca.der().clone()], chain, key).unwrap();
    // lives as long as the program
    let _hub = ScHub::start(&format!("127.0.0.1:{port}"), hub_tls, [0x10; 6], [0x10; 16])
        .await
        .expect("cannot start the hub");
    let url = format!("wss://localhost:{port}/");

    let identity = DeviceIdentity::new(5001, 42).unwrap().with_name("SC-AHU");
    let database = build_database_with_extra(&identity, objects()).unwrap();
    let (chain, key) = device.der();
    let device = BACnetServer::sc_builder()
        .hub_url(&url)
        .tls_config(ScNodeTlsConfig::from_der(vec![ca.der().clone()], chain, key).unwrap())
        .vmac([0x22, 0, 0, 0, 0, 1])
        .device_uuid([0x22; 16])
        .device(identity)
        .database(database)
        .build()
        .await
        .expect("cannot start the device on the hub");
    println!(
        "Simulated BACnet/SC hub at {url} with device 5001 \"SC-AHU\"; the certificate files are in \
         {}. Ctrl+C to stop.",
        folder.display()
    );

    let supply = ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 1).unwrap();
    let mut tick = tokio::time::interval(MOVE_EVERY);
    let mut step = 0u32;
    loop {
        tick.tick().await;
        step += 1;
        let value = 21.0 + (step % 10) as f32 / 10.0;
        if let Err(error) = device
            .set_present_value_local(&supply, PropertyValue::Real(value))
            .await
        {
            eprintln!("cannot move the supply air temperature: {error}");
        }
    }
}
