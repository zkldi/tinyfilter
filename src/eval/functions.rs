use super::operators::{compare_values, number};
use super::*;

macro_rules! assert_arg_count {
    ($args:ident, $count:literal) => {
        if $args.len() != $count {
            return Err(Error::Type("wrong number of arguments"));
        }
    };
}

impl Function {
    pub(super) fn call<'a>(self, args: &[RuntimeValue<'a>]) -> Result<RuntimeValue<'a>> {
        let value = match self {
            Self::Num => {
                assert_arg_count!(args, 1);
                let value = match args[0] {
                    RuntimeValue::Num(value) => value,
                    RuntimeValue::Str(value) => {
                        if value.contains('e') || value.contains('E') {
                            return Err(Error::Type("string is not a number"));
                        }
                        value
                            .parse()
                            .map_err(|_| Error::Type("string is not a number"))?
                    }
                    _ => return Err(Error::Type("num() requires a number or string")),
                };
                RuntimeValue::num(value)?
            }
            Self::Type => {
                assert_arg_count!(args, 1);
                RuntimeValue::Str(args[0].type_name())
            }
            Self::Contains => {
                assert_arg_count!(args, 2);
                RuntimeValue::Bool(string_arg(args[0])?.contains(string_arg(args[1])?))
            }
            Self::StartsWith => {
                assert_arg_count!(args, 2);
                RuntimeValue::Bool(string_arg(args[0])?.starts_with(string_arg(args[1])?))
            }
            Self::EndsWith => {
                assert_arg_count!(args, 2);
                RuntimeValue::Bool(string_arg(args[0])?.ends_with(string_arg(args[1])?))
            }
            Self::Min => {
                assert_arg_count!(args, 2);
                validate_numbers(args)?;
                if compare_values(args[0], args[1])?.is_le() {
                    args[0]
                } else {
                    args[1]
                }
            }
            Self::Max => {
                assert_arg_count!(args, 2);
                validate_numbers(args)?;
                if compare_values(args[0], args[1])?.is_ge() {
                    args[0]
                } else {
                    args[1]
                }
            }
            Self::Abs => {
                assert_arg_count!(args, 1);
                RuntimeValue::num(number(args[0])?.abs())?
            }
            Self::Ceil => {
                assert_arg_count!(args, 1);
                RuntimeValue::num(number(args[0])?.ceil())?
            }
            Self::Floor => {
                assert_arg_count!(args, 1);
                RuntimeValue::num(number(args[0])?.floor())?
            }
            Self::Round => {
                assert_arg_count!(args, 1);
                RuntimeValue::num(number(args[0])?.round())?
            }
        };
        Ok(value)
    }
}

fn string_arg<'a>(value: RuntimeValue<'a>) -> Result<&'a str> {
    if let RuntimeValue::Str(value) = value {
        Ok(value)
    } else {
        Err(Error::Type("expected string argument"))
    }
}

fn validate_numbers(values: &[RuntimeValue<'_>]) -> Result<()> {
    if values
        .iter()
        .all(|value| matches!(value, RuntimeValue::Num(_)))
    {
        Ok(())
    } else {
        Err(Error::Type("expected number arguments"))
    }
}
