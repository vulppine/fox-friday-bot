use super::base64;
use super::parameter::Parameter;
use super::TWITTER_ENCODING;
use crypto::{hmac::Hmac, mac::Mac, sha1::Sha1};
use log::debug;
use percent_encoding::percent_encode;
use rand::RngCore;
use reqwest::{blocking::Request, header::*, Method, Url};
use std::time::SystemTime;

type SyncError = Box<dyn std::error::Error + std::marker::Sync + std::marker::Send>;

pub struct OAuthClient {
    app_key: String,
    app_secret: String,
    user_token: String,
    user_secret: String,
}

impl OAuthClient {
    pub fn new(
        app_key: String,
        app_secret: String,
        user_token: String,
        user_secret: String,
    ) -> Self {
        OAuthClient {
            app_key,
            app_secret,
            user_token,
            user_secret,
        }
    }

    // external params only exists because i don't know how to read the request's body
    pub fn auth_request(
        &self,
        mut request: Request,
        mut external_params: Vec<Parameter>,
    ) -> Result<Request, SyncError> {
        let mut parameters: Vec<Parameter> = Vec::new();

        // add all the external parameters before continuing
        parameters.append(&mut external_params);

        // add the consumer key and the token
        parameters.push(Parameter::new(
            "oauth_consumer_key".to_string(),
            self.app_key.clone(),
        ));
        parameters.push(Parameter::new(
            "oauth_token".to_string(),
            self.user_token.clone(),
        ));

        // HMAC-SHA1 is a given
        parameters.push(Parameter::new(
            "oauth_signature_method".to_string(),
            "HMAC-SHA1".to_string(),
        ));

        // push the version as well
        parameters.push(Parameter::new(
            "oauth_version".to_string(),
            "1.0".to_string(),
        ));

        // timestamp is simple enough
        let oauth_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string();
        parameters.push(Parameter::new(
            "oauth_timestamp".to_string(),
            oauth_timestamp.clone(),
        ));
        debug!("Current timestamp: {}", oauth_timestamp);

        let nonce = Self::create_nonce()?;
        debug!("Nonce length: {}", nonce.len());
        parameters.push(Parameter::new("oauth_nonce".to_string(), nonce.clone()));

        let signature = percent_encode(
            self.create_signature(request.method(), request.url(), parameters)
                .as_bytes(),
            TWITTER_ENCODING,
        )
        .to_string();

        let auth = self.create_auth(signature, nonce, oauth_timestamp);

        request.headers_mut().insert(
            HeaderName::from_bytes(b"Authorization")?,
            HeaderValue::from_str(&auth)?,
        );
        debug!("Final request: {:?}", request);

        Ok(request)
    }

    fn create_nonce() -> Result<String, std::string::FromUtf8Error> {
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        String::from_utf8(
            base64::bytes_to_base64(nonce.to_vec())
                .as_bytes()
                .iter()
                .filter(|c| !vec!['+', '=', '/'].contains(&(**c as char)))
                .cloned()
                .collect::<Vec<u8>>(),
        )
    }

    fn create_auth(&self, signature: String, nonce: String, timestamp: String) -> String {
        let mut result = String::from("OAuth ");
        result.push_str(
            vec![
                "oauth_consumer_key=\"".to_string() + self.app_key.clone().as_str() + "\"",
                "oauth_nonce=\"".to_string() + &nonce + "\"",
                "oauth_signature=\"".to_string() + &signature + "\"",
                "oauth_signature_method=\"HMAC-SHA1\"".to_string(),
                "oauth_timestamp=\"".to_string() + &timestamp + "\"",
                "oauth_token=\"".to_string() + self.user_token.clone().as_str() + "\"",
                "oauth_version=\"1.0\"".to_string(),
            ]
            .join(",")
            .as_str(),
        );
        debug!("OAuth Authorization: {}", result);

        result
    }

