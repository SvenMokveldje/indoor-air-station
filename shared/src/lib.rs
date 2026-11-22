#![no_std]
use serde::{Deserialize, Serialize};
use serde_json_core::heapless;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub seq: u32,
    pub temperature_c: f32,
    pub humidity_pct: f32,
    pub pressure_hpa: f32,
    pub co2_ppm: Option<u16>,
}

impl TryFrom<&[u8]> for Reading {
    type Error = serde_json_core::de::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json_core::from_slice(bytes).map(|(reading, _)| reading)
    }
}

impl Reading {
    pub fn to_json(&self) -> Result<heapless::Vec<u8, 256>, serde_json_core::ser::Error> {
        serde_json_core::to_vec(self)
    }
}

#[cfg(test)]
mod tests {
    use core::str::from_utf8;

    use super::*;

    #[test]
    fn to_json_serializes_all_fields() {
        let reading = Reading {
            seq: 1,
            temperature_c: 25.0,
            humidity_pct: 50.0,
            pressure_hpa: 1013.0,
            co2_ppm: None,
        };

        let json = reading.to_json().expect("serialization failed");
        let json_str = from_utf8(&json).expect("output is not valid UTF-8");

        assert_eq!(
            json_str,
            r#"{"seq":1,"temperature_c":25.0,"humidity_pct":50.0,"pressure_hpa":1013.0,"co2_ppm":null}"#
        );
    }
    #[test]
    fn try_from_deserializes_all_fields() {
        let reading = Reading {
            seq: 42,
            temperature_c: 21.4,
            humidity_pct: 45.2,
            pressure_hpa: 1013.2,
            co2_ppm: Some(415),
        };

        let json = reading.to_json().expect("serialization failed");
        let parsed = Reading::try_from(json.as_slice()).expect("deserialization failed");

        assert_eq!(parsed, reading);
    }
}
