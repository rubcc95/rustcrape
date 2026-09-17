use std::sync::Arc;
use anyhow::Result;

use crate::verboser::Verboser;
use crate::vpn::VpnRotator;

/// Version de Chrome declarada en el User-Agent y los Client Hints. Al no haber
/// navegador, es un valor fijo y actual (sin mismatch TLS-vs-UA porque el TLS lo
/// emite reqwest, no Chrome).
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36";

/// Manejador global de recursos compartidos por todos los targets: el rotador
/// de VPN, el cliente HTTP y el informador de progreso. Se pasa como una sola
/// dependencia a las funciones que necesitan mas de uno de estos servicios.
///
/// Es clonable: el rotador y el `Verboser` van tras `Arc` y el `reqwest::Client`
/// ya comparte su pool/estado internamente.
#[derive(Clone)]
pub struct Context {
    verboser: Arc<dyn Verboser>,
    vpn: Arc<VpnRotator>,
    http: reqwest::Client,
}

impl Context {
    /// Construye el contexto global. El cliente HTTP imita una sesion de
    /// navegador (cookie jar y cabeceras coherentes) y se comparte entre todos
    /// los targets, por lo que no lleva timeout global: cada peticion decide el
    /// suyo.
    pub fn new(verboser: impl Verboser, nordvpn_path: Option<String>, frequency: u32) -> Self {
        use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderName, HeaderValue, USER_AGENT as UA};

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(UA, HeaderValue::from_static(USER_AGENT));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
            ),
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("es-ES,es;q=0.9,en;q=0.8"),
        );
        headers.insert(
            HeaderName::from_static("sec-ch-ua"),
            HeaderValue::from_static(
                "\"Chromium\";v=\"152\", \"Google Chrome\";v=\"152\", \"Not?A_Brand\";v=\"24\"",
            ),
        );
        headers.insert(
            HeaderName::from_static("sec-ch-ua-mobile"),
            HeaderValue::from_static("?0"),
        );
        headers.insert(
            HeaderName::from_static("sec-ch-ua-platform"),
            HeaderValue::from_static("\"Windows\""),
        );

        let http = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .expect("no se pudo construir el cliente HTTP global");

        Self {
            verboser: Arc::new(verboser),
            vpn: Arc::new(VpnRotator::new(nordvpn_path, frequency)),
            http,
        }
    }

    /// Informador de progreso compartido.
    #[inline]
    pub fn verboser(&self) -> &dyn Verboser {
        self.verboser.as_ref()
    }

    /// Cliente HTTP compartido (cookie jar + cabeceras de navegador).
    #[inline]
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    #[inline]
    pub fn vpn_tick(&self) -> impl Future<Output = Result<bool>> {
        self.vpn.tick(self)
    }

    #[inline]
    pub fn vpn_rotate(&self) -> impl Future<Output = Result<bool>> {
        self.vpn.force_rotate(self)
    }

    #[inline]
    pub fn vpn_rotate_awaited(&self) -> impl Future<Output = Result<bool>> {
        self.vpn.force_rotate_awaited(self)
    }   

}
