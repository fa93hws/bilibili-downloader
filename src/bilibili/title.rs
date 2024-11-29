use anyhow::{anyhow, Result};
use scraper::{Html, Selector};

pub fn extract_title(document: &Html, video_id: &str) -> Result<String> {
    let title_selector = Selector::parse("h1").unwrap();
    let mut potential_titles: Vec<String> = Vec::new();
    for title_element in document.select(&title_selector) {
        potential_titles.push(title_element.inner_html());
    }
    if potential_titles.len() > 1 {
        return Err(anyhow!(
            "multiple <h1> tag found in the page '{}'",
            video_id,
        ));
    } else if potential_titles.is_empty() {
        return Err(anyhow!("no <h1> tag found in the page '{}'", video_id));
    } else {
        return Ok(potential_titles[0].to_string());
    }
}
