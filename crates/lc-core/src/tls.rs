//! TLS: Agent tự sinh chứng chỉ mỗi lần chạy (không lưu khóa riêng ra đĩa);
//! Console chấp nhận chứng chỉ đó nhưng ghi lại dấu vân tay để gắn vào chữ ký xác thực.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{ring, verify_tls12_signature, verify_tls13_signature, CryptoProvider, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error, ServerConfig, SignatureScheme};
use sha2::{Digest, Sha256};

fn provider() -> Arc<CryptoProvider> {
    Arc::new(ring::default_provider())
}

pub fn fingerprint(cert_der: &[u8]) -> String {
    hex::encode(Sha256::digest(cert_der))
}

/// Cấu hình TLS phía Agent + dấu vân tay chứng chỉ của nó.
pub fn server_config(name: &str) -> Result<(Arc<ServerConfig>, String)> {
    let ck = rcgen::generate_simple_self_signed(vec![name.to_string(), "labcontrol-agent".into()])?;
    let der = ck.cert.der().to_vec();
    let fp = fingerprint(&der);
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
    let cfg = ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(vec![CertificateDer::from(der)], key)?;
    Ok((Arc::new(cfg), fp))
}

#[derive(Debug)]
struct Capture {
    seen: Arc<Mutex<Option<String>>>,
    algs: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for Capture {
    fn verify_server_cert(&self, ee: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8],
                          _: UnixTime) -> Result<ServerCertVerified, Error> {
        *self.seen.lock().unwrap() = Some(fingerprint(ee.as_ref()));
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct)
                              -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(m, c, d, &self.algs)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct)
                              -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(m, c, d, &self.algs)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

/// Cấu hình phía Console. Dấu vân tay thực sự nhìn thấy được ghi vào `seen` sau khi bắt tay.
pub fn client_config() -> (Arc<ClientConfig>, Arc<Mutex<Option<String>>>) {
    let p = provider();
    let seen: Arc<Mutex<Option<String>>> = Default::default();
    let cfg = ClientConfig::builder_with_provider(p.clone())
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Capture { seen: seen.clone(), algs: p.signature_verification_algorithms }))
        .with_no_client_auth();
    (Arc::new(cfg), seen)
}
