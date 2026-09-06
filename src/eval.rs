mod functions;
mod operators;

use std::collections::BTreeMap;

use operators::{binary, index_value, unary};

use crate::parser::{BinaryOp, Function, Node, NodeId, Program, UnaryOp};
use crate::value::{Context, Value, ValueRepr};
use crate::{Error, LimitKind, MAX_DEPTH, Result};

#[derive(Debug, Clone, Copy)]
pub(crate) enum RuntimeValue<'a> {
    Nil,
    Bool(bool),
    Num(f64),
    Str(&'a str),
    Map(&'a BTreeMap<String, Value>),
}

impl<'a> RuntimeValue<'a> {
    fn borrowed(value: &'a Value) -> Self {
        match &value.0 {
            ValueRepr::Nil => Self::Nil,
            ValueRepr::Bool(value) => Self::Bool(*value),
            ValueRepr::Num(value) => Self::Num(*value),
            ValueRepr::Str(value) => Self::Str(value),
            ValueRepr::Map(value) => Self::Map(value),
        }
    }

    pub(crate) fn as_bool(self) -> Option<bool> {
        if let Self::Bool(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn num(value: f64) -> Result<Self> {
        if value.is_finite() {
            Ok(Self::Num(value))
        } else {
            Err(Error::Arithmetic("non-finite number"))
        }
    }

    #[cfg(test)]
    pub(crate) fn into_value(self) -> Value {
        match self {
            Self::Nil => Value::nil(),
            Self::Bool(value) => Value::bool(value),
            Self::Num(value) => Value(ValueRepr::Num(value)),
            Self::Str(value) => Value(ValueRepr::Str(value.into())),
            Self::Map(value) => Value(ValueRepr::Map(value.clone())),
        }
    }

    fn type_name(self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Bool(_) => "bool",
            Self::Num(_) => "num",
            Self::Str(_) => "str",
            Self::Map(_) => "map",
        }
    }
}

pub(crate) struct Evaluator<'a> {
    program: &'a Program,
    context: &'a Context,
}

impl<'a> Evaluator<'a> {
    pub(crate) fn new(program: &'a Program, context: &'a Context) -> Self {
        Self { program, context }
    }

    pub(crate) fn run(self) -> Result<RuntimeValue<'a>> {
        self.evaluate(self.program.root, 1)
    }

    fn evaluate(&self, id: NodeId, depth: usize) -> Result<RuntimeValue<'a>> {
        if depth > MAX_DEPTH {
            return Err(Error::limit(LimitKind::NestingDepth, MAX_DEPTH));
        }
        let program = self.program;
        match &program.nodes[id] {
            Node::Literal(value) => Ok(RuntimeValue::borrowed(value)),
            Node::Ident(name) => self
                .context
                .get(name)
                .map(RuntimeValue::borrowed)
                .ok_or(Error::UnknownVariable("unknown variable")),
            Node::Unary(operator, node) => {
                let value = self.evaluate(*node, depth + 1)?;
                unary(*operator, value)
            }
            Node::Binary(BinaryOp::And, left, right) => {
                let left = expect_bool(self.evaluate(*left, depth + 1)?)?;
                if !left {
                    Ok(RuntimeValue::Bool(false))
                } else {
                    let right = expect_bool(self.evaluate(*right, depth + 1)?)?;
                    Ok(RuntimeValue::Bool(right))
                }
            }
            Node::Binary(BinaryOp::Or, left, right) => {
                let left = expect_bool(self.evaluate(*left, depth + 1)?)?;
                if left {
                    Ok(RuntimeValue::Bool(true))
                } else {
                    let right = expect_bool(self.evaluate(*right, depth + 1)?)?;
                    Ok(RuntimeValue::Bool(right))
                }
            }
            Node::Binary(operator, left, right) => {
                let left = self.evaluate(*left, depth + 1)?;
                let right = self.evaluate(*right, depth + 1)?;
                binary(*operator, left, right)
            }
            Node::Index(value, index) => {
                let value = self.evaluate(*value, depth + 1)?;
                let index = self.evaluate(*index, depth + 1)?;
                index_value(value, index)
            }
            Node::Otherwise(value, fallback) => {
                let value = self.evaluate(*value, depth + 1)?;
                if matches!(value, RuntimeValue::Nil) {
                    self.evaluate(*fallback, depth + 1)
                } else {
                    Ok(value)
                }
            }
            Node::Conditional {
                condition,
                consequent,
                alternative,
            } => {
                if expect_bool(self.evaluate(*condition, depth + 1)?)? {
                    self.evaluate(*consequent, depth + 1)
                } else {
                    self.evaluate(*alternative, depth + 1)
                }
            }
            Node::Call { function, args } => match args.as_slice() {
                [] => function.call(&[]),
                [arg] => {
                    let values = [self.evaluate(*arg, depth + 1)?];
                    function.call(&values)
                }
                [left, right] => {
                    let values = [
                        self.evaluate(*left, depth + 1)?,
                        self.evaluate(*right, depth + 1)?,
                    ];
                    function.call(&values)
                }
                _ => Err(Error::Type("wrong number of arguments")),
            },
        }
    }
}

fn expect_bool(value: RuntimeValue<'_>) -> Result<bool> {
    match value {
        RuntimeValue::Bool(value) => Ok(value),
        _ => Err(Error::Type("expected boolean")),
    }
}
