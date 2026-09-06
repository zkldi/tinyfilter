use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Nil,
    Bool,
    Num,
    Str,
    Map,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value(pub(crate) ValueRepr);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ValueRepr {
    Nil,
    Bool(bool),
    Num(f64),
    Str(Arc<str>),
    Map(BTreeMap<String, Value>),
}

impl Default for Value {
    fn default() -> Self {
        Self::nil()
    }
}

impl Value {
    pub const fn nil() -> Self {
        Self(ValueRepr::Nil)
    }

    pub const fn bool(value: bool) -> Self {
        Self(ValueRepr::Bool(value))
    }

    pub fn num(value: f64) -> Result<Self> {
        if value.is_finite() {
            Ok(Self(ValueRepr::Num(value)))
        } else {
            Err(Error::Arithmetic("non-finite number"))
        }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self(ValueRepr::Str(Arc::from(value.into())))
    }

    pub fn map(values: impl IntoIterator<Item = (String, Self)>) -> Self {
        Self(ValueRepr::Map(values.into_iter().collect()))
    }

    pub const fn kind(&self) -> ValueKind {
        match self.0 {
            ValueRepr::Nil => ValueKind::Nil,
            ValueRepr::Bool(_) => ValueKind::Bool,
            ValueRepr::Num(_) => ValueKind::Num,
            ValueRepr::Str(_) => ValueKind::Str,
            ValueRepr::Map(_) => ValueKind::Map,
        }
    }

    pub const fn as_bool(&self) -> Option<bool> {
        if let ValueRepr::Bool(value) = self.0 {
            Some(value)
        } else {
            None
        }
    }

    pub const fn as_num(&self) -> Option<f64> {
        if let ValueRepr::Num(value) = self.0 {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let ValueRepr::Str(value) = &self.0 {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Self>> {
        if let ValueRepr::Map(value) = &self.0 {
            Some(value)
        } else {
            None
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::from(value as f64)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        assert!(
            value.is_finite(),
            "Value cannot contain a non-finite number"
        );
        Self(ValueRepr::Num(value))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::bool(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::string(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::string(value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Context {
    pub(crate) values: BTreeMap<String, Value>,
}

impl Context {
    pub fn from_values(values: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.values.insert(name.into(), value.into());
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }
}
