use std::collections::HashMap;

use zvariant::{OwnedValue, Value};

use crate::model::{SoundHint, Urgency};

/// Raw RGBA image carried by the `image-data` hint.
#[derive(Clone, Debug)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub rowstride: u32,
    pub bits_per_sample: u8,
    pub channels: u8,
    pub data: Vec<u8>,
}

/// Everything the daemon cares about that arrives through the `hints` map.
#[derive(Clone, Debug, Default)]
pub struct ParsedHints {
    pub urgency: Urgency,
    pub category: Option<String>,
    pub desktop_entry: Option<String>,
    pub image_path: Option<String>,
    pub image_data: Option<ImageData>,
    pub sound: SoundHint,
    pub resident: bool,
    pub transient: bool,
}

impl ParsedHints {
    pub fn parse(hints: &HashMap<String, OwnedValue>) -> Self {
        let mut parsed = Self {
            urgency: hint_value(hints, "urgency")
                .and_then(u8_of)
                .map(Urgency::from_hint)
                .unwrap_or_default(),
            category: hint_string(hints, "category"),
            desktop_entry: hint_string(hints, "desktop-entry"),
            image_path: hint_string(hints, "image-path")
                .or_else(|| hint_string(hints, "image_path")),
            image_data: hint_value(hints, "image-data")
                .or_else(|| hint_value(hints, "image_data"))
                .and_then(image_data_of),
            resident: hint_value(hints, "resident").and_then(bool_of).unwrap_or(false),
            transient: hint_value(hints, "transient").and_then(bool_of).unwrap_or(false),
            sound: SoundHint::default(),
        };

        parsed.sound = if hint_value(hints, "suppress-sound")
            .and_then(bool_of)
            .unwrap_or(false)
        {
            SoundHint::Suppress
        } else if let Some(file) = hint_string(hints, "sound-file") {
            SoundHint::File(file)
        } else if let Some(name) = hint_string(hints, "sound-name") {
            SoundHint::Name(name)
        } else {
            SoundHint::Default
        };

        parsed
    }
}

fn hint_value<'a>(
    hints: &'a HashMap<String, OwnedValue>,
    key: &str,
) -> Option<&'a Value<'static>> {
    hints.get(key).map(|value| &**value)
}

fn hint_string(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    hint_value(hints, key).and_then(str_of)
}

fn str_of(value: &Value<'_>) -> Option<String> {
    value.downcast_ref::<&str>().ok().map(str::to_owned)
}

fn u8_of(value: &Value<'_>) -> Option<u8> {
    value.downcast_ref::<u8>().ok()
}

fn bool_of(value: &Value<'_>) -> Option<bool> {
    value.downcast_ref::<bool>().ok()
}

fn i32_of(value: &Value<'_>) -> Option<i32> {
    value.downcast_ref::<i32>().ok()
}

fn image_data_of(value: &Value<'_>) -> Option<ImageData> {
    let Value::Structure(structure) = value else {
        return None;
    };

    let fields = structure.fields();
    if fields.len() < 7 {
        return None;
    }

    let data = fields[6]
        .downcast_ref::<zvariant::Array>()
        .ok()
        .map(|array| {
            array
                .iter()
                .filter_map(|value| value.downcast_ref::<u8>().ok())
                .collect()
        })
        .unwrap_or_default();

    Some(ImageData {
        width: i32_of(&fields[0])?.max(0) as u32,
        height: i32_of(&fields[1])?.max(0) as u32,
        rowstride: i32_of(&fields[2])?.max(0) as u32,
        bits_per_sample: i32_of(&fields[4])?.max(0) as u8,
        channels: i32_of(&fields[5])?.max(0) as u8,
        data,
    })
}
