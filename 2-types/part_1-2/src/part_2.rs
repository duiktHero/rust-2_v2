use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum RequestType {
    Success,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    #[serde(rename = "type")]
    pub request_type: RequestType,
    pub stream: Stream,
    pub gifts: Vec<Gift>,
    pub debug: DebugInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stream {
    pub user_id: uuid::Uuid,
    pub is_private: bool,
    pub settings: i64,
    pub shard_url: url::Url,
    pub public_tariff: Tariff,
    pub private_tariff: Tariff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tariff {
    pub id: Option<i64>,           
    pub client_price: Option<i64>, 
    pub price: Option<i64>,
    #[serde(with = "humantime_serde")]
    pub duration: std::time::Duration,  
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gift {
    pub id: i64,
    pub price: i64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugInfo {
    #[serde(with = "humantime_serde")]
    pub duration: std::time::Duration,  
    #[serde(with = "time::serde::rfc3339")]
    pub at: time::OffsetDateTime,      
}

pub const REQUEST_JSON_PATH: &str = "../../request.json";

pub fn load_request_at(path: &Path) -> Result<Request, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let req: Request = serde_json::from_reader(reader)?;
    Ok(req)
}


pub fn load_request() -> Result<Request, Box<dyn std::error::Error>> {
    load_request_at(Path::new(REQUEST_JSON_PATH))
}

pub fn to_toml(request: &Request) -> Result<String, Box<dyn std::error::Error>> {
    Ok(toml::to_string_pretty(request)?)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_print_toml() {
        let req = load_request().unwrap();
        assert_eq!(req.stream.public_tariff.duration, "1h");
        assert_eq!(req.stream.private_tariff.duration, "1m");

        let toml_text = to_toml(&req).unwrap();
        assert!(toml_text.contains("[stream.public_tariff]"));
        assert!(toml_text.contains("duration = \"1h\""));
        assert!(toml_text.contains("[stream.private_tariff]"));
        assert!(toml_text.contains("duration = \"1m\""));
    }
}
