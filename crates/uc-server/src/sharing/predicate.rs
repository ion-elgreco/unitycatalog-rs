//! `jsonPredicateHints` as a kernel predicate, so the kernel skips files
//! by their partition values and stats. Hints are best effort: a hint that
//! does not convert is dropped, and the client filters the rows anyway.
//!
//! A hint is a tree of ops: `column` and `literal` leaves, `isNull`,
//! comparisons (`equal`, `lessThan`, `lessThanOrEqual`, `greaterThan`,
//! `greaterThanOrEqual`), and `and`, `or`, `not`. Literals take the type of
//! the column they are compared with, from the table's schema.

use std::sync::Arc;

use delta_kernel::PredicateRef;
use delta_kernel::expressions::{Expression, Predicate, Scalar};
use delta_kernel::schema::{DataType, PrimitiveType, StructType};
use serde::Deserialize;

/// The largest hint read, as the reference server.
const MAX_HINT_BYTES: usize = 1024 * 1024;
/// The deepest hint read.
const MAX_DEPTH: usize = 100;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Op {
    op: String,
    #[serde(default)]
    children: Vec<Op>,
    name: Option<String>,
    value: Option<String>,
}

/// The kernel predicate of `hint` for a table of `schema`, if it converts.
/// The terms of a top-level `and` that do not convert are left out, which
/// keeps more files, never fewer.
pub fn from_json(hint: &str, schema: &StructType) -> Option<PredicateRef> {
    if hint.len() > MAX_HINT_BYTES {
        return None;
    }
    let op: Op = serde_json::from_str(hint).ok()?;
    let predicate = if op.op == "and" {
        let terms: Vec<Predicate> = op
            .children
            .iter()
            .filter_map(|child| predicate(child, schema, 1))
            .collect();
        match terms.len() {
            0 => return None,
            1 => terms.into_iter().next()?,
            _ => Predicate::and_from(terms),
        }
    } else {
        predicate(&op, schema, 0)?
    };
    Some(Arc::new(predicate))
}

fn predicate(op: &Op, schema: &StructType, depth: usize) -> Option<Predicate> {
    if depth > MAX_DEPTH {
        return None;
    }
    let children = || {
        op.children
            .iter()
            .map(|child| predicate(child, schema, depth + 1))
            .collect::<Option<Vec<_>>>()
    };
    match op.op.as_str() {
        "and" if op.children.len() >= 2 => Some(Predicate::and_from(children()?)),
        "or" if op.children.len() >= 2 => Some(Predicate::or_from(children()?)),
        "not" if op.children.len() == 1 => Some(Predicate::not(predicate(
            &op.children[0],
            schema,
            depth + 1,
        )?)),
        "isNull" if op.children.len() == 1 => {
            let (name, _) = column(&op.children[0], schema)?;
            Some(Predicate::is_null(Expression::column([name])))
        }
        "equal" | "lessThan" | "lessThanOrEqual" | "greaterThan" | "greaterThanOrEqual"
            if op.children.len() == 2 =>
        {
            let (left, right) = (&op.children[0], &op.children[1]);
            let (left, right) = match (left.op.as_str(), right.op.as_str()) {
                ("column", "literal") => {
                    let (name, data_type) = column(left, schema)?;
                    (Expression::column([name]), literal(right, &data_type)?)
                }
                ("literal", "column") => {
                    let (name, data_type) = column(right, schema)?;
                    (literal(left, &data_type)?, Expression::column([name]))
                }
                _ => return None,
            };
            Some(match op.op.as_str() {
                "equal" => Predicate::eq(left, right),
                "lessThan" => Predicate::lt(left, right),
                "lessThanOrEqual" => Predicate::le(left, right),
                "greaterThan" => Predicate::gt(left, right),
                _ => Predicate::ge(left, right),
            })
        }
        _ => None,
    }
}

