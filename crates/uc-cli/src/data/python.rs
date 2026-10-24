//! `function call`, as upstream's `PythonInvoker`: `python3` runs the
//! function's body with upstream's `python_engine.py`.

use anyhow::{anyhow, bail};
use serde_json::{Value, json};
use uc_client::models::functions::v1::FunctionInfo;
use uc_client::models::tables::v1::ColumnTypeName;

use crate::commands::options::wire_name;
use crate::commands::{Context, upstream};

/// Upstream's engine (`etc/data/function/python_engine.py` there).
const PYTHON_ENGINE: &str = include_str!("python_engine.py");

pub async fn call_function(
    c: &Context<'_>,
    full_name: &str,
    input_params: &str,
) -> anyhow::Result<String> {
    let function = c
        .client
        .function(full_name)
        .get()
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
        .as_option()
        .map(|p| p.parameters.as_slice())
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
        values.push(match parameter.type_name.as_known() {
            Some(ColumnTypeName::Int) => json!(
                argument
                    .parse::<i32>()
                    .map_err(|_| anyhow!("For input string: \"{argument}\""))?
            ),
            Some(ColumnTypeName::Double) => json!(
                argument
                    .parse::<f64>()
                    .map_err(|_| anyhow!("For input string: \"{argument}\""))?
            ),
            Some(ColumnTypeName::String) => json!(argument),
            _ => bail!(
                "Unsupported parameter type: {}",
                wire_name(&parameter.type_name)
            ),
        });
    }
    let name = function.name.as_str();
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
