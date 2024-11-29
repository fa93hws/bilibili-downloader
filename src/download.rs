use std::{fs, path::PathBuf, process::Command};

use anyhow::{anyhow, Result};
use scraper::Html;

use crate::{
    bilibili::{extract_initial_state, extract_title, fetch_video_info},
    crawler::Fetching,
    logger::Logger,
};

struct VideoSource {
    title: String,
    video_url: String,
    audio_url: String,
}

pub struct Downloader<'a, F: Fetching> {
    logger: &'a Logger,
    crawler: &'a F,
}

impl<'a, F: Fetching> Downloader<'a, F> {
    pub fn new(logger: &'a Logger, crawler: &'a F) -> Self {
        Downloader { logger, crawler }
    }

    async fn fetch_html_body(&self, video_id: &str) -> Result<Html> {
        let url = format!("https://www.bilibili.com/video/{video_id}/");
        let bytes = self.crawler.fetch_body(&url).await?;
        let str = std::str::from_utf8(&bytes)?;
        Ok(Html::parse_document(str))
    }

    fn merge_video_and_audio(
        &self,
        video_path: &PathBuf,
        audio_path: &PathBuf,
        output_path: &PathBuf,
    ) -> Result<()> {
        let output = Command::new("ffmpeg")
            .arg("-i")
            .arg(&video_path)
            .arg("-i")
            .arg(&audio_path)
            .arg("-c:v")
            .arg("copy")
            .arg("-c:a")
            .arg("aac")
            .arg(output_path)
            .output()
            .expect("合并视频音频失败");

        let Some(exit_code) = output.status.code() else {
            return Err(anyhow!(
                "ffmpeg exit without exit code, stdout: {:}, stderr: {:}",
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            ));
        };
        if exit_code != 0 {
            return Err(anyhow!(
                "ffmpeg exit code: {}, stdout: {:}, stderr: {:}",
                exit_code,
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            ));
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

    async fn download_and_merge(&self, source: &VideoSource) -> Result<()> {
        let title = source.title.replace("/", "|");
        let base_dir = PathBuf::from(".");
        let video_path = base_dir
            .join("download")
            .join(format!("{}_video.mp4", title));
        let audio_path = base_dir
            .join("download")
            .join(format!("{}_audio.mp4", title));
        let output_path = base_dir.join("download").join(format!("{}.mp4", title));
        fs::create_dir_all(output_path.parent().unwrap())?;

        tokio::try_join!(
            self.crawler.download_to(&source.video_url, &video_path),
            self.crawler.download_to(&source.audio_url, &audio_path),
        )?;
        self.merge_video_and_audio(&video_path, &audio_path, &output_path)?;
        self.logger.info(&format!("{title} 下载完成"));
        fs::remove_file(video_path)?;
        fs::remove_file(audio_path)?;
        Ok(())
    }

    pub async fn download(&self, video_id: &str) -> Result<()> {
        let html = self.fetch_html_body(video_id).await?;
        let title = extract_title(&html, video_id)?;
        self.logger.info(&format!("title found as '{title}'"));
        let initial_state = extract_initial_state(&html)?;
        let video_info =
            fetch_video_info(self.crawler, &initial_state.bvid, initial_state.cid).await?;
        self.logger.info(&format!(
            "use quality: {}",
            video_info.get_hightest_quality_name()
        ));
        let source = VideoSource {
            title,
            video_url: video_info.get_best_video().base_url,
            audio_url: video_info.get_best_audio().base_url,
        };
        self.download_and_merge(&source).await?;
        Ok(())
    }
}
