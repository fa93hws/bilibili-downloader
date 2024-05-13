mod crawler;
mod logger;
mod video;
mod video_info;

use serde::{Deserialize, Serialize};
use std::fs;

use clap::Parser;
use crawler::Crawler;
use dialoguer::Select;
use logger::{Logger, Logging};
use video::Video;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    video_id: String,

    #[arg(short, long, default_value_t = 5)]
    log_level: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Config {
    #[serde(alias = "SESSDATA")]
    sess_data: String,
}

fn read_config(path: &str, logger: &Logger) -> Config {
    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<Config>(&contents) {
            Ok(config) => {
                logger.debug(&format!(
                    "sess_data parsed as '{}' from '{path}'",
                    config.sess_data
                ));
                return config;
            }
            Err(_) => {
                logger.warn("配置文件格式不正确，无法下载高清视频");
                return Config {
                    sess_data: "".to_owned(),
                };
            }
        },
        Err(_) => {
            logger.warn(&format!("找不到配置文件 '{path}', 无法下载高清视频"));
            return Config {
                sess_data: "".to_owned(),
            };
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
    let video = Video::new(&args.video_id, &logger, &crawler);
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
        .interact()
    {
        Ok(index) => index,
        Err(error) => {
            logger.fatal("failed to select the quality");
            panic!("{:?}", error)
        }
    };
    if let Err(e) = video.download(&video_info, selected_quality_index).await {
        logger.fatal("下载视频音频失败");
        panic!("{:?}", e)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempdir::TempDir;

    use crate::logger::Logger;
    use crate::{read_config, Config};

    #[test]
    fn read_config_config_not_exist() {
        let temp_dir = TempDir::new("read_config").unwrap();
        let temp_file = temp_dir
            .path()
            .join("non_existing")
            .to_str()
            .unwrap()
            .to_owned();
        let logger = Logger::new(0);
        let config = read_config(&temp_file, &logger);
        assert_eq!(
            config,
            Config {
                sess_data: "".to_owned(),
            },
            "sess_data should be parsed to '' if no config is presented"
        );
    }

    #[test]
    fn read_config_missing_sess_data() {
        let temp_dir = TempDir::new("read_config").unwrap();
        let temp_file = temp_dir
            .path()
            .join("missing_sess_data")
            .to_str()
            .unwrap()
            .to_owned();
        let config_content = "{}";
        fs::write(&temp_file, config_content).expect("Unable to write file");
        let logger = Logger::new(0);
        let config = read_config(&temp_file, &logger);
        assert_eq!(
            config,
            Config {
                sess_data: "".to_owned(),
            },
            "sess_data should be parsed to '' if config does not contain SESSDATA"
        );
    }

    #[test]
    fn read_config_wrong_sess_type() {
        let temp_dir = TempDir::new("read_config").unwrap();
        let temp_file = temp_dir
            .path()
            .join("wrong_sess_type")
            .to_str()
            .unwrap()
            .to_owned();
        let config_content = "{ \"SESSDATA\": 2 }";
        fs::write(&temp_file, config_content).expect("Unable to write file");
        let logger = Logger::new(0);
        let config = read_config(&temp_file, &logger);
        assert_eq!(
            config,
            Config {
                sess_data: "".to_owned(),
            },
            "sess_data should be parsed to '' if SESSDATA is not a string"
        );
    }

    #[test]
    fn read_config_success() {
        let temp_dir = TempDir::new("read_config").unwrap();
        let temp_file = temp_dir.path().join("success").to_str().unwrap().to_owned();
        let config_content = "{ \"SESSDATA\": \"2\" }";
        fs::write(&temp_file, config_content).expect("Unable to write file");
        let logger = Logger::new(0);
        let config = read_config(&temp_file, &logger);
        assert_eq!(
            config,
            Config {
                sess_data: "2".to_owned(),
            },
            "sess_data should be parsed correctly"
        );
    }
}
