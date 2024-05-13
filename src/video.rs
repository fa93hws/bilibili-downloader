use std::{path::PathBuf, process::Command};

use crate::{
    crawler::Fetching,
    logger::Logging,
    video_info::{RawVideoInfo, VideoInfo},
};
use anyhow::{anyhow, Result};
use scraper::{Html, Selector};

pub struct Video<'a, L: Logging, F: Fetching> {
    url: String,
    crawler: &'a F,
    logger: &'a L,
}

impl<'a, L: Logging, F: Fetching> Video<'a, L, F> {
    pub fn new(id: &str, logger: &'a L, crawler: &'a F) -> Self {
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
            return Ok(potential_titles[0].to_string().replace("/", "|"));
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
    ) -> Result<()> {
        let video_url = video_info.get_video_url(selected_quality_index);
        self.logger.verbose(&format!(
            "video url for '{}' is '{}'",
            video_info.quality_description[selected_quality_index], video_url
        ));
        let video_file_path = format!("./download/{}_video.mp4", video_info.title);

        let audio_url = video_info.get_audio_url();
        self.logger
            .verbose(&format!("audio url is '{}'", audio_url));
        let audio_file_path = format!("./download/{}_audio.mp4", video_info.title);

        tokio::try_join!(
            self.crawler.download_to(&video_url, PathBuf::from(&video_file_path)),
            self.crawler.download_to(&audio_url, PathBuf::from(&audio_file_path))
        )?;
        self.merge_video_and_audio(&video_file_path, &audio_file_path, &video_info.title)
    }

    fn merge_video_and_audio(
        &self,
        video_file_path: &String,
        audio_file_path: &String,
        title: &String,
    ) -> Result<()> {
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


        let Some(exit_code) = output.status.code() else {
            return Err(anyhow!("ffmpeg exit without exit code, stdout: {:}, stderr: {:}", String::from_utf8(output.stdout).unwrap(), String::from_utf8(output.stderr).unwrap()));
        };
        if exit_code != 0 {
            return Err(anyhow!("ffmpeg exit code: {}, stdout: {:}, stderr: {:}", exit_code, String::from_utf8(output.stdout).unwrap(), String::from_utf8(output.stderr).unwrap()));
        }
        self.logger.verbose(&format!(
            "ffmpeg stdout: {:}",
            String::from_utf8(output.stdout).unwrap()
        ));
        self.logger.verbose(&format!(
            "ffmpeg stderr: {:}",
            String::from_utf8(output.stderr).unwrap()
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{logger::Logger, crawler::Crawler};

    use super::*;
    use scraper::Html;

    #[test]
    fn extract_title_success() {
        let html_str = "<html><h1>foo</h1></html>";
        let document = Html::parse_document(html_str);
        let logger = Logger::new(0);
        let crawler = Crawler::new("", &logger);
        let video = Video::new("", &logger, &crawler);
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
        let video = Video::new("", &logger, &crawler);
        let title = video.extract_title(&document);
        assert_eq!(title.is_ok(), false);
    }

    #[test]
    fn extract_title_no_title() {
        let html_str = "<html><h2>bar</h2><h2>foo</h2></html>";
        let document = Html::parse_document(html_str);
        let logger = Logger::new(0);
        let crawler = Crawler::new("", &logger);
        let video = Video::new("", &logger, &crawler);
        let title = video.extract_title(&document);
        assert_eq!(title.is_ok(), false);
    }

    #[test]
    fn extract_title_slash() {
        let html_str = "<html><h1>foo/bar</h1></html>";
        let document = Html::parse_document(html_str);
        let logger = Logger::new(0);
        let crawler = Crawler::new("", &logger);
        let video = Video::new("", &logger, &crawler);
        let title = video
            .extract_title(&document)
            .expect("title should be extracted");
        assert_eq!(title, "foo|bar");
    }
}
