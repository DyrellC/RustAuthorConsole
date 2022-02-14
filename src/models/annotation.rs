use crate::models::AlvariumAnnotation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Annotation {
    pub action: String,
    pub content: String,
    #[serde(rename = "messageType")]
    pub message_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnnotationWrapper {
    #[serde(rename = "readingId")]
    pub reading_id: String,
    pub annotation: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnnotationList {
    pub items: Vec<AlvariumAnnotation>
}

impl Annotation {
    pub fn get_annotations(&self) -> AnnotationList {
        let bytes = base64::decode(self.content.clone()).unwrap();
        let contents: serde_json::Result<AnnotationList> = serde_json::from_slice(&bytes);
        match contents {
            Ok(ann_list) => {
                ann_list
            },
            Err(e) => {
                println!("Error getting annotations: {}", e);
                AnnotationList { items: Vec::new() }
            }
        }
    }

    pub fn get_confidence_score(&self) -> f64 {
        /*self.annotation.payload.avl*/
        0_f64
    }
}
