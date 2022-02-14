use crate::models::SensorId;
use serde::{Deserialize, Serialize};
use anyhow::Result;


#[derive(Debug, Deserialize, Serialize)]
pub struct ReadingWrapper {
    #[serde(rename = "sensorId")]
    pub sensor_id: String,
    #[serde(rename = "readingId")]
    pub reading_id: String,
    pub reading: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Reading {
    #[serde(rename = "sensorId")]
    pub sensor_id: SensorId,
    pub value: ReadingValue,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct ReadingValue {
    #[serde(rename = "@odata.context")]
    pub context: String,
    #[serde(rename = "value")]
    pub sub_value: Vec<SubValue>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct SubValue {
    #[serde(rename = "FQN")]
    pub fqn: String,
    #[serde(rename = "DateTime")]
    pub date_time: String,
    #[serde(rename = "OpcQuality")]
    pub opc_quality: u32,
    #[serde(rename = "Value")]
    pub value: f64,
    #[serde(rename = "Text")]
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SheetReading {
    #[serde(rename = "sheetId")]
    pub sheet_id: SensorId,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SheetReadingValue(pub String);


impl Reading {
    pub fn new() -> Self {
        Reading {
            sensor_id: SensorId::default(),
            value: ReadingValue::default(),
        }
    }

    pub fn with_sensor_id(mut self, id: SensorId) -> Self {
        self.sensor_id = id;
        self
    }


    pub fn with_data(mut self, data: ReadingValue) -> Self {
        self.value = data;
        self
    }

    pub fn get_sensor_id(&self) -> &SensorId {
        &self.sensor_id
    }

    pub fn get_data(&self) -> &ReadingValue {
        &self.value
    }
}

impl SheetReading {
    pub fn get_value(&self) -> Result<SheetReadingValue> {
        let bytes = base64::decode(&self.value)?;
        let value = String::from_utf8(bytes)?;
        Ok(SheetReadingValue(value))
    }
}