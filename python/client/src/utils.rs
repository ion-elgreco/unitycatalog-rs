//! Helpers shared across the crate.

use pyo3::prelude::*;

/// The extension module, as Python imports it.
pub const CORE_MODULE: &str = "unitycatalog._core";

/// Register the submodule `unitycatalog._core.<name>` with its classes. It
/// also goes in `sys.modules`, so `import unitycatalog._core.<name>` finds it.
///
/// Usage:
/// ```ignore
/// register_submodule!(parent, "tables", [ColumnInfo, TableInfo]);
/// ```
macro_rules! register_submodule {
    ($parent:expr, $mod_name:expr, [$($type:ty),* $(,)?]) => {{
        let py = $parent.py();
        let child = pyo3::types::PyModule::new(py, $mod_name)?;
        $(
            child.add_class::<$type>()?;
        )*
        $parent.add_submodule(&child)?;
        py.import("sys")?
            .getattr("modules")?
            .set_item(format!("{}.{}", $crate::utils::CORE_MODULE, $mod_name), child)
    }};
}

/// The `repr` of a value class: `Name(field=value, …)` with the `repr` of
/// each value, without the fields that are `None`.
pub fn repr(obj: &Bound<'_, PyAny>, fields: &[&str]) -> PyResult<String> {
    let mut shown = Vec::with_capacity(fields.len());
    for field in fields {
        let value = obj.getattr(*field)?;
        if !value.is_none() {
            shown.push(format!("{field}={}", value.repr()?));
        }
    }
    Ok(format!("{}({})", obj.get_type().name()?, shown.join(", ")))
}
