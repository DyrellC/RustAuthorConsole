use anyhow::{Result, anyhow};
use std::collections::HashMap;
use crate::models::{SensorId, Reading, ReadingId, SheetReading, SheetReadingValue};

pub struct ReadingStore {
    readings: HashMap<SensorId, HashMap<ReadingId, Reading>>,
    sheet_readings: HashMap<SensorId, HashMap<ReadingId, SheetReadingValue>>
}

impl ReadingStore {
    pub fn new() -> Self {
        ReadingStore {
            readings: HashMap::<SensorId, HashMap<ReadingId, Reading>>::new(),
            sheet_readings: HashMap::<SensorId, HashMap<ReadingId, SheetReadingValue>>::new()
        }
    }

    pub fn insert_reading(&mut self, sensor_id: &SensorId, reading: Reading) -> Result<()> {
        let reading_id = ReadingId(sha256::digest(&serde_json::to_string(&reading).unwrap()));
        println!("Inserted reading {}", reading_id.0);
        match self.readings.get_mut(sensor_id) {
            Some(reading_map) => {
                if !reading.value.sub_value.is_empty() &&
                    reading_map.contains_key(&reading_id) {
                    return Err(anyhow!("Reading already exists at this location {}", reading_id.0))
                }
                reading_map.insert(reading_id, reading);
            },
            None => {
                let mut readings = HashMap::new();
                readings.insert(reading_id, reading);
                self.readings.insert(sensor_id.clone(), readings);
            },
        }
        Ok(())
    }

    pub fn insert_sheet_reading(&mut self, sensor_id: &SensorId, reading: SheetReading) -> Result<()> {
        let reading_id = ReadingId(sha256::digest(&serde_json::to_string(&reading).unwrap()));
        println!("Inserted sheet reading {}", reading_id.0);
        match self.sheet_readings.get_mut(sensor_id) {
            Some(sheet_reading_map) => {
                if sheet_reading_map.contains_key(&reading_id) {
                    return Err(anyhow!("Reading already exists at this location {}", reading_id.0))
                }
                sheet_reading_map.insert(reading_id, reading.get_value()?);
            },
            None => {
                let mut readings = HashMap::new();
                readings.insert(reading_id, reading.get_value()?);
                self.sheet_readings.insert(sensor_id.clone(), readings);
            },
        }
        Ok(())
    }

    pub fn get(&mut self, sensor_id: &SensorId) -> Result<&HashMap<ReadingId, Reading>> {
        match self.readings.get(sensor_id) {
            Some(r) => Ok(r),
            None => {
                Err(anyhow!("Key not present"))
            }

        }

    }
}
