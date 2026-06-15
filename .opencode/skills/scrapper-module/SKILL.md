---
name: scrapper-module
description: >-
  Use when working on rustcraper/src/scrapper.rs — the Google Maps scraper
  using chromiumoxide. Covers the buscar() function signature, feed scraping
  loop, anti-detection, data extraction via JS evaluation, and chromiumoxide
  API patterns. Use when you need to modify scraping logic, add new selectors,
  fix extraction bugs, or improve scrolling behavior. NOT for grid generation
  or shared types.
---

# Scrapper Module

File: `rustcraper/src/scrapper.rs`

## Public API

```rust
pub async fn buscar(
    browser: &Browser,         // chromiumoxide Browser (already launched)
    config: &SearchConfig,     // lat, lng, zoom, query, delays, etc.
    progress: &dyn ProgressReporter,  // status/error callbacks
) -> Result<Vec<Tintoreria>>   // anyhow::Result
```

**Key contract:** The caller manages the browser lifecycle (launch + handler task).
`buscar()` only creates a new `Page` via `browser.new_page(&url)`.

## Internal flow

1. **Navigate** → `browser.new_page(&maps_url)` → `page.wait_for_navigation()`
2. **Accept cookies** → JS evaluation that clicks the cookie button by text content
3. **Detect mode** → checks for `[role="feed"]` (list) vs single detail page
4. **Feed mode** (`scrape_feed`):
   - Loop: count `a` elements in feed, click next unprocessed one by index
   - Extract data from info panel via `extract_current_result()`
   - Scroll feed via `scroll_feed()` (sets `scrollTop = scrollHeight`)
   - Break after 5 consecutive scrolls without new items
5. **Single mode** (`scrape_single`) → extract from current detail page

## chromiumoxide patterns used

### Navigation
```rust
let page = browser.new_page(&url).await?;
page.wait_for_navigation().await?;
```

### JS evaluation (preferred over Element API)
```rust
// Execute JS and extract typed value
let count: i32 = page
    .evaluate("document.querySelectorAll('selector').length")
    .await?
    .into_value()?;

// Execute JS for side effects (click, scroll)
page.evaluate("document.querySelector('selector').click()").await?;
```

### Finding elements (only when needed)
```rust
let has_feed = page.find_element("[role=\"feed\"]").await.is_ok();
```

### Element staleness
After scrolling or DOM changes, previously acquired `Element` references may become
stale. **Always re-query elements** or use JS evaluation (`.evaluate()`) instead
of storing Element handles across async operations.

## JS extraction selectors (Google Maps DOM)

Data is extracted from the info panel (visible after clicking a result in the feed):

| Field | CSS selectors tried |
|-------|---------------------|
| Name | `.fontHeadline` → textContent |
| Phone | `button[data-tooltip*=teléfono]`, `button[data-tooltip*=phone]`, `[data-item-id*=phone]` |
| Email | `a[href^="mailto:"]` → strip `mailto:` prefix |
| Website | `button[data-tooltip*="sitio web"]`, `[data-item-id*=authority]`, `a[aria-label*="Sitio web"]` |

Extraction uses `JSON.stringify({name, phone, email, web})` + `serde_json::from_str()`
in Rust.

## Helper functions

| Function | Purpose |
|----------|---------|
| `random_delay(min_ms, max_ms)` | Non-crypto-random delay using subsec_nanos |
| `clean_maps_url(url)` | Strips query params and trailing slash |
| `accept_cookies(page)` | Clicks cookie accept button by iterating button text content |
| `extract_current_result(page)` | JS eval: extract data from visible info panel → `Tintoreria` |
| `scroll_feed(page)` | JS eval: scrolls `[role="feed"]` to bottom |

## Testing notes

The scrapper requires a real Chrome/Chromium instance. No unit tests exist yet
(mock-based testing is not implemented). Manual testing:

```rust
// In a binary crate (future CLI):
let (browser, mut handler) = Browser::launch(config).await?;
tokio::spawn(async move { while let Some(_) = handler.next().await {} });
let coincidences = search(&browser, &config, &NoopProgress).await?;
```

## Legacy comparison

The legacy `legacy/src/scrapper.ts` uses Playwright. Key differences:
- No `ensureBrowser()` — browser is passed in
- No Playwright pseudo-selectors (`:has-text()`) — all text matching via JS
- Progress via trait (`&dyn ProgressReporter`) instead of callback mutation
- Random delays use `SystemTime::subsec_nanos` instead of `Math.random()`
