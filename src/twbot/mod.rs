use log::{debug, info};
use reqwest::{
    blocking::{multipart, Client},
    Url,
};
use serde::Deserialize;
// use serde_json::from_str;
use crate::oauth::{client, parameter::Parameter};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;

pub struct Bot {
    authenticator: client::OAuthClient,
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct Errors {
    errors: Vec<Error>,
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Errors {}

#[derive(Clone, Debug, Deserialize)]
pub struct Error {
    code: usize,
    name: Option<String>,
    message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Deserialize)]
pub struct Media {
    media_id: usize,
    media_id_string: String,
    expires_after_secs: Option<usize>,
    processing_info: Option<MediaStatus>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MediaStatus {
    state: String,
    check_after_secs: Option<usize>,
    progress_percent: Option<u8>,
    error: Option<Error>,
}

// pub struct Tweet {}

type SyncError = Box<dyn std::error::Error + std::marker::Sync + std::marker::Send>;

impl Bot {
    pub fn new_from_env() -> Result<Self, SyncError> {
        let app_key = match env::var("TWAPP_KEY") {
            Ok(val) => val,
            Err(e) => return Err(Box::new(e)),
        };
        let app_secret = match env::var("TWAPP_SECRET") {
            Ok(val) => val,
            Err(e) => return Err(Box::new(e)),
        };

        let user_token = match env::var("TWUSER_TOKEN") {
            Ok(val) => val,
            Err(e) => return Err(Box::new(e)),
        };
        let user_secret = match env::var("TWUSER_SECRET") {
            Ok(val) => val,
            Err(e) => return Err(Box::new(e)),
        };

        Self::new(app_key, app_secret, user_token, user_secret)
    }

    pub fn new(
        app_key: String,
        app_secret: String,
        user_token: String,
        user_secret: String,
    ) -> Result<Self, SyncError> {
        Ok(Bot {
            authenticator: client::OAuthClient::new(app_key, app_secret, user_token, user_secret),
            client: Client::builder().pool_max_idle_per_host(0).build()?,
        })
    }

    pub fn tweet_status_with_media(
        &self,
        status: String,
        media: Vec<Media>,
    ) -> Result<(), SyncError> {
        let mut parameters = vec![Parameter::new("status", status.clone())];
        let mut form = HashMap::new();
        form.insert("status", &status);

        let media_ids: String;
        if !media.is_empty() {
            media_ids = media
                .iter()
                .map(|m| m.media_id_string.clone())
                .collect::<Vec<String>>()
                .join(",");
            parameters.push(Parameter::new("media_ids", media_ids.clone()));
            form.insert("media_ids", &media_ids);
        }

        let mut request = self
            .client
            .post("https://api.twitter.com/1.1/statuses/update.json")
            .form(&form)
            .build()?;
        request = self.authenticator.auth_request(request, parameters)?;
        let response = self.client.execute(request)?;

        if !response.status().is_success() {
            let err: Errors = response.json()?;
            return Err(Box::new(err));
        }

        Ok(())
    }

    pub fn upload_media(
        &self,
        file: impl std::io::Read,
        file_len: usize,
    ) -> Result<Media, SyncError> {
        info!("Initializing media upload now.");
        let mut media = self.init_media_upload(file_len)?;
        debug!(
            "Media expiration: {} minutes",
            media.expires_after_secs.unwrap() / 60
        );
        info!("Got media ID: {}", media.media_id_string);
        sleep(Duration::from_secs(1));
        self.chunked_file_upload(file, file_len, media.media_id_string.clone())?;
        info!("Successfully uploaded media to endpoint.");
        media = self.finalize_media_upload(media.media_id_string.clone())?;
        info!("Successfully finalized media upload.");

        if media.processing_info.is_some() {
            info!("Media processing detected, waiting until finished.");
            let mut media_processing_info = media.processing_info.as_ref().unwrap().clone();
            while media_processing_info.state != "succeeded" {
                if media_processing_info.state == "failed" {
                    return Err(Box::new(media_processing_info.error.unwrap()));
                }

                sleep(Duration::from_secs(
                    media_processing_info.check_after_secs.unwrap() as u64,
                ));
                media = self.get_media_status(media.media_id_string.clone())?;
                media_processing_info = media.processing_info.as_ref().unwrap().clone();
            }
        }

        info!("Media successfully uploaded.");
        Ok(media)
    }

