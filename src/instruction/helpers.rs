use glam::Vec2;
use quick_xml::de::DeError as XmlError;
use serde::de::{Deserialize, Deserializer, Error, Unexpected, Visitor};
use serde::ser::{Serialize, Serializer};
use std::str::FromStr;

fn separated<'a, const N: usize>(string: &'a str, sep: &str) -> Option<[&'a str; N]> {
    let mut iter = string.split(sep);
    let mut array = [""; N];

    for i in 0..N {
        array[i] = iter.next()?;
    }
    if iter.next().is_some() {
        return None;
    }
    Some(array)
}

fn de_float<E: Error>(s: &str) -> Result<f32, E> {
    s.parse()
        .map_err(|_| E::invalid_value(Unexpected::Str(s), &"a float"))
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self { r, g, b, a } = self;
        serializer.serialize_str(&format!("{r}:{g}:{b}:{a}"))
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MyVisitor;
        impl<'de> Visitor<'de> for MyVisitor {
            type Value = Color;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("A Color value, formatted as an r:g:b:a string with four floats")
            }
            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                let err = || E::invalid_value(Unexpected::Str(v), &self);
                let rgba = separated(v, ":").ok_or_else(err)?;
                let [Ok(r), Ok(g), Ok(b), Ok(a)] = rgba.map(f32::from_str) else {
                    return Err(err());
                };
                Ok(Color { r, g, b, a })
            }
        }
        deserializer.deserialize_str(MyVisitor)
    }
}

serde_with::serde_conv!(
    pub(crate) Arr4Space,
    [f32; 4],
    |[x, y, z, w]: &[f32; 4]| {
        format!("{x} {y} {z} {w}")
    },
    |v: &str| -> Result<[f32; 4], XmlError> {
        let [x, y, z, w] = separated(v, " ").expect("TODO: better error").map(de_float::<XmlError>);
        Ok([x?, y?, z?, w?])
    }
);

serde_with::serde_conv!(
    pub(crate) Vec2Space,
    Vec2,
    |v: &Vec2| {
        let [x, y] = v.to_array();
        format!("{x} {y}")
    },
    |v: &str| -> Result<Vec2, XmlError> {
        let [x, y] = separated(v, " ").expect("TODO: better error").map(de_float::<XmlError>);
        Ok(Vec2::new(x?, y?))
    }
);

serde_with::serde_conv!(
    pub(crate) UpperBool,
    bool,
    |v: &bool| if *v { "True" } else { "False" },
    |v: &str| match v {
        "True" | "true" => Ok(true),
        "False" | "false" => Ok(false),
        _ => Err(format!("Invalid value {v:?}, expected 'True' or 'False'")),
    }
);
