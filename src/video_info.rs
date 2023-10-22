use std::{collections::HashSet, error::Error, fmt};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct VideoParseError {
  message: String,
}

impl Error for VideoParseError {}

impl fmt::Display for VideoParseError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{}", self.message)
  }
}
impl VideoParseError {
  pub fn new(message: String) -> Self {
      VideoParseError { message }
  }
}

#[derive(Serialize, Deserialize, Debug)]
struct Audio {
    base_url: String,
    bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Video {
    id: u8,
    base_url: String,
    bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Dash {
    video: Vec<Video>,
    audio: Vec<Audio>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    accept_description: Vec<String>,
    accept_quality: Vec<u8>,
    dash: Dash,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawVideoInfo {
    data: Data,
}

pub struct VideoInfo {
    title: String,
    pub quality_description: Vec<String>,
    pub quality: Vec<u8>,
    video: Vec<Video>,
    audio: Vec<Audio>,
}

impl VideoInfo {
    pub fn new(title: String, raw_video_info: RawVideoInfo) -> Result<Self, VideoParseError> {
        let all_quality = raw_video_info.data.accept_quality;
        let all_description = raw_video_info.data.accept_description;
        let dash = raw_video_info.data.dash;
        let mut available_quality = dash.video.iter().map(|v| v.id).collect::<HashSet<u8>>().into_iter().collect::<Vec<u8>>();
        available_quality.sort_by(|a, b| b.cmp(a));
        let mut quality_not_found: Vec<u8> = Vec::new();
        let quality_description = available_quality.iter().map(|q| {
          match all_quality.iter().position(|r| r == q) {
            Some(index) => String::from(&all_description[index]),
            None => { quality_not_found.push(*q); return "".to_owned() }
          }
        }).collect::<Vec<String>>();
        if quality_not_found.len() > 0 {
          return Err(VideoParseError::new(format!("no description found for quality id = {:?}", quality_not_found)));
        }

        Ok(VideoInfo {
            title,
            quality: available_quality,
            quality_description,
            video: dash.video,
            audio: dash.audio,
        })
    }

    pub fn get_video_url(&self, selected_quality_index: usize) -> String {
      let mut max_bandwidth = 0;
      let mut url = String::new();
      let quality = self.quality[selected_quality_index];
      for video in &self.video {
        if video.id == quality && video.bandwidth > max_bandwidth {
          max_bandwidth = video.bandwidth;
          url = video.base_url.clone();
        }
      }
      return url;
    }

    pub fn get_audio_url(&self) -> String {
      let mut max_bandwidth = 0;
      let mut url = String::new();
      for audio in &self.audio {
        if audio.bandwidth > max_bandwidth {
          max_bandwidth = audio.bandwidth;
          url = audio.base_url.clone();
        }
      }
      return url;
    }
}
