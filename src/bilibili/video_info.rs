use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::crawler::Fetching;

#[derive(Serialize, Deserialize, Debug)]
pub struct AudioSpec {
    pub base_url: String,
    pub bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VideoSpec {
    pub id: u8,
    pub base_url: String,
    pub bandwidth: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DashSpec {
    pub video: Vec<VideoSpec>,
    pub audio: Vec<AudioSpec>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataSpec {
    pub accept_description: Vec<String>,
    pub accept_quality: Vec<u8>,
    pub dash: DashSpec,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VideoInfoSpec {
    pub data: DataSpec,
}

#[derive(Clone)]
pub struct Resource {
    pub base_url: String,
    pub bandwidth: u32,
}

pub struct VideoInfo {
    pub accept_description: Vec<String>,
    pub accept_quality: Vec<u8>,
    pub video: Vec<Resource>,
    pub audio: Vec<Resource>,
}

pub async fn fetch_video_info<'a, F: Fetching>(
    crawler: &F,
    bvid: &str,
    cid: i64,
) -> Result<VideoInfo> {
    let url = format!(
        "https://api.bilibili.com/x/player/wbi/playurl?bvid={}&cid={}&fnval=4048",
        bvid, cid
    );
    let body_bytes = crawler.fetch_body(&url).await?;
    let body_str = std::str::from_utf8(&body_bytes)?;
    let raw_info = serde_json::from_str::<VideoInfoSpec>(&body_str)?;
    Ok(VideoInfo {
        accept_description: raw_info.data.accept_description,
        accept_quality: raw_info.data.accept_quality,
        video: raw_info
            .data
            .dash
            .video
            .iter()
            .map(|v| Resource {
                base_url: v.base_url.clone(),
                bandwidth: v.bandwidth,
            })
            .collect(),
        audio: raw_info
            .data
            .dash
            .audio
            .iter()
            .map(|v| Resource {
                base_url: v.base_url.clone(),
                bandwidth: v.bandwidth,
            })
            .collect(),
    })
}

impl VideoInfo {
    pub fn get_hightest_quality_name(&self) -> String {
        let mut max_idx = 0;
        let mut max_quality = 0;
        for (idx, quality) in self.accept_quality.iter().enumerate() {
            if *quality > max_quality {
                max_quality = *quality;
                max_idx = idx;
            }
        }
        self.accept_description[max_idx].clone()
    }

    fn find_best_resource(&self, resources: &[Resource]) -> Resource {
        let mut max_bandwidth = 0;
        let mut best_resource_idx = 0;
        for (idx, resource) in resources.iter().enumerate() {
            if resource.bandwidth > max_bandwidth {
                max_bandwidth = resource.bandwidth;
                best_resource_idx = idx;
            }
        }
        resources[best_resource_idx].clone()
    }

    pub fn get_best_audio(&self) -> Resource {
        self.find_best_resource(&self.audio)
    }

    pub fn get_best_video(&self) -> Resource {
        self.find_best_resource(&self.video)
    }
}
