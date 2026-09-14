use anyhow::Result;

use crate::empresite::config::EmpresiteParams;
use crate::scraper::ScrapeResult;
use crate::types::EmpresiteConfig;
use crate::verboser::Verboser;
use chromiumoxide::Browser;
use std::time::Duration;

/// Scrapea una pagina del listado de Empresite.
///
/// NOTA: implementacion inicial. La navegacion real (selectores, paginacion y
/// extraccion de fichas) esta pendiente. Devuelve `has_more = false` para que
/// la cola termine tras la primera pagina.
pub async fn scrape(
    browser: &Browser,
    params: &EmpresiteParams,
    config: &EmpresiteConfig,
    verboser: &dyn Verboser,
) -> Result<ScrapeResult> {
    verboser.searching_coincidences();

    let url = format!(
        "https://empresite.eleconomista.es/Actividad/{}/?pagina={}",
        config.search_query, params.page
    );

    let page = browser.new_page(&url).await?;
    let _ = page.wait_for_navigation().await;
    tokio::time::sleep(Duration::from_secs(3)).await;

    if verboser.is_cancelled() {
        return Ok(ScrapeResult::empty(false));
    }

    // TODO: extraer coincidencias del listado y detectar si hay pagina siguiente.
    let coincidences = Vec::new();
    let has_more = false;

    Ok(ScrapeResult::new(coincidences, has_more))
}
