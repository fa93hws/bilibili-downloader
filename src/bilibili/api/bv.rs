use anyhow::{anyhow, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::{crawler::Fetching, logger::Logging};

#[derive(Serialize, Deserialize, Debug)]
pub struct Audio {
    pub base_url: String,
    pub bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Video {
    pub id: u8,
    pub base_url: String,
    pub bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Dash {
    pub video: Vec<Video>,
    pub audio: Vec<Audio>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub accept_description: Vec<String>,
    pub accept_quality: Vec<u8>,
    pub dash: Dash,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BVInfo {
    pub data: Data,
}

fn extract_title(document: &Html, video_url: &String) -> Result<String> {
    let title_selector = Selector::parse("h1").unwrap();
    let mut potential_titles: Vec<String> = Vec::new();
    for title_element in document.select(&title_selector) {
        potential_titles.push(title_element.inner_html());
    }
    if potential_titles.len() > 1 {
        return Err(anyhow!(
            "multiple <h1> tag found in the page '{}'",
            video_url,
        ));
    } else if potential_titles.is_empty() {
        return Err(anyhow!("no <h1> tag found in the page '{}'", video_url));
    } else {
        return Ok(potential_titles[0].to_string());
    }
}

fn extract_video_json(document: &Html, video_url: &String) -> Result<String> {
    let script_selector = Selector::parse("script").unwrap();
    let prefix = "window.__playinfo__=";
    for script_element in document.select(&script_selector) {
        let script = script_element.text().collect::<Vec<_>>().join("");
        if script.starts_with(prefix) {
            return Ok(script[prefix.len()..].to_owned());
        }
    }
    return Err(anyhow!("can't find video json from '{}'", video_url));
}

pub async fn get_bv_info<'a, T: Logging, F: Fetching>(
    crawler: &F,
    logger: &T,
    id: &String,
) -> Result<(BVInfo, String)> {
    let video_url = format!("https://www.bilibili.com/video/{id}/");
    let body_bytes = crawler.fetch_body(&video_url).await?;
    let body_str = std::str::from_utf8(&body_bytes)?;
    let document = Html::parse_document(body_str);
    let title = extract_title(&document, &video_url)?;
    logger.info(&format!("title found as '{title}'"));
    let video_json_str = extract_video_json(&document, &video_url)?;
    let info = serde_json::from_str::<BVInfo>(&video_json_str)?;
    Ok((info, title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_title_success() {
        let html_str = "<html><h1>foo</h1></html>";
        let document = Html::parse_document(html_str);
        let title = extract_title(&document, &String::from("")).expect("title should be extracted");
        assert_eq!(title, "foo");
    }

    #[test]
    fn extract_title_multiple_title() {
        let html_str = "<html><h1>bar</h1><h1>foo</h1></html>";
        let document = Html::parse_document(html_str);
        let title = extract_title(&document, &String::from(""));
        assert_eq!(title.is_ok(), false);
    }

    #[test]
    fn extract_title_no_title() {
        let html_str = "<html><h2>bar</h2><h2>foo</h2></html>";
        let document = Html::parse_document(html_str);
        let title = extract_title(&document, &String::from(""));
        assert_eq!(title.is_ok(), false);
    }
}
