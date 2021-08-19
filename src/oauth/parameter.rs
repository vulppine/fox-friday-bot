use percent_encoding::utf8_percent_encode;
use std::cmp::Ordering;

#[derive(Clone, Debug, Eq)]
pub struct Parameter {
    key: String,
    value: String,
}

impl Parameter {
    pub fn new<T, U>(key: T, value: U) -> Self
        where T: Into<String>,
              U: Into<String>
    {
        Parameter {
            key: key.into(),
            value: value.into()
        }
    }

    // putting this here feels... messy.
    // however, it's the only way to get to the internal non-public key/value
    pub fn join(params: Vec<Self>) -> String {
        params
            .iter()
            .map(|p| {
                let key = utf8_percent_encode(&p.key, super::TWITTER_ENCODING).to_string();
                let val = utf8_percent_encode(&p.value, super::TWITTER_ENCODING).to_string();

                String::new() + key.as_str() + "=" + val.as_str()
            })
            .collect::<Vec<String>>()
            .join("&")
    }
}

impl Ord for Parameter {
    fn cmp(&self, other: &Self) -> Ordering {
        let s = vec![self.key.clone(), self.value.clone()].join("=");
        let o = vec![other.key.clone(), other.value.clone()].join("=");

        s.cmp(&o)
    }
}

impl PartialOrd for Parameter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Parameter {
    fn eq(&self, other: &Self) -> bool {
        let s = vec![self.key.clone(), self.value.clone()].join("=");
        let o = vec![other.key.clone(), other.value.clone()].join("=");

        s == o
    }
}
