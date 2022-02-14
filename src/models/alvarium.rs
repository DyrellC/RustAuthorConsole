use serde::{Serialize, Deserialize};
use crate::models::ReadingId;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AlvariumHeader {
    alg: String,
    typ: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AlvariumAnnotationPayload {
    pub iss: String,
    pub sub: String,
    pub iat: u64,
    pub jti: String,
    pub ann: String,
    pub avl: f64
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AlvariumSignature(String);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AlvariumAnnotation {
    pub id: String,
    pub key: String,
    pub hash: String,
    pub host : String,
    pub kind: String,
    pub signature : AlvariumSignature,
    #[serde(rename = "isSatisfied")]
    pub is_satisfied: bool,
    pub timestamp: String,
}

impl AlvariumAnnotation {
    pub fn get_reading_id(&self) -> ReadingId {
        ReadingId(self.key.clone())
    }

    pub fn get_confidence_score(&self) -> f64 {
        //TODO: Intercede a calculation process here
        0_f64
    }
}