//! TLS to PostgreSQL with rustls, by libpq's `sslmode`:
//!
//! | `sslmode` | TLS | Certificate checked | Host name checked |
//! | --- | --- | --- | --- |
//! | `disable` | no | — | — |
//! | `allow`, `prefer` (default) | when the server has it | no | no |
//! | `require` | yes | only with `sslrootcert` | no |
//! | `verify-ca` | yes | yes | no |
//! | `verify-full` | yes | yes | yes |
//!
//! Certificates chain to `sslrootcert` (PEM) or, without it or with
//! `sslrootcert=system`, to the system's roots.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_postgres::tls::{ChannelBinding, MakeTlsConnect, TlsConnect, TlsStream};

/// libpq's `sslmode`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SslMode {
    Disable,
    Allow,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl SslMode {
    pub(super) fn parse(s: &str) -> Result<Self, String> {
        Ok(match s {
            "disable" => SslMode::Disable,
            "allow" => SslMode::Allow,
            "prefer" => SslMode::Prefer,
            "require" => SslMode::Require,
            "verify-ca" => SslMode::VerifyCa,
            "verify-full" => SslMode::VerifyFull,
            other => {
                return Err(format!(
                    "sslmode '{other}': expected disable, allow, prefer, require, verify-ca or \
                     verify-full"
                ));
            }
        })
    }

    /// Whether tokio-postgres asks for TLS, and may go without.
    pub(super) fn client_mode(self) -> tokio_postgres::config::SslMode {
        use tokio_postgres::config::SslMode as Client;
        match self {
            SslMode::Disable => Client::Disable,
            SslMode::Allow | SslMode::Prefer => Client::Prefer,
            SslMode::Require | SslMode::VerifyCa | SslMode::VerifyFull => Client::Require,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct TlsSettings {
    pub(super) mode: SslMode,
    /// A PEM file, or `system`.
    pub(super) root_cert: Option<String>,
}

/// What a certificate must pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Check {
    Nothing,
    Chain,
    ChainAndName,
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn roots(root_cert: Option<&str>) -> Result<RootCertStore, String> {
    let mut roots = RootCertStore::empty();
    match root_cert {
        Some(path) if path != "system" => {
            let certs = CertificateDer::pem_file_iter(path)
                .and_then(|certs| certs.collect::<Result<Vec<_>, _>>())
                .map_err(|e| format!("sslrootcert {path}: {e}"))?;
            let (added, _) = roots.add_parsable_certificates(certs);
            if added == 0 {
                return Err(format!("sslrootcert {path}: no certificates"));
            }
        }
        _ => {
            let found = rustls_native_certs::load_native_certs();
            roots.add_parsable_certificates(found.certs);
            if roots.is_empty() {
                return Err(format!(
                    "no system root certificates for PostgreSQL TLS: {:?}; set sslrootcert",
                    found.errors
                ));
            }
        }
    }
    Ok(roots)
}

/// The connector for `settings`; `None` without TLS.
pub(super) fn connector(settings: &TlsSettings) -> Result<Option<MakeRustlsConnect>, String> {
    let check = match (settings.mode, settings.root_cert.is_some()) {
        (SslMode::Disable, _) => return Ok(None),
        (SslMode::Allow | SslMode::Prefer, _) | (SslMode::Require, false) => Check::Nothing,
        (SslMode::Require, true) | (SslMode::VerifyCa, _) => Check::Chain,
        (SslMode::VerifyFull, _) => Check::ChainAndName,
    };
    let provider = provider();
    let verifier: Arc<dyn ServerCertVerifier> = match check {
        Check::Nothing => Arc::new(NoCheck(provider.clone())),
        Check::Chain | Check::ChainAndName => {
            let roots = roots(settings.root_cert.as_deref())?;
            let webpki =
                WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider.clone())
                    .build()
                    .map_err(|e| format!("PostgreSQL TLS: {e}"))?;
            match check {
                Check::ChainAndName => webpki,
                _ => Arc::new(ChainOnly(webpki)),
            }
        }
    };
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("PostgreSQL TLS: {e}"))?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok(Some(MakeRustlsConnect(Arc::new(config))))
}

/// `prefer` and `require`: encrypt, check no certificate (as libpq). The
/// handshake signatures are still checked against the certificate.
#[derive(Debug)]
struct NoCheck(Arc<CryptoProvider>);

impl ServerCertVerifier for NoCheck {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// `verify-ca`: the chain must lead to a trusted root; the name may differ.
#[derive(Debug)]
struct ChainOnly(Arc<WebPkiServerVerifier>);

impl ServerCertVerifier for ChainOnly {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        match self
            .0
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
        {
            Err(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_verify_schemes()
    }
}

#[derive(Clone)]
pub(super) struct MakeRustlsConnect(Arc<ClientConfig>);

impl<S> MakeTlsConnect<S> for MakeRustlsConnect
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Stream = RustlsStream<S>;
    type TlsConnect = RustlsConnect;
    type Error = io::Error;

    fn make_tls_connect(&mut self, host: &str) -> io::Result<RustlsConnect> {
        let name = ServerName::try_from(host.to_string())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        Ok(RustlsConnect {
            config: self.0.clone(),
            name,
        })
    }
}

pub(super) struct RustlsConnect {
    config: Arc<ClientConfig>,
    name: ServerName<'static>,
}

impl<S> TlsConnect<S> for RustlsConnect
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Stream = RustlsStream<S>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = io::Result<RustlsStream<S>>> + Send>>;

    fn connect(self, stream: S) -> Self::Future {
        Box::pin(async move {
            tokio_rustls::TlsConnector::from(self.config)
                .connect(self.name, stream)
                .await
                .map(RustlsStream)
        })
    }
}

pub(super) struct RustlsStream<S>(tokio_rustls::client::TlsStream<S>);

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for RustlsStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for RustlsStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> TlsStream for RustlsStream<S> {
    /// No channel binding: SCRAM signs in without it.
    fn channel_binding(&self) -> ChannelBinding {
        ChannelBinding::none()
    }
}
