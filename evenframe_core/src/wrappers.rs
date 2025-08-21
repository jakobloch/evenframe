use core::fmt;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Visitor},
};
use std::{marker::PhantomData, ops::Deref};
use surrealdb::RecordId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvenframeRecordId(pub RecordId);

impl From<String> for EvenframeRecordId {
    fn from(value: String) -> Self {
        let mut parts = value.splitn(2, ':');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("").replace(['⟨', '⟩'], "");
        EvenframeRecordId((key, val).into())
    }
}

impl Deref for EvenframeRecordId {
    type Target = RecordId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl EvenframeRecordId {
    pub fn as_inner(&self) -> &RecordId {
        &self.0
    }

    pub fn into_inner(self) -> RecordId {
        self.0
    }
}

impl fmt::Display for EvenframeRecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0.to_string().replace("⟩", "").replace("⟨", "")
        )
    }
}
impl serde::Serialize for EvenframeRecordId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Use the to_string method on the inner RecordId
        serializer.serialize_str(&self.0.to_string().replace(['⟨', '⟩'], ""))
    }
}
impl<'de> Deserialize<'de> for EvenframeRecordId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EvenframeRecordIdVisitor;

        impl<'de> Visitor<'de> for EvenframeRecordIdVisitor {
            type Value = EvenframeRecordId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter
                    .write_str("a RecordId, a string that can be parsed into a RecordId, or null")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut parts = value.splitn(2, ':');
                let key = parts.next().unwrap_or("");
                let val = parts.next().unwrap_or("").replace(['⟨', '⟩'], "");
                Ok(EvenframeRecordId((key, val).into()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                // fall back to deserializing a full RecordId struct/map
                let record_id = RecordId::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(EvenframeRecordId(record_id))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // JSON `null` → treat as empty string
                self.visit_str("no:access")
            }
        }

        deserializer.deserialize_any(EvenframeRecordIdVisitor)
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvenframePhantomData<T>(pub PhantomData<T>);

impl<T> Deref for EvenframePhantomData<T> {
    type Target = PhantomData<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> EvenframePhantomData<T> {
    pub fn new() -> Self {
        EvenframePhantomData(PhantomData)
    }

    pub fn as_inner(&self) -> &PhantomData<T> {
        &self.0
    }

    pub fn into_inner(self) -> PhantomData<T> {
        self.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvenframeValue(pub serde_value::Value);

impl Deref for EvenframeValue {
    type Target = serde_value::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl EvenframeValue {
    pub fn as_inner(&self) -> &serde_value::Value {
        &self.0
    }

    pub fn into_inner(self) -> serde_value::Value {
        self.0
    }
}

// We remove `Serialize` from the derive macro to provide a custom implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvenframeDuration(pub chrono::TimeDelta);

// Manually implement `Serialize` to control the output format.
impl Serialize for EvenframeDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // `num_nanoseconds()` returns an `Option<i64>` to prevent overflow,
        // as TimeDelta can represent a duration too large for an i64 of nanoseconds
        // (roughly +/- 292 years).
        match self.0.num_nanoseconds() {
            Some(nanos) => serializer.serialize_i64(nanos),
            None => {
                // If the duration is too large, it's best to return a serialization error.
                let msg =
                    "EvenframeDuration is too large to serialize as nanoseconds (i64 overflow)";
                Err(serde::ser::Error::custom(msg))
            }
        }
    }
}

// Your original `Deserialize` implementation remains correct.
impl<'de> Deserialize<'de> for EvenframeDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let nanos = i64::deserialize(deserializer)?;
        let td = chrono::TimeDelta::nanoseconds(nanos);
        Ok(EvenframeDuration(td))
    }
}
