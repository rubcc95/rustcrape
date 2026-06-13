
use chromiumoxide::{Browser, BrowserConfig};
use anyhow::{Context, Result};
use futures::StreamExt; 
 
pub struct BrowserInstance {
    browser: Browser,
    _handler_handle: tokio::task::JoinHandle<()>,
}

impl BrowserInstance {
    pub async fn launch() -> Result<Self> {
        let config = BrowserConfig::builder()
            .with_head()
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build browser config: {}", e))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .context("failed to launch chromium browser")?;

        let _handler_handle = tokio::spawn(async move {
            while let Some(_) = handler.next().await {}
        });

        Ok(Self { browser, _handler_handle })
    }

    pub fn browser(&self) -> &Browser {
        &self.browser
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::NoopProgress;
    use crate::scrapper::buscar;
    use crate::types::SearchConfig;

    #[tokio::test]
    #[ignore]
    async fn test_browser_instance_launch() {
        let instance = BrowserInstance::launch().await.unwrap();
        let page = instance.browser().new_page("about:blank").await.unwrap();
        page.wait_for_navigation().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_buscar_with_browser_instance() {
        let instance = BrowserInstance::launch().await.unwrap();
        let config = SearchConfig {
            search_query: "tintorerías".to_string(),
            ..Default::default()
        };
        let result = buscar(&instance, &config, &NoopProgress).await.unwrap();
        println!("Found {} results", result.len());
    }
}
