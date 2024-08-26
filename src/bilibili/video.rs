use std::{fs, path::PathBuf, process::Command};

use anyhow::{anyhow, Result};

use crate::{crawler::Fetching, logger::Logger};

use super::api::{self, bv::BVInfo};

pub struct Video<'a, F: Fetching> {
    info: BVInfo,
    title: String,
    logger: &'a Logger,
    crawler: &'a F,
}

struct VideoSource {
    title: String,
    video_url: String,
    audio_url: String,
}

impl<'a, F: Fetching> Video<'a, F> {
    pub async fn fetch_info(id: String, crawler: &'a F, logger: &'a Logger) -> Result<Self> {
        let (info, title) = api::bv::get_bv_info(crawler, logger, &id).await?;
        Ok(Self {
            logger,
            info,
            title,
            crawler,
        })
    }

    pub fn get_quality_description(&self) -> Vec<String> {
        self.info.data.accept_description.clone()
    }

    fn get_video_url(&self, selected_quality_index: usize) -> String {
        let mut max_bandwidth = 0;
        let mut url = String::new();
        let quality = self.info.data.accept_quality[selected_quality_index];
        for video in &self.info.data.dash.video {
            if video.id == quality && video.bandwidth > max_bandwidth {
                max_bandwidth = video.bandwidth;
                url = video.base_url.clone();
            }
        }
        if url.len() == 0 {
            panic!("failed to find video");
        }
        url
    }

    fn get_audio_url(&self) -> String {
        let mut max_bandwidth = 0;
        let mut url = String::new();
        for audio in &self.info.data.dash.audio {
            if audio.bandwidth > max_bandwidth {
                max_bandwidth = audio.bandwidth;
                url = audio.base_url.clone();
            }
        }
        return url;
    }

    fn select_quality(&self, selected_quality_index: usize) -> VideoSource {
        let selected_quality_description =
            self.get_quality_description()[selected_quality_index].clone();
        let video_url = self.get_video_url(selected_quality_index);
        self.logger.verbose(&format!(
            "video url for '{}' is '{}'",
            selected_quality_description, video_url
        ));

        let audio_url = self.get_audio_url();
        self.logger
            .verbose(&format!("audio url is '{}'", audio_url,));
        VideoSource {
            title: self.title.clone(),
            video_url,
            audio_url,
        }
    }

    pub fn get_best_quality_index(&self) -> usize {
        let mut max_idx = 0;
        let mut max_quality = 0;
        for (idx, quality) in self.info.data.accept_quality.iter().enumerate() {
            if *quality > max_quality {
                max_quality = *quality;
                max_idx = idx;
            }
        }
        self.logger.info(&format!("use best quality: {}", self.info.data.accept_description[max_idx]));
        max_idx
    }

    fn merge_video_and_audio(
        &self,
        video_file_path: &String,
        audio_file_path: &String,
        output_file_path: &String,
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
            .arg(output_file_path)
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

    pub async fn download(&self, selected_quality_index: usize, dir: String) -> Result<()> {
        let video_source = self.select_quality(selected_quality_index);
        let title = video_source.title.replace("/", "|");
        fs::create_dir_all(&dir)?;
        let video_file_path = format!("./{}/{}_video.mp4", dir, title);
        let audio_file_path = format!("./{}/{}_audio.mp4", dir, title);
        let output_file_path = format!("./{}/{}.mp4", dir, title);

        tokio::try_join!(
            self.crawler
                .download_to(&video_source.video_url, PathBuf::from(&video_file_path)),
            self.crawler
                .download_to(&video_source.audio_url, PathBuf::from(&audio_file_path))
        )?;
        self.merge_video_and_audio(&video_file_path, &audio_file_path, &output_file_path)?;
        self.logger.info(&format!("{title} 下载完成"));
        fs::remove_file(video_file_path)?;
        fs::remove_file(audio_file_path)?;
        Ok(())
    }
}
