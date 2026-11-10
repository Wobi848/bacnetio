//! BACnet/SC (docs/concept-sc.md): bacnetio on a hub with a device, both made in the test with
//! certificates of a CA made here.
#![cfg(feature = "sc")]

use std::time::Duration;

use bacnet_objects::analog::AnalogInputObject;
use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
use bacnet_objects::traits::BACnetObject;
use bacnet_server::server::BACnetServer;
use bacnet_transport::sc::ScTransport;
use bacnet_transport::sc_hub::{ScHub, ScHubTlsConfig};
use bacnet_transport::sc_tls::{ScNodeTlsConfig, TlsWebSocket};
use bacnet_types::enums::{EngineeringUnits, ObjectType, PropertyIdentifier};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use bacnetio::address::Address;
use bacnetio::connection::{self, open_sc};
use bacnetio::sc_identity::Identity;
use bacnetio::scan;
use rcgen::{CertificateParams, Issuer, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

const WHO_IS_WAIT: Duration = Duration::from_secs(2);
const DEVICE_VMAC: [u8; 6] = [0x22, 0, 0, 0, 0, 1];

struct Ca {
    certificate: CertificateDer<'static>,
    issuer: Issuer<'static, KeyPair>,
}

impl Ca {
    fn new() -> Ca {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let key = KeyPair::generate().unwrap();
        let certificate = params.self_signed(&key).unwrap().der().clone();
        Ca {
            certificate,
            issuer: Issuer::new(params, key),
        }
    }

    /// A certificate for `localhost` signed by this CA, and its key.
    fn issue(&self) -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
        let params = CertificateParams::new(vec!["localhost".into()]).unwrap();
        let key = KeyPair::generate().unwrap();
        let certificate = params.signed_by(&key, &self.issuer).unwrap().der().clone();
        (
            vec![certificate],
            PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )
    }

    fn node(&self) -> ScNodeTlsConfig {
        let (chain, key) = self.issue();
        ScNodeTlsConfig::from_der(vec![self.certificate.clone()], chain, key).unwrap()
    }
}

/// A hub with one device on it, `SC-AHU` (5001) with a supply air temperature; the hub's URL.
async fn plant(ca: &Ca) -> (ScHub, BACnetServer<ScTransport<TlsWebSocket>>, String) {
    let (chain, key) = ca.issue();
    let tls = ScHubTlsConfig::from_der(vec![ca.certificate.clone()], chain, key).unwrap();
    let hub = ScHub::start("127.0.0.1:0", tls, [0x10; 6], [0x10; 16])
        .await
        .unwrap();
    let url = format!("wss://localhost:{}/", hub.local_addr().unwrap().port());
    let identity = DeviceIdentity::new(5001, 42).unwrap().with_name("SC-AHU");
    let supply: Box<dyn BACnetObject> = Box::new(
        AnalogInputObject::new(1, "Supply air temp", EngineeringUnits::DEGREES_CELSIUS).unwrap(),
    );
    let device = BACnetServer::sc_builder()
        .hub_url(&url)
        .tls_config(ca.node())
        .vmac(DEVICE_VMAC)
        .device_uuid([0x22; 16])
        .device(identity.clone())
        .database(build_database_with_extra(&identity, vec![supply]).unwrap())
        .build()
        .await
        .unwrap();
    (hub, device, url)
}

#[tokio::test]
async fn finds_and_reads_a_device_through_the_hub() {
    let ca = Ca::new();
    let (_hub, _device, url) = plant(&ca).await;
    let connection = open_sc(&url, ca.node(), Identity::new().unwrap())
        .await
        .unwrap();

    let report = scan::run(&connection.client, WHO_IS_WAIT, Vec::new(), &|_| {})
        .await
        .unwrap();
    let found = report
        .devices
        .iter()
        .find(|device| device.instance == 5001)
        .expect("the device on the hub");
    // its VMAC, not an IP address of the same six octets
    assert_eq!(found.address, Address::Sc(DEVICE_VMAC));
    assert_eq!(found.details.name.as_deref(), Some("SC-AHU"));

    let name = connection::read_value(
        &connection.client,
        5001,
        ObjectIdentifier::new(ObjectType::ANALOG_INPUT, 1).unwrap(),
        PropertyIdentifier::OBJECT_NAME,
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        name,
        PropertyValue::CharacterString("Supply air temp".into())
    );
    connection.close().await.unwrap();
}

#[tokio::test]
async fn a_certificate_of_another_ca_is_refused() {
    let ca = Ca::new();
    let (_hub, _device, url) = plant(&ca).await;
    let error = open_sc(&url, Ca::new().node(), Identity::new().unwrap())
        .await
        .err()
        .expect("refused");
    assert!(
        format!("{error:#}").contains("cannot connect to the hub"),
        "{error:#}"
    );
}
