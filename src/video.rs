use std::{fs::File, io::Write, process::Command};

use crate::{
    crawler::Crawler,
    logger::Logger,
    video_info::{RawVideoInfo, VideoInfo},
};
use anyhow::{anyhow, Result};
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

    fn extract_title(&self, document: &Html) -> Result<String> {
        let title_selector = Selector::parse("h1").unwrap();
        let mut potential_titles: Vec<String> = Vec::new();
        for title_element in document.select(&title_selector) {
            potential_titles.push(title_element.inner_html());
        }
        if potential_titles.len() > 1 {
            return Err(anyhow!(
                "multiple <h1> tag found in the page '{}'",
                self.url
            ));
        } else if potential_titles.is_empty() {
            return Err(anyhow!("no <h1> tag found in the page '{}'", self.url));
        } else {
            return Ok(potential_titles[0].to_string());
        }
    }

    fn extract_video_json(&self, document: &Html) -> Result<String> {
        let script_selector = Selector::parse("script").unwrap();
        let prefix = "window.__playinfo__=";
        for script_element in document.select(&script_selector) {
            let script = script_element.text().collect::<Vec<_>>().join("");
            if script.starts_with(prefix) {
                return Ok(script[prefix.len()..].to_owned());
            }
        }
        return Err(anyhow!("can't find video json from '{}'", self.url));
    }

    pub async fn get_video_info(&self) -> Result<VideoInfo> {
        let body_bytes = self.crawler.fetch_body(&self.url).await?;
        let body_str = std::str::from_utf8(&body_bytes)?;
        let document = Html::parse_document(body_str);
        let title = self.extract_title(&document)?;
        self.logger.info(&format!("title found as '{title}'"));
        let video_json_str = self.extract_video_json(&document)?;
        let raw_video_info: RawVideoInfo = serde_json::from_str(&video_json_str)?;
        let video_info = VideoInfo::new(title, raw_video_info)?;
        return Ok(video_info);
    }

    pub async fn download(
        &self,
        video_info: &VideoInfo,
        selected_quality_index: usize,
    ) -> Result<(String, String)> {
        let video_url = video_info.get_video_url(selected_quality_index);
        self.logger.verbose(&format!(
            "video url for '{}' is '{}'",
            video_info.quality_description[selected_quality_index], video_url
        ));
        let audio_url = video_info.get_audio_url();
        self.logger
            .verbose(&format!("audio url is '{}'", audio_url));

        let video_file_path = format!("./{}_video.mp4", video_info.title);
        self.logger.info("视频下载中");
        let video_bytes = self.crawler.fetch_body(&video_url).await?;
        let mut video_file = File::create(&video_file_path)?;
        video_file.write_all(&video_bytes)?;

        let audio_file_path = format!("./{}_audio.mp4", video_info.title);
        self.logger.info("音频下载中");
        let audio_bytes = self.crawler.fetch_body(&audio_url).await?;
        let mut audio_file = File::create(&audio_file_path)?;
        audio_file.write_all(&audio_bytes)?;

        Ok((video_file_path, audio_file_path))
    }

    pub fn merge_video_and_audio(
        &self,
        video_file_path: String,
        audio_file_path: String,
        title: String,
    ) {
        let output = Command::new("ffmpeg")
            .arg("-i")
            .arg(&video_file_path)
            .arg("-i")
            .arg(&audio_file_path)
            .arg("-c:v")
            .arg("copy")
            .arg("-c:a")
            .arg("aac")
            .arg(&format!("{title}.mp4"))
            .output()
            .expect("合并视频音频失败");
        match output.status.code() {
            Some(0) => {
                self.logger.info("视频音频合并成功");
            }
            _ => {
                self.logger.fatal("视频音频合并失败");
                self.logger.fatal(&format!(
                    "ffmpeg stderr: {:}",
                    String::from_utf8(output.stderr).unwrap()
                ))
            }
        }
        self.logger.verbose(&format!(
            "ffmpeg stdout: {:}",
            String::from_utf8(output.stdout).unwrap()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

    #[test]
    fn extract_title_success() {
        let html_str = "<html><h1>foo</h1></html>";
        let document = Html::parse_document(html_str);
        let logger = Logger::new(0);
        let crawler = Crawler::new("", &logger);
        let video = Video::new("", &crawler, &logger);
        let title = video
            .extract_title(&document)
            .expect("title should be extracted");
        assert_eq!(title, "foo");
    }

    #[test]
    fn extract_title_multiple_title() {
        let html_str = "<html><h1>bar</h1><h1>foo</h1></html>";
        let document = Html::parse_document(html_str);
        let logger = Logger::new(0);
        let crawler = Crawler::new("", &logger);
        let video = Video::new("", &crawler, &logger);
        let title = video.extract_title(&document);
        assert_eq!(title.is_ok(), false);
    }

    #[test]
    fn extract_title_no_title() {
        let html_str = "<html><h2>bar</h2><h2>foo</h2></html>";
        let document = Html::parse_document(html_str);
        let logger = Logger::new(0);
        let crawler = Crawler::new("", &logger);
        let video = Video::new("", &crawler, &logger);
        let title = video.extract_title(&document);
        assert_eq!(title.is_ok(), false);
    }
}
