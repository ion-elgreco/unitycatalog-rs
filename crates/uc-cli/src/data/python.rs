//! `function call`, as upstream's `PythonInvoker`: `python3` runs the
//! function's body with upstream's `python_engine.py`.

use anyhow::{anyhow, bail};
use serde_json::{Value, json};
use uc_api::{ColumnTypeName, FunctionInfo};

use crate::commands::options::wire_name;
use crate::commands::{Context, upstream};

/// Upstream's engine (`etc/data/function/python_engine.py` there).
const PYTHON_ENGINE: &str = include_str!("python_engine.py");

pub async fn call_function(
    c: &Context<'_>,
    full_name: &str,
    input_params: &str,
) -> anyhow::Result<String> {
    let function: FunctionInfo = c
        .client
        .get_function(full_name)
        .await
        .map_err(upstream("getFunction"))?;
    let args: Vec<String> = input_params
        .split(',')
        .map(|a| a.trim().to_string())
        .collect();
    invoke(&function, &args).map_err(|e| anyhow!("Failed to execute function {full_name}: {e}"))
}

fn invoke(function: &FunctionInfo, args: &[String]) -> anyhow::Result<String> {
    let parameters = function
        .input_params
        .as_ref()
        .and_then(|p| p.parameters.as_deref())
        .filter(|p| !p.is_empty())
        .ok_or_else(|| anyhow!("Function parameters not found."))?;
    if args.len() < parameters.len() {
        let missing: Vec<&str> = parameters[args.len()..]
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        bail!(
            "Not enough parameters provided: {}, expected: {}, missing: [{}]",
            args.len(),
            parameters.len(),
            missing.join(", ")
        );
    }
    let mut names = Vec::new();
    let mut values = Vec::new();
    for parameter in parameters {
        names.push(parameter.name.clone());
        let position = parameter.position.unwrap_or(0).max(0) as usize;
        let argument = args
            .get(position)
            .ok_or_else(|| anyhow!("Not enough parameters provided"))?;
        values.push(match parameter.type_name {
            ColumnTypeName::Int => json!(
                argument
                    .parse::<i32>()
                    .map_err(|_| anyhow!("For input string: \"{argument}\""))?
            ),
            ColumnTypeName::Double => json!(
                argument
                    .parse::<f64>()
                    .map_err(|_| anyhow!("For input string: \"{argument}\""))?
            ),
            ColumnTypeName::String => json!(argument),
            other => bail!("Unsupported parameter type: {}", wire_name(&other)),
        });
    }
    let name = function.name.as_deref().unwrap_or_default();
    let body = function.routine_definition.as_deref().unwrap_or_default();
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(PYTHON_ENGINE)
        .arg(name)
        .arg(body)
        .arg(names.join(", "))
        .arg(Value::Array(values).to_string())
        .output()
        .map_err(|e| anyhow!("Error invoking Python script: {e}"))?;
    // Upstream reads the output line by line, each ending in a new line.
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| format!("{l}\n"))
        .collect())
}
