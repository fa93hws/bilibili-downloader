use std::{time::SystemTime, fs::File, io::Write};

use crate::{crawler::Crawler, logger::Logger, video_info::{RawVideoInfo, VideoInfo, VideoParseError}};
use scraper::{Html, Selector};

pub struct Video<'a> {
    url: String,
    crawler: &'a Crawler<'a>,
    logger: &'a Logger,
}

impl<'a> Video<'a> {
    pub fn new(id: &str, crawler: &'a Crawler, logger: &'a Logger) -> Self {
        Video {
            url: format!("https://www.bilibili.com/video/{id}/"),
            crawler,
            logger,
        }
    }

    fn extract_title(&self, document: &Html) -> String {
        let title_selector = Selector::parse("h1").unwrap();
        let mut potential_titles: Vec<String> = Vec::new();
        for title_element in document.select(&title_selector) {
            potential_titles.push(title_element.inner_html());
        }
        if potential_titles.len() > 1 {
            self.logger.warn(&format!("multiple <h1> tag found in the page '{}', while only 1 is expected. The first one will be used", self.url));
            return potential_titles[0].to_string();
        } else if potential_titles.is_empty() {
            self.logger.warn(&format!(
                "no <h1> tag found in the page '{}', will use timestamp for the title",
                self.url
            ));
            return SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string();
        } else {
            return potential_titles[0].to_string();
        }
    }

    fn extract_video_json(&self, document: &Html) -> Result<String, VideoParseError> {
        let script_selector = Selector::parse("script").unwrap();
        let prefix = "window.__playinfo__=";
        for script_element in document.select(&script_selector) {
            let script = script_element.text().collect::<Vec<_>>().join("");
            if script.starts_with(prefix) {
                return Ok(script[prefix.len()..].to_owned());
            }
        }
        return Err(VideoParseError::new(format!("can't find video json from '{}'", self.url)));
    }

    pub async fn get_video_info(&self) -> Result<VideoInfo, Box<dyn std::error::Error>> {
        let body_bytes = self.crawler.fetch_body(&self.url).await?;
        let body_str = std::str::from_utf8(&body_bytes)?;
        let document = Html::parse_document(body_str);
        let title = self.extract_title(&document);
        self.logger.info(&format!("title found as '{title}'"));
        let video_json_str = self.extract_video_json(&document)?;
        let raw_video_info: RawVideoInfo = serde_json::from_str(&video_json_str)?;
        let video_info = VideoInfo::new(title, raw_video_info)?;
        return Ok(video_info)
    }

    pub async fn download(&self, video_info: &VideoInfo, selected_quality_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let video_url = video_info.get_video_url(selected_quality_index);
        self.logger.verbose(&format!("video url for '{}' is '{}'", video_info.quality_description[selected_quality_index], video_url));
        let audio_url = video_info.get_audio_url();
        self.logger.verbose(&format!("audio url is '{}'", audio_url));

        self.logger.info("视频下载中");
        let video_bytes = self.crawler.fetch_body(&video_url).await?;
        let mut video_file = File::create(format!("./{}_video.mp4", video_info.title))?;
        video_file.write_all(&video_bytes)?;

        self.logger.info("音频下载中");
        let audio_bytes = self.crawler.fetch_body(&audio_url).await?;
        let mut audio_file = File::create(format!("./{}audio.mp4", video_info.title))?;
        audio_file.write_all(&audio_bytes)?;
        Ok(())
    }
}
