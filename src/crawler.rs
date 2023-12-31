use crate::logger::Logging;

use async_trait::async_trait;
use anyhow::Result;
use flate2::read::GzDecoder;
use reqwest::StatusCode;
use std::io::Read;

#[async_trait(?Send)]
pub trait Fetching {
    async fn fetch_body(&self, url: &str) -> Result<Vec<u8>>;
}

pub struct Crawler<'a, T: Logging> {
    sess_data: String,
    logger: &'a T,
}

impl<'a, T: Logging> Crawler<'a, T> {
    pub fn new(sess_data: &str, logger: &'a T) -> Self {
        Crawler {
            sess_data: String::from(sess_data),
            logger,
        }
    }
}

#[async_trait(?Send)]
impl<'a, T: Logging> Fetching for Crawler<'a, T> {
    async fn fetch_body(&self, url: &str) -> Result<Vec<u8>> {
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
}