    fn chunked_file_upload(
        &self,
        mut file: impl std::io::Read,
        file_len: usize,
        id: String,
    ) -> Result<(), SyncError> {
        // let mut chunk_amount = (len as f64 / (1024.0 * 1000.0)).ceil();
        let mut chunk_index = 0;
        let mut buf: [u8; 1024 * 1000] = [0; 1024 * 1000];
        let mut total_bytes_read = 0; while total_bytes_read != file_len {
            let bytes_read = file.read(&mut buf[..])?;
            self.upload_media_chunk(id.clone(), chunk_index, buf[..bytes_read].to_vec())?;
            info!("Uploaded chunk {}", chunk_index);

            chunk_index += 1;
            total_bytes_read += bytes_read;
        }

        Ok(())
    }

    fn init_media_upload(&self, length: usize) -> Result<Media, SyncError> {
        let length_string = length.to_string();
        let mut form = HashMap::new();
        form.insert("command", "INIT");
        form.insert("total_bytes", &length_string);
        form.insert("media_category", "tweet_video");
        form.insert("media_type", "video/mp4");
        let parameters = vec![
            Parameter::new("command", "INIT"),
            Parameter::new("total_bytes", &length.to_string()),
            Parameter::new("media_category", "tweet_video"),
            Parameter::new("media_type", "video/mp4"),
        ];
        let mut request = self
            .client
            .post("https://upload.twitter.com/1.1/media/upload.json")
            .form(&form)
            .build()?;
        request = self.authenticator.auth_request(request, parameters)?;
        let response = self.client.execute(request)?.text()?;

        let media = serde_json::from_str(&response);

        match media {
            Ok(v) => Ok(v),
            Err(_) => {
                let err: Errors = serde_json::from_str(&response)?;
                Err(Box::new(err))
            }
        }
    }

    fn finalize_media_upload(&self, id: String) -> Result<Media, SyncError> {
        let mut form = HashMap::new();
        form.insert("command", "FINALIZE");
        form.insert("media_id", &id);
        let parameters = vec![
            Parameter::new("command", "FINALIZE"),
            Parameter::new("media_id", id.clone()),
        ];

        let mut request = self
            .client
            .post("https://upload.twitter.com/1.1/media/upload.json")
            .form(&form)
            .build()?;
        request = self.authenticator.auth_request(request, parameters)?;
        let response = self.client.execute(request)?.text()?;

        let media = serde_json::from_str(&response);

        match media {
            Ok(v) => Ok(v),
            Err(_) => {
                let err: Errors = serde_json::from_str(&response)?;
                Err(Box::new(err))
            }
        }
    }

    fn get_media_status(&self, id: String) -> Result<Media, SyncError> {
        let parameters = vec![
            Parameter::new("command", "STATUS"),
            Parameter::new("media_id", id.clone()),
        ];
        let mut request = self
            .client
            .request(
                reqwest::Method::GET,
                Url::parse_with_params(
                    "https://upload.twitter.com/1.1/media/upload.json",
                    &[("command", "STATUS"), ("media_id", id.clone().as_str())],
                )?,
            )
            .build()?;
        request = self.authenticator.auth_request(request, parameters)?;
        let response = self.client.execute(request)?.text()?;

        let media = serde_json::from_str(&response);

        match media {
            Ok(v) => Ok(v),
            Err(_) => {
                let err: Errors = serde_json::from_str(&response)?;
                Err(Box::new(err))
            }
        }
    }

    // big thanks to Keea (@keeakita) for telling me how a Cow works
    // to my head ass, saving me from an unsafe block :fox: :eye:
    fn upload_media_chunk(&self, id: String, segment: u16, buf: Vec<u8>) -> Result<(), SyncError> {
        let form = multipart::Form::new()
            .text("command", "APPEND")
            .text("media_id", id)
            .text("segment_index", segment.to_string())
            .part("media", multipart::Part::bytes(buf));

        let mut request = self
            .client
            .post("https://upload.twitter.com/1.1/media/upload.json")
            .multipart(form)
            .build()?;
        request = self.authenticator.auth_request(request, vec![])?;
        let response = self.client.execute(request)?.text()?;

        if !response.is_empty() {
            let err: Errors = serde_json::from_str(&response)?;
            return Err(Box::new(err));
        }

        Ok(())
    }
}
