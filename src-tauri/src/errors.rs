use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    #[serde(skip)]
    technical: String,
}

impl AppError {
    pub fn new(code: &'static str, message: &'static str, technical: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            technical: technical.into(),
        }
    }

    pub fn technical(&self) -> &str {
        &self.technical
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::AppError;

    #[test]
    fn serializes_only_the_safe_error_contract() {
        let error = AppError::new(
            "DATABASE_UNAVAILABLE",
            "La base de datos no está disponible.",
            "secret path",
        );
        let value = serde_json::to_value(error).expect("error should serialize");

        assert_eq!(value["code"], json!("DATABASE_UNAVAILABLE"));
        assert_eq!(
            value["message"],
            json!("La base de datos no está disponible.")
        );
        assert!(value.get("technical").is_none());
        assert!(value.get("details").is_none());
    }
}
