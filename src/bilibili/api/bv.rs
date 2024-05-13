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
        let script = script_element.text().collect::<Vec<_>>().join("").trim().to_owned();
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
    use crate::crawler::MockFetching;

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

    #[test]
    fn extract_video_json_success() {
        let html_str = r#"<html><script>window.__playinfo__={"foo": "bar"}</script></html>"#;
        let document = Html::parse_document(html_str);
        let video_json_str =
            extract_video_json(&document, &String::from("")).expect("json should be extracted");
        assert_eq!(video_json_str, r#"{"foo": "bar"}"#);
    }

    #[test]
    fn extract_video_json_no_script() {
        let html_str = "<html><h1>foo</h1></html>";
        let document = Html::parse_document(html_str);
        let video_json_str = extract_video_json(&document, &String::from(""));
        assert_eq!(video_json_str.is_ok(), false);
    }

    #[test]
    fn extract_video_json_no_json() {
        let html_str = r#"<html><script>window.__playinfo__=foo</script></html>"#;
        let document = Html::parse_document(html_str);
        let video_json_str = extract_video_json(&document, &String::from(""));
        assert_eq!(video_json_str.is_ok(), false);
    }

    #[test]
    fn extract_video_json_no_window() {
        let html_str = r#"<html><script>foo=bar</script></html>"#;
        let document = Html::parse_document(html_str);
        let video_json_str = extract_video_json(&document, &String::from(""));
        assert_eq!(video_json_str.is_ok(), false);
    }

    #[tokio::test]
    async fn get_bv_info_success() {
        let logger = crate::logger::Logger::new(0);
        let mut mock_crawler = MockFetching::new();
        let mock_html_str = r#"
        <html>
            <script>
                window.__playinfo__={
                    "data": {
                        "accept_description":["超清 4K","高清 1080P60","高清 1080P","高清 720P","清晰 480P","流畅 360P"],
                        "accept_quality":[120,116,80,64,32,16],
                        "dash": {
                            "video": [{
                                "id": 120,
                                "base_url": "base_url__v_120_0",
                                "bandwidth": 1200
                            }, {
                                "id": 120,
                                "base_url": "base_url_v_1201",
                                "bandwidth": 1201
                            }, {
                                "id": 116,
                                "base_url": "base_url_v_116",
                                "bandwidth": 116
                            }],
                            "audio": [{
                                "id": 30280,
                                "base_url": "base_url_a_30280",
                                "bandwidth": 30280
                            }, {
                                "id": 30216,
                                "base_url": "base_url_a_30216",
                                "bandwidth": 30216
                            }]
                        }
                    }
                }
            </script>
            <body>
                <h1>fake title</h1>
            </body>
        </html>
        "#;
        mock_crawler.expect_fetch_body().times(1).returning(|url| {
            assert_eq!(url, "https://www.bilibili.com/video/BV12345678/");
            Ok(mock_html_str.to_owned().into_bytes())
        });
        let (info, title) = get_bv_info(&mock_crawler, &logger, &String::from("BV12345678"))
            .await.unwrap();
        assert_eq!(title, "fake title");
        assert_eq!(
            info.data.accept_description,
            [
                "超清 4K",
                "高清 1080P60",
                "高清 1080P",
                "高清 720P",
                "清晰 480P",
                "流畅 360P"
            ]
        );
        assert_eq!(info.data.accept_quality, [120, 116, 80, 64, 32, 16]);
        assert_eq!(info.data.dash.video[0].id, 120);
        assert_eq!(info.data.dash.video[0].base_url, "base_url__v_120_0");
        assert_eq!(info.data.dash.video[0].bandwidth, 1200);
        assert_eq!(info.data.dash.video[1].id, 120);
        assert_eq!(info.data.dash.video[1].base_url, "base_url_v_1201");
        assert_eq!(info.data.dash.video[1].bandwidth, 1201);
        assert_eq!(info.data.dash.video[2].id, 116);
        assert_eq!(info.data.dash.video[2].base_url, "base_url_v_116");
        assert_eq!(info.data.dash.video[2].bandwidth, 116);
        assert_eq!(info.data.dash.audio[0].base_url, "base_url_a_30280");
        assert_eq!(info.data.dash.audio[0].bandwidth, 30280);
        assert_eq!(info.data.dash.audio[1].base_url, "base_url_a_30216");
        assert_eq!(info.data.dash.audio[1].bandwidth, 30216);
    }
}
