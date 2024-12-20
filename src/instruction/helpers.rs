use glam::{Vec2, Vec3};
use quick_xml::de::DeError as XmlError;
use serde::de::{Deserialize, Deserializer, Error, Unexpected, Visitor};
use serde::ser::{Serialize, Serializer};
use std::fmt::Write;
use std::str::FromStr;

use super::page::ResizeBar;

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

fn shorter(a: String, b: String) -> String {
    if a.len() <= b.len() {
        a
    } else {
        b
    }
}

fn spaces(values: &[f32]) -> String {
    let mut buf = String::new();
    for &val in values {
        let mut scientific = format!("{val:E}");
        // replace E-1 with E-01, but not E-14 with E-014
        if scientific.chars().nth_back(1) == Some('-') {
            scientific.insert(scientific.len() - 1, '0');
        }
        let s = shorter(val.to_string(), scientific);
        buf.push_str(&s);
        buf.push(' ');
    }
    buf.pop();
    buf
}

serde_with::serde_conv!(
    pub(crate) Arr4Space,
    [f32; 4],
    spaces,
    |v: &str| -> Result<[f32; 4], XmlError> {
        let [x, y, z, w] = separated(v, " ").expect("TODO: better error").map(de_float::<XmlError>);
        Ok([x?, y?, z?, w?])
    }
);

serde_with::serde_conv!(
    pub(crate) Vec2Space,
    Vec2,
    |v: &Vec2| spaces(&v.to_array()),
    |v: &str| -> Result<Vec2, XmlError> {
        let [x, y] = separated(v, " ").expect("TODO: better error").map(de_float::<XmlError>);
        Ok(Vec2::new(x?, y?))
    }
);

serde_with::serde_conv!(
    pub(crate) Vec2SpaceOpt,
    Option<Vec2>,
    |v: &Option<Vec2>| spaces(&v.unwrap().to_array()),
    |v: &str| -> Result<Option<Vec2>, XmlError> {
        let [x, y] = separated(v, " ").expect("TODO: better error").map(de_float::<XmlError>);
        Ok(Some(Vec2::new(x?, y?)))
    }
);

serde_with::serde_conv!(
    pub(crate) Vec3Space,
    Vec3,
    |v: &Vec3| spaces(&v.to_array()),
    |v: &str| -> Result<Vec3, XmlError> {
        let [x, y, z] = separated(v, " ").expect("TODO: better error").map(de_float::<XmlError>);
        Ok(Vec3::new(x?, y?, z?))
    }
);

serde_with::serde_conv!(
    pub(crate) Vec3SpaceOpt,
    Option<Vec3>,
    |v: &Option<Vec3>| spaces(&v.unwrap().to_array()),
    |v: &str| -> Result<Option<Vec3>, XmlError> {
        let [x, y, z] = separated(v, " ").expect("TODO: better error").map(de_float::<XmlError>);
        Ok(Some(Vec3::new(x?, y?, z?)))
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

serde_with::serde_conv!(
    pub(crate) UpperBoolOpt,
    Option<bool>,
    |v: &Option<bool>| if *v == Some(true) { "True" } else { "False" },
    |v: &str| match v {
        "True" | "true" => Ok(Some(true)),
        "False" | "false" => Ok(Some(false)),
        _ => Err(format!("Invalid value {v:?}, expected 'True' or 'False'")),
    }
);

pub(crate) mod resize_bar_list {
    use super::*;

    pub fn serialize<S: Serializer>(list: &[ResizeBar], serializer: S) -> Result<S::Ok, S::Error> {
        let mut buf = String::new();
        for bar in list {
            let vertical = if bar.vertical { "True" } else { "False" };
            let i1 = bar.ref_index_1;
            let i2 = bar.ref_index_2;
            let offset = bar.offset;
            write!(
                buf,
                "{vertical} {i1} {i2} {} ",
                shorter(offset.to_string(), format!("{offset:.2}"))
            )
            .unwrap();
        }
        buf.pop();
        serializer.serialize_str(&buf)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<ResizeBar>, D::Error> {
        struct MyVisitor;
        impl<'de> Visitor<'de> for MyVisitor {
            type Value = Vec<ResizeBar>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a list of resize bar quadruplets, separated by spaces")
            }
            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                visit(v)
            }
        }
        de.deserialize_str(MyVisitor)
    }

    fn visit<E: Error>(v: &str) -> Result<Vec<ResizeBar>, E> {
        let mut iter = v.split_ascii_whitespace();
        let mut list = vec![];
        while let Some(vertical) = iter.next() {
            let vertical = match vertical {
                "True" | "true" => true,
                "False" | "false" => false,
                _ => {
                    return Err(E::invalid_value(
                        Unexpected::Str(vertical),
                        &"True or False",
                    ))
                }
            };

            let i1 = iter.next().ok_or_else(|| E::missing_field("refIndex1"))?;
            let ref_index_1 = i1
                .parse()
                .map_err(|_| E::invalid_value(Unexpected::Str(i1), &"an integer"))?;

            let i2 = iter.next().ok_or_else(|| E::missing_field("refIndex2"))?;
            let ref_index_2 = i2
                .parse()
                .map_err(|_| E::invalid_value(Unexpected::Str(i2), &"an integer"))?;

            let offset = iter.next().ok_or_else(|| E::missing_field("offset"))?;
            let offset = offset
                .parse()
                .map_err(|_| E::invalid_value(Unexpected::Str(offset), &"a float"))?;

            list.push(ResizeBar {
                vertical,
                ref_index_1,
                ref_index_2,
                offset,
            });
        }
        Ok(list)
    }
}
