// let config = rustls::ClientConfig::builder()
//         .with_safe_defaults()
//         .with_root_certificates(root_store)
//         .with_client_auth_cert(client_cert, client_key)
//         .context("failed to attach client certificate to TLS config")?;

// /// Represents a private key and X509 cert as a client certificate.
// #[derive(Clone)]
// pub struct Identity {
//     #[cfg_attr(not(any(feature = "native-tls", feature = "__rustls")), allow(unused))]
//     inner: ClientCert,
// }

// enum ClientCert {
//     #[cfg(feature = "native-tls")]
//     Pkcs12(native_tls_crate::Identity),
//     #[cfg(feature = "native-tls")]
//     Pkcs8(native_tls_crate::Identity),

//     #[cfg(feature = "__rustls")]
//     Pem {
//         key: rustls_pki_types::PrivateKeyDer<'static>,
//         certs: Vec<rustls_pki_types::CertificateDer<'static>>,
//     },
// }
