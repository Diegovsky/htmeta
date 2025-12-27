use std::borrow::Cow;

use kdl::KdlValue;
use rhai::Dynamic;

use crate::{Text};
mod builtins;
pub use builtins::make_engine;

/// Possible values a variable can have.
#[derive(Debug, Clone)]
pub enum Value<'a> {
    String(Cow<'a, str>),
    Other(Dynamic),
}

impl<'a> Default for Value<'a> {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl<'a> Value<'a> {
    // pub fn map_str(self, map: impl FnOnce(Text<'a>)->Text<'a>) -> Self {
    //     match self {
    //         Self::String(text) => Self::String(map(text)),
    //         _ => self
    //     }
    // }
    pub fn as_owned(&self) -> Value<'static> {
        self.clone().into_owned()
    }
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::String(cow) => Value::String(cow.into_owned().into()),
            Value::Other(i) => Value::Other(i),
        }
    }
    pub fn as_str(&self) -> Text<'a> {
        match self {
            Value::String(cow) => cow.clone(),
            Value::Other(i) => Cow::Owned(i.to_string())
        }
    }

    pub fn into_dynamic(self) -> Dynamic {
        match self {
            Value::String(st) => Dynamic::from(st.clone().into_owned()),
            Value::Other(i) => Dynamic::from(i)
        }
    }

    pub fn as_dynamic(&self) -> Dynamic {
        self.clone().into_dynamic()
    }
}

impl<'a> From<Value<'a>> for Dynamic {
    fn from(value: Value<'a>) -> Self {
        value.into_dynamic()
    }
}

impl From<Dynamic> for Value<'static> {
    fn from(value: Dynamic) -> Self {
        match value.try_cast_result::<String>() {
            Ok(it) => Self::String(Cow::Owned(it)),
            Err(value) => Self::Other(value),
        }
    }
}

impl<'a> From<&'a KdlValue> for Value<'a> {
    fn from(value: &'a KdlValue) -> Self {
        match value {
            KdlValue::String(text) => Self::String(Cow::Borrowed(&**text)),
            KdlValue::Integer(i) => Self::Other(Dynamic::from(*i as i64)),
            KdlValue::Float(f) => Self::Other(Dynamic::from(*f)),
            KdlValue::Bool(b) => Self::Other(Dynamic::from(*b)),
            KdlValue::Null => Self::Other(Dynamic::from(())),
        }
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self::String(value)
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(value: i64) -> Self {
        Self::Other(value.into())
    }
}
