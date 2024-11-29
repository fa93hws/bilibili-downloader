use crate::logger::Logger;

use anyhow::Result;
use async_trait::async_trait;
use flate2::read::GzDecoder;
use reqwest::StatusCode;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait(?Send)]
pub trait Fetching {
    async fn fetch_body(&self, url: &String) -> Result<Vec<u8>>;
    async fn download_to(&self, url: &String, output: &PathBuf) -> Result<()>;
}

pub struct Crawler<'a> {
    sess_data: String,
    logger: &'a Logger,
}

impl<'a> Crawler<'a> {
    pub fn new(sess_data: &str, logger: &'a Logger) -> Self {
        Crawler {
            sess_data: String::from(sess_data),
            logger,
        }
    }
}

#[async_trait(?Send)]
impl<'a> Fetching for Crawler<'a> {
    async fn fetch_body(&self, url: &String) -> Result<Vec<u8>> {
        let mut cookie = "CURRENT_QUALITY=32;".to_owned();
        if self.sess_data != "" {
            cookie.push_str(&format!("SESSDATA={};", self.sess_data));
        }
        let response = reqwest::Client::new().get(url)
        .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36")
        .header("referer", "https://www.bilibili.com")
        .header("cookie", cookie)
        .send()
        .await?;
        let status = response.status();
        if status != StatusCode::OK {
            self.logger
                .fatal(&format!("non 200 status: '{url}': {status}"));
            std::process::exit(1);
        } else {
            self.logger.verbose(&format!("status for'{url}': {status}"));
        }
        let encoding = match response.headers().get("Content-Encoding") {
            Some(header_value) => header_value.to_str()?.to_owned(),
            None => String::from(""),
        };
        self.logger
            .verbose(&format!("encoding is '{encoding}' for '{url}'"));

        let body_bytes = response.bytes().await?;
        if encoding == "gzip" {
            let mut reader = GzDecoder::new(&body_bytes[..]);
            let mut buf: Vec<u8> = Vec::new();
            reader.read_to_end(&mut buf)?;
            return Ok(buf);
        } else {
            Ok(Vec::from(&body_bytes[..]))
        }
    }

    async fn download_to(&self, url: &String, output: &PathBuf) -> Result<()> {
        if let Some(output_dir) = output.parent() {
            fs::create_dir_all(output_dir)?;
        };
        self.logger.verbose(&format!("downloading '{url}'"));
        let content_bytes = self.fetch_body(url).await?;
        // TODO Change to buffer
        self.logger
            .verbose(&format!("writing to '{}'", output.display()));
        let mut file = fs::File::create(output)?;
        file.write_all(&content_bytes)?;
        Ok(())
    }
}
