use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Availability {
    pub payload_available: String,
    pub payload_not_available: String,
    pub topic: String,
    pub value_template: String,
}
impl Availability {
    pub fn new(base_topic: &str) -> Self {
        Availability {
            payload_available: "online".to_string(),
            payload_not_available: "offline".to_string(),
            topic: format!("{}/availability", base_topic),
            value_template: "{{ value_json.state }}".to_string(),
        }
    }
}

impl Availability {
    pub fn set_online(&self) -> (String, String) {
        let payload = format!("{{\"state\": \"{}\"}}", self.payload_available);
        let topic = self.topic.clone();

        (topic, payload)
    }

    pub fn _set_offline(&self) -> (String, String) {
        let payload = format!("{{\"state\": \"{}\"}}", self.payload_not_available);
        let topic = self.topic.clone();

        (topic, payload)
    }
}

impl Display for Availability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_string_pretty(&self).unwrap();
        write!(f, "{}", s)
    }
}
