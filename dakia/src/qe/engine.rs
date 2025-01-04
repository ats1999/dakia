use crate::error::{DakiaError, DakiaResult};

use super::query::{Composite, Map, Operator, Query, Scaler, SupplierValue, Value};

static OPERATOR_IDENTIFIRE: &str = "$";
static LOGICAL_OPERATOR: [Operator; 2] = [Operator::And, Operator::Or];

fn match_str(operator: &Operator, qstr: &str, sstr: &str) -> DakiaResult<bool> {
    let matched = match operator {
        Operator::Eq => qstr == sstr,
        Operator::Ne => qstr != sstr,
        Operator::Contains => sstr.contains(qstr),
        Operator::NotContains => !sstr.contains(qstr),
        Operator::StartsWith => sstr.starts_with(qstr),
        Operator::NotStartWith => !sstr.starts_with(qstr),
        Operator::EndsWith => sstr.ends_with(qstr),
        Operator::NotEndsWith => !sstr.ends_with(qstr),
        Operator::Matches => {
            // TODO: create regex registry after once dakia will be moved to shared nothing arch..
            // registry will allow us to reuse the same compiled regex multiple times throughout the application
            // make sure to consider thread sefty
            let regex = pcre2::bytes::Regex::new(qstr)?;
            regex.is_match(sstr.as_bytes())?
        }
        _ => {
            return Err(DakiaError::i_explain(format!(
                "Invalid operator {operator:?} for string {qstr}"
            )))
        }
    };

    Ok(matched)
}

fn match_int<T, U>(operator: &Operator, a: T, b: U) -> DakiaResult<bool>
where
    T: PartialOrd<U>,
{
    let result = match operator {
        Operator::Eq => a == b,
        Operator::Ne => a != b,
        _ => {
            return Err(DakiaError::i_explain(format!(
                "Invalid operator {operator:?} for integer argumetns "
            )))
        }
    };

    Ok(result)
}

fn match_bool(operator: &Operator, qval: bool, sval: &SupplierValue) -> DakiaResult<bool> {
    match operator {
        Operator::Exists => {
            if let SupplierValue::None = sval {
                return Ok(!qval); // not exists
            } else {
                return Ok(qval); // exists
            }
        }
        _ => {
            return Err(DakiaError::i_explain(format!(
                "Invalid operator {operator:?} for boolean argumetns "
            )))
        }
    }
}

fn is_val_in_vec(ar: &Vec<Value>, sval: &SupplierValue) -> DakiaResult<bool> {
    for val in ar.iter() {
        let matched = exec_operator(&Operator::Eq, val, sval)?;
        if matched {
            return Ok(true);
        }
    }
    Ok(false)
}

fn match_vec(operator: &Operator, vec: &Vec<Value>, sval: &SupplierValue) -> DakiaResult<bool> {
    let present = is_val_in_vec(vec, sval)?;
    let result = match operator {
        Operator::In => present,
        Operator::Nin => !present,
        _ => {
            return Err(DakiaError::i_explain(format!(
                "Invalid operator {operator:?} for array type {vec:?}!"
            )))
        }
    };

    Ok(result)
}

fn exec_operator(operator: &Operator, qval: &Value, sval: &SupplierValue) -> DakiaResult<bool> {
    match qval {
        Value::Scaler(scaler_val) => match scaler_val {
            Scaler::String(qstr) => match sval {
                SupplierValue::Str(sstr) => match_str(operator, &qstr, &sstr),
                _ => Err(DakiaError::i_explain(format!(
                    "expected string and found {sval:?} !"
                ))),
            },
            Scaler::I32(qint) => match sval {
                SupplierValue::I32(sint) => match_int(operator, qint, sint),
                _ => Err(DakiaError::i_explain(format!(
                    "expected integer and found {sval:?} !"
                ))),
            },
            Scaler::Bool(b) => match_bool(operator, *b, sval),
        },
        Value::Composite(composite_val) => match composite_val {
            Composite::Map(_) => Err(DakiaError::i_explain(format!("{qval:?} can not be map!"))),
            Composite::Array(vec) => match_vec(operator, vec, sval),
        },
    }
}

fn exec_omap<'a, F>(path: &'a str, omap: &Map, supplier: &F) -> DakiaResult<bool>
where
    F: Fn(&'a str) -> DakiaResult<SupplierValue<'a>>,
{
    let sval = supplier(&path)?;
    for (okey, qval) in omap.iter() {
        let operator = Operator::try_from(okey.as_str())?;
        if LOGICAL_OPERATOR.contains(&operator) {
            return Err(DakiaError::i_explain(
                "nested logical operator is not supported.",
            ));
        }

        let matched = exec_operator(&operator, qval, &sval)?;
        if !matched {
            return Ok(false);
        }
    }

    Ok(true)
}

fn exec_logical<'a, F>(query_map: &'a Map, and: bool, supplier: &F) -> DakiaResult<bool>
where
    F: Fn(&'a str) -> DakiaResult<SupplierValue<'a>>,
{
    for (key, value) in query_map.iter() {
        if key.starts_with(OPERATOR_IDENTIFIRE) {
            return Err(DakiaError::i_explain(
                "No operator should appear directly after logical operator.",
            ));
        }

        // scaler values are matched like $eq operator
        if let Value::Scaler(_) = value {
            let sval = supplier(key.as_str())?;
            let matched = exec_operator(&Operator::Eq, value, &sval)?;
            // if operator is $and then return false if any match is false
            if and && !matched {
                return Ok(false);
            }
            // if operator is $or then return true if any match is true
            else if matched {
                return Ok(true);
            }
        }

        // value must be operator map
        let matched = match value {
            Value::Composite(Composite::Map(omap)) => exec_omap(&key, omap, supplier),
            _ => Err(DakiaError::i_explain(format!(
                "a map was expected, found {value:?}"
            ))),
        }?;

        // if operator is $and then return false if any match is false
        if and && !matched {
            return Ok(false);
        }
        // if operator is $or then return if any match is true
        else if matched {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn exec<'a, F>(query: &'a Query, supplier: F) -> DakiaResult<bool>
where
    F: Fn(&'a str) -> DakiaResult<SupplierValue<'a>>,
{
    for (key, qval) in query.iter() {
        if key.starts_with(OPERATOR_IDENTIFIRE) {
            let operator = Operator::try_from(key.as_str())?;

            // only logical operator can be specified at the root level
            if !LOGICAL_OPERATOR.contains(&operator) {
                return Err(DakiaError::i_explain(format!(
                    "expected $or or $and, found {operator:?}!"
                )));
            }

            // value must be operator map because it's logical
            let matched = match qval {
                Value::Composite(Composite::Map(omap)) => {
                    exec_logical(omap, matches!(operator, Operator::And), &supplier)
                }
                _ => Err(DakiaError::i_explain(format!(
                    "a map was expected, found {qval:?}"
                ))),
            }?;

            if !matched {
                return Ok(false);
            }
        }

        // scaler values are matched like $eq operator
        if let Value::Scaler(_) = qval {
            let sval = supplier(key.as_str())?;
            let matched = exec_operator(&Operator::Eq, qval, &sval)?;

            if !matched {
                return Ok(false);
            }
        }

        // key is not logical operator, value is not scaler then it must be an operator map
        let matched = match qval {
            Value::Composite(Composite::Map(omap)) => exec_omap(&key, omap, &supplier),
            _ => Err(DakiaError::i_explain(format!(
                "a map was expected, found {qval:?}"
            ))),
        }?;

        if !matched {
            return Ok(false);
        }
    }

    Ok(true)
}
