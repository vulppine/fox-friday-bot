use lambda_runtime::{handler_fn, run, Context, Error};
use simple_logger::SimpleLogger;

mod oauth;
mod twbot;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .unwrap();
    run(handler_fn(tweet_fox_video)).await?;

    Ok(())
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Event {
    foo: String,
} // don't care

#[derive(serde::Serialize)]
struct Response {
    msg: &'static str,
} // don't care

async fn tweet_fox_video(_: Event, _: Context) -> Result<Response, Error> {
    let bot = twbot::Bot::new_from_env()?;
    let fox_video = s3::Bucket::new(
        "fox-friday-bot-bucket",
        s3::Region::UsWest1,
        s3::creds::Credentials::from_env()?,
    )?
    .get_object("/fox_friday.mp4")
    .await?;

    log::info!("Got fox video: code: {}, len: {}", fox_video.1, fox_video.0.len());

    let media = bot.upload_media(fox_video.0.as_slice(), fox_video.0.len())?;

    bot.tweet_status_with_media(String::from(""), vec![media])?;

    let resp = Response { msg: "OK" };

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization() {
        SimpleLogger::new()
            .with_level(log::LevelFilter::Info)
            .env()
            .init()
            .unwrap();

        let bot = twbot::Bot::new_from_env().unwrap();
        let fox_video = std::fs::File::open("./fox_friday.mp4").unwrap();
        let fox_video_len = fox_video.metadata().unwrap().len() as usize;

        bot.upload_media(fox_video, fox_video_len).unwrap();
    }
}