    fn create_signature(&self, method: &Method, url: &Url, mut params: Vec<Parameter>) -> String {
        let digest = Sha1::new();
        let key = self.app_secret.clone() + "&" + self.user_secret.as_str();
        let mut hmac = Hmac::new(digest, key.as_bytes());
        let mut base_url = url.clone();

        base_url.set_query(None);
        params.sort();

        // println!("{:?}", params);
        // println!("{}", Parameter::join(params.clone()));

        let base_string = vec![
            method.as_str().to_string(),
            base_url.to_string(),
            Parameter::join(params),
        ]
        .iter()
        .map(|v| percent_encode(v.as_bytes(), TWITTER_ENCODING).to_string())
        .collect::<Vec<String>>()
        .join("&");

        debug!("Base string: {}", base_string);

        hmac.input(base_string.as_bytes());
        base64::bytes_to_base64(hmac.result().code().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Method, Url};

    #[test]
    fn test_encoding_thing() {
        let mut parameters: Vec<Parameter> = Vec::new();

        let client = super::OAuthClient::new(
            "xvz1evFS4wEEPTGEFPHBog".to_string(),
            "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw".to_string(),
            "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb".to_string(),
            "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE".to_string(),
        );

        parameters.push(Parameter::new(
            "oauth_consumer_key".to_string(),
            client.app_key.clone(),
        ));
        parameters.push(Parameter::new(
            "oauth_token".to_string(),
            client.user_token.clone(),
        ));

        // HMAC-SHA1 is a given
        parameters.push(Parameter::new(
            "oauth_signature_method".to_string(),
            "HMAC-SHA1".to_string(),
        ));

        // push the version as well
        parameters.push(Parameter::new(
            "oauth_version".to_string(),
            "1.0".to_string(),
        ));
        parameters.push(Parameter::new(
            "oauth_timestamp".to_string(),
            "1318622958".to_string(),
        ));
        parameters.push(Parameter::new(
            "oauth_nonce".to_string(),
            "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg".to_string(),
        ));
        parameters.push(Parameter::new(
            "include_entities".to_string(),
            "true".to_string(),
        ));
        parameters.push(Parameter::new(
            "status".to_string(),
            "Hello Ladies + Gentlemen, a signed OAuth request!".to_string(),
        ));

        /*
        let signature = client.create_signature(
            &Method::POST,
            &Url::parse("https://api.twitter.com/1.1/statuses/update.json").unwrap(),
            parameters);
        */

        parameters.sort();
        let cloned_params = parameters.clone();
        let base_string = vec![
            Method::POST.as_str().to_string(),
            Url::parse("https://api.twitter.com/1.1/statuses/update.json")
                .unwrap()
                .to_string(),
            Parameter::join(parameters),
        ]
        .iter()
        .map(|v| percent_encode(v.as_bytes(), TWITTER_ENCODING).to_string())
        .collect::<Vec<String>>()
        .join("&");

        println!("{}", base_string);
        // println!("{}", client.app_secret + "&" + client.user_secret.as_str());
        assert_eq!(base_string, "POST&https%3A%2F%2Fapi.twitter.com%2F1.1%2Fstatuses%2Fupdate.json&include_entities%3Dtrue%26oauth_consumer_key%3Dxvz1evFS4wEEPTGEFPHBog%26oauth_nonce%3DkYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg%26oauth_signature_method%3DHMAC-SHA1%26oauth_timestamp%3D1318622958%26oauth_token%3D370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb%26oauth_version%3D1.0%26status%3DHello%2520Ladies%2520%252B%2520Gentlemen%252C%2520a%2520signed%2520OAuth%2520request%2521");
        let signature = client.create_signature(
            &Method::POST,
            &Url::parse("https://api.twitter.com/1.1/statuses/update.json").unwrap(),
            cloned_params,
        );
        assert_eq!(signature, "hCtSmYh+iHYCEqBWrE7C7hYmtUk=");
    }
}
