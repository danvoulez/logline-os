use crate::errors::{LogLineError, Result};

/// Serializes a value to pretty JSON with canonical error handling.
pub fn to_pretty_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| LogLineError::SerializationError(err.to_string()))
}

/// Deserializes a JSON string into the provided type with shared error semantics.
pub fn from_json_str<'a, T: serde::de::DeserializeOwned>(input: &'a str) -> Result<T> {
    serde_json::from_str(input).map_err(|err| LogLineError::DeserializationError(err.to_string()))
}

/// Deserializes JSON bytes.
pub fn from_json_bytes<T: serde::de::DeserializeOwned>(input: &[u8]) -> Result<T> {
    serde_json::from_slice(input).map_err(|err| LogLineError::DeserializationError(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_round_trip() {
        let value = serde_json::json!({"key": "value"});
        let json = to_pretty_json(&value).expect("serialize");
        let decoded: serde_json::Value = from_json_str(&json).expect("deserialize");
        assert_eq!(decoded["key"], "value");
    }
}
