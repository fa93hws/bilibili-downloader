mod crawler;
mod logger;
mod video;
mod video_info;

use std::fs;
use serde::{Deserialize, Serialize};

use clap::Parser;
use crawler::Crawler;
use dialoguer::Select;
use logger::Logger;
use video::Video;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    video_id: String,

    #[arg(short, long, default_value_t = 5)]
    log_level: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    #[serde(alias = "SESSDATA")] 
    sess_data: String
}

fn read_config(path: &str, logger: &Logger) -> Config {
    match fs::read_to_string(path) {
        Ok(contents) => {
            match serde_json::from_str::<Config>(&contents) {
                Ok(config) => {
                    logger.debug(&format!("sess_data parsed as '{}' from '{path}'", config.sess_data));
                    return config;
                }
                Err(_) => {
                    logger.warn("配置文件格式不正确，无法下载高清视频");
                    return Config { sess_data: "".to_owned() };
                }
            }
        }
        Err(_) => {
            logger.warn(&format!("找不到配置文件 '{path}', 无法下载高清视频"));
            return Config { sess_data: "".to_owned() };    
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();
    let logger = Logger::new(args.log_level);
    logger.debug(&format!("args are: {:#?}", args));

    let config = read_config("./config.json", &logger);
    let crawler = Crawler::new(&config.sess_data, &logger);
    let video = Video::new(&args.video_id, &crawler, &logger);
    let video_info = match video.get_video_info().await {
        Err(error) => {
            logger.fatal("failed to find raw video info");
            panic!("{:?}", error);
        }
        Ok(val) => val,
    };
    let selected_quality_index = match Select::new()
        .with_prompt("quality?")
        .items(&video_info.quality_description)
        .default(0)
        .interact() {
            Ok(index) => index,
            Err(error) => {
                logger.fatal("failed to select the quality");
                panic!("{:?}", error)
            }
        };
    video.download(&video_info, selected_quality_index).await;
}