/// The top-level primitive column a `column` op names, by its name in
/// the schema (names match without case, as Spark's).
fn column(op: &Op, schema: &StructType) -> Option<(String, PrimitiveType)> {
    if op.op != "column" {
        return None;
    }
    let name = op.name.as_deref()?;
    let field = schema.field(name).or_else(|| {
        schema
            .fields()
            .find(|f| f.name().eq_ignore_ascii_case(name))
    })?;
    match field.data_type() {
        DataType::Primitive(p) => Some((field.name().clone(), p.clone())),
        _ => None,
    }
}

/// The value of a `literal` op as a value of `data_type`. Timestamps are
/// in the protocol's format (`2022-01-01T00:00:00Z`); other values as
/// Delta writes partition values.
fn literal(op: &Op, data_type: &PrimitiveType) -> Option<Expression> {
    let value = op.value.as_deref()?;
    let scalar = match data_type {
        PrimitiveType::Timestamp | PrimitiveType::TimestampNtz => {
            let micros = chrono::DateTime::parse_from_rfc3339(value)
                .ok()?
                .timestamp_micros();
            match data_type {
                PrimitiveType::Timestamp => Scalar::Timestamp(micros),
                _ => Scalar::TimestampNtz(micros),
            }
        }
        _ => data_type.parse_scalar(value).ok()?,
    };
    Some(Expression::literal(scalar))
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta_kernel::schema::StructField;

    fn schema() -> StructType {
        StructType::try_new([
            StructField::nullable("id", DataType::LONG),
            StructField::nullable("hireDate", DataType::DATE),
            StructField::nullable("name", DataType::STRING),
            StructField::nullable("ts", DataType::TIMESTAMP),
        ])
        .unwrap()
    }

    #[test]
    fn comparisons_take_the_column_type() {
        let hint = r#"{"op":"and","children":[
            {"op":"equal","children":[
                {"op":"column","name":"hireDate","valueType":"date"},
                {"op":"literal","value":"2021-04-29","valueType":"date"}]},
            {"op":"lessThan","children":[
                {"op":"literal","value":"25","valueType":"int"},
                {"op":"column","name":"ID","valueType":"int"}]},
            {"op":"greaterThanOrEqual","children":[
                {"op":"column","name":"ts","valueType":"timestamp"},
                {"op":"literal","value":"1970-01-01T00:00:01Z","valueType":"timestamp"}]}
        ]}"#;
        let expected = Predicate::and_from([
            Predicate::eq(
                Expression::column(["hireDate"]),
                Expression::literal(Scalar::Date(18746)),
            ),
            Predicate::lt(
                Expression::literal(Scalar::Long(25)),
                Expression::column(["id"]),
            ),
            Predicate::ge(
                Expression::column(["ts"]),
                Expression::literal(Scalar::Timestamp(1_000_000)),
            ),
        ]);
        assert_eq!(from_json(hint, &schema()).as_deref(), Some(&expected));
    }

    #[test]
    fn hints_that_do_not_convert_keep_files() {
        let unknown_column = r#"{"op":"equal","children":[
            {"op":"column","name":"nope","valueType":"int"},
            {"op":"literal","value":"1","valueType":"int"}]}"#;
        assert_eq!(from_json(unknown_column, &schema()), None);
        // Only the terms of a top-level `and` may go.
        let partly = format!(
            r#"{{"op":"and","children":[{unknown_column},
                {{"op":"not","children":[{{"op":"isNull","children":[
                    {{"op":"column","name":"name","valueType":"string"}}]}}]}}]}}"#
        );
        assert_eq!(
            from_json(&partly, &schema()).as_deref(),
            Some(&Predicate::not(Predicate::is_null(Expression::column([
                "name"
            ]))))
        );
        let negated = format!(r#"{{"op":"not","children":[{partly}]}}"#);
        assert_eq!(from_json(&negated, &schema()), None);
        assert_eq!(from_json("not json", &schema()), None);
        let bad_value = r#"{"op":"equal","children":[
            {"op":"column","name":"id","valueType":"int"},
            {"op":"literal","value":"x","valueType":"int"}]}"#;
        assert_eq!(from_json(bad_value, &schema()), None);
    }
}
