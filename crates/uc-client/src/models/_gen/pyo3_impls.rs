// @generated — do not edit by hand.
/// `CredentialPurpose` of Python: a member of `unitycatalog._enums.CredentialPurpose`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyCredentialPurpose(pub super::credentials::v1::CredentialPurpose);
impl PyCredentialPurpose {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::credentials::v1::CredentialPurpose as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("SERVICE_PRINCIPAL"),
            2i32 => ::core::option::Option::Some("STORAGE"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::credentials::v1::CredentialPurpose> for PyCredentialPurpose {
    fn from(value: super::credentials::v1::CredentialPurpose) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyCredentialPurpose> for super::credentials::v1::CredentialPurpose {
    fn from(value: PyCredentialPurpose) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyCredentialPurpose {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "SERVICE_PRINCIPAL" => 1i32,
            "STORAGE" => 2i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid CredentialPurpose: {:?}; expected one of: SERVICE_PRINCIPAL, STORAGE",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::credentials::v1::CredentialPurpose as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyCredentialPurpose {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "CredentialPurpose")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "CredentialPurpose value {} has no Python member",
                <super::credentials::v1::CredentialPurpose as ::buffa::Enumeration>::to_i32(
                    &self.0
                )
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyCredentialPurpose {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.CredentialPurpose".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.CredentialPurpose | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `ModelVersionOperation` of Python: a member of `unitycatalog._enums.ModelVersionOperation`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyModelVersionOperation(pub super::credentials::v1::ModelVersionOperation);
impl PyModelVersionOperation {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::credentials::v1::ModelVersionOperation as ::buffa::Enumeration>::to_i32(
            &self.0,
        ) {
            1i32 => ::core::option::Option::Some("UNKNOWN_MODEL_VERSION_OPERATION"),
            2i32 => ::core::option::Option::Some("READ_MODEL_VERSION"),
            3i32 => ::core::option::Option::Some("READ_WRITE_MODEL_VERSION"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::credentials::v1::ModelVersionOperation>
    for PyModelVersionOperation
{
    fn from(value: super::credentials::v1::ModelVersionOperation) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyModelVersionOperation>
    for super::credentials::v1::ModelVersionOperation
{
    fn from(value: PyModelVersionOperation) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyModelVersionOperation {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "UNKNOWN_MODEL_VERSION_OPERATION" => 1i32,
            "READ_MODEL_VERSION" => 2i32,
            "READ_WRITE_MODEL_VERSION" => 3i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid ModelVersionOperation: {:?}; expected one of: UNKNOWN_MODEL_VERSION_OPERATION, READ_MODEL_VERSION, READ_WRITE_MODEL_VERSION",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::credentials::v1::ModelVersionOperation as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyModelVersionOperation {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "ModelVersionOperation")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "ModelVersionOperation value {} has no Python member",
                <super::credentials::v1::ModelVersionOperation as ::buffa::Enumeration>::to_i32(
                    &self.0
                )
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyModelVersionOperation {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ModelVersionOperation".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ModelVersionOperation | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `PathOperation` of Python: a member of `unitycatalog._enums.PathOperation`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyPathOperation(pub super::credentials::v1::PathOperation);
impl PyPathOperation {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::credentials::v1::PathOperation as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("UNKNOWN_PATH_OPERATION"),
            2i32 => ::core::option::Option::Some("PATH_READ"),
            3i32 => ::core::option::Option::Some("PATH_READ_WRITE"),
            4i32 => ::core::option::Option::Some("PATH_CREATE_TABLE"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::credentials::v1::PathOperation> for PyPathOperation {
    fn from(value: super::credentials::v1::PathOperation) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyPathOperation> for super::credentials::v1::PathOperation {
    fn from(value: PyPathOperation) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyPathOperation {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "UNKNOWN_PATH_OPERATION" => 1i32,
            "PATH_READ" => 2i32,
            "PATH_READ_WRITE" => 3i32,
            "PATH_CREATE_TABLE" => 4i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid PathOperation: {:?}; expected one of: UNKNOWN_PATH_OPERATION, PATH_READ, PATH_READ_WRITE, PATH_CREATE_TABLE",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::credentials::v1::PathOperation as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyPathOperation {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "PathOperation")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "PathOperation value {} has no Python member",
                <super::credentials::v1::PathOperation as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyPathOperation {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.PathOperation".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.PathOperation | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `TableOperation` of Python: a member of `unitycatalog._enums.TableOperation`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyTableOperation(pub super::credentials::v1::TableOperation);
impl PyTableOperation {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::credentials::v1::TableOperation as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("UNKNOWN_TABLE_OPERATION"),
            2i32 => ::core::option::Option::Some("READ"),
            3i32 => ::core::option::Option::Some("READ_WRITE"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::credentials::v1::TableOperation> for PyTableOperation {
    fn from(value: super::credentials::v1::TableOperation) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyTableOperation> for super::credentials::v1::TableOperation {
    fn from(value: PyTableOperation) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyTableOperation {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "UNKNOWN_TABLE_OPERATION" => 1i32,
            "READ" => 2i32,
            "READ_WRITE" => 3i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid TableOperation: {:?}; expected one of: UNKNOWN_TABLE_OPERATION, READ, READ_WRITE",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::credentials::v1::TableOperation as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyTableOperation {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "TableOperation")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "TableOperation value {} has no Python member",
                <super::credentials::v1::TableOperation as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyTableOperation {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.TableOperation".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.TableOperation | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `VolumeOperation` of Python: a member of `unitycatalog._enums.VolumeOperation`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyVolumeOperation(pub super::credentials::v1::VolumeOperation);
impl PyVolumeOperation {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::credentials::v1::VolumeOperation as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("UNKNOWN_VOLUME_OPERATION"),
            2i32 => ::core::option::Option::Some("READ_VOLUME"),
            3i32 => ::core::option::Option::Some("WRITE_VOLUME"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::credentials::v1::VolumeOperation> for PyVolumeOperation {
    fn from(value: super::credentials::v1::VolumeOperation) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyVolumeOperation> for super::credentials::v1::VolumeOperation {
    fn from(value: PyVolumeOperation) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyVolumeOperation {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "UNKNOWN_VOLUME_OPERATION" => 1i32,
            "READ_VOLUME" => 2i32,
            "WRITE_VOLUME" => 3i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid VolumeOperation: {:?}; expected one of: UNKNOWN_VOLUME_OPERATION, READ_VOLUME, WRITE_VOLUME",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::credentials::v1::VolumeOperation as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyVolumeOperation {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "VolumeOperation")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "VolumeOperation value {} has no Python member",
                <super::credentials::v1::VolumeOperation as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyVolumeOperation {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.VolumeOperation".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.VolumeOperation | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `DeltaCredentialOperation` of Python: a member of `unitycatalog._enums.DeltaCredentialOperation`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyDeltaCredentialOperation(pub super::delta::v1::DeltaCredentialOperation);
impl PyDeltaCredentialOperation {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::delta::v1::DeltaCredentialOperation as ::buffa::Enumeration>::to_i32(&self.0)
        {
            1i32 => ::core::option::Option::Some("READ"),
            2i32 => ::core::option::Option::Some("READ_WRITE"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::delta::v1::DeltaCredentialOperation>
    for PyDeltaCredentialOperation
{
    fn from(value: super::delta::v1::DeltaCredentialOperation) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaCredentialOperation>
    for super::delta::v1::DeltaCredentialOperation
{
    fn from(value: PyDeltaCredentialOperation) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyDeltaCredentialOperation {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "READ" => 1i32,
            "READ_WRITE" => 2i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid DeltaCredentialOperation: {:?}; expected one of: READ, READ_WRITE",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::delta::v1::DeltaCredentialOperation as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyDeltaCredentialOperation {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "DeltaCredentialOperation")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "DeltaCredentialOperation value {} has no Python member",
                <super::delta::v1::DeltaCredentialOperation as ::buffa::Enumeration>::to_i32(
                    &self.0
                )
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyDeltaCredentialOperation {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DeltaCredentialOperation".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DeltaCredentialOperation | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `DeltaMaintenanceOperation` of Python: a member of `unitycatalog._enums.DeltaMaintenanceOperation`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyDeltaMaintenanceOperation(pub super::delta::v1::DeltaMaintenanceOperation);
impl PyDeltaMaintenanceOperation {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::delta::v1::DeltaMaintenanceOperation as ::buffa::Enumeration>::to_i32(&self.0)
        {
            1i32 => ::core::option::Option::Some("DATA_REORGANIZATION"),
            2i32 => ::core::option::Option::Some("DATA_CLEANUP"),
            3i32 => ::core::option::Option::Some("METADATA_CLEANUP"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::delta::v1::DeltaMaintenanceOperation>
    for PyDeltaMaintenanceOperation
{
    fn from(value: super::delta::v1::DeltaMaintenanceOperation) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaMaintenanceOperation>
    for super::delta::v1::DeltaMaintenanceOperation
{
    fn from(value: PyDeltaMaintenanceOperation) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyDeltaMaintenanceOperation {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "DATA_REORGANIZATION" => 1i32,
            "DATA_CLEANUP" => 2i32,
            "METADATA_CLEANUP" => 3i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid DeltaMaintenanceOperation: {:?}; expected one of: DATA_REORGANIZATION, DATA_CLEANUP, METADATA_CLEANUP",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::delta::v1::DeltaMaintenanceOperation as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyDeltaMaintenanceOperation {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "DeltaMaintenanceOperation")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "DeltaMaintenanceOperation value {} has no Python member",
                <super::delta::v1::DeltaMaintenanceOperation as ::buffa::Enumeration>::to_i32(
                    &self.0
                )
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyDeltaMaintenanceOperation {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DeltaMaintenanceOperation".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DeltaMaintenanceOperation | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `DeltaTableType` of Python: a member of `unitycatalog._enums.DeltaTableType`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyDeltaTableType(pub super::delta::v1::DeltaTableType);
impl PyDeltaTableType {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::delta::v1::DeltaTableType as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("MANAGED"),
            2i32 => ::core::option::Option::Some("EXTERNAL"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::delta::v1::DeltaTableType> for PyDeltaTableType {
    fn from(value: super::delta::v1::DeltaTableType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaTableType> for super::delta::v1::DeltaTableType {
    fn from(value: PyDeltaTableType) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyDeltaTableType {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "MANAGED" => 1i32,
            "EXTERNAL" => 2i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid DeltaTableType: {:?}; expected one of: MANAGED, EXTERNAL",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::delta::v1::DeltaTableType as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyDeltaTableType {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "DeltaTableType")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "DeltaTableType value {} has no Python member",
                <super::delta::v1::DeltaTableType as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyDeltaTableType {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DeltaTableType".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DeltaTableType | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `FunctionParameterMode` of Python: a member of `unitycatalog._enums.FunctionParameterMode`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyFunctionParameterMode(pub super::functions::v1::FunctionParameterMode);
impl PyFunctionParameterMode {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::functions::v1::FunctionParameterMode as ::buffa::Enumeration>::to_i32(&self.0)
        {
            1i32 => ::core::option::Option::Some("IN"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::functions::v1::FunctionParameterMode>
    for PyFunctionParameterMode
{
    fn from(value: super::functions::v1::FunctionParameterMode) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyFunctionParameterMode>
    for super::functions::v1::FunctionParameterMode
{
    fn from(value: PyFunctionParameterMode) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyFunctionParameterMode {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "IN" => 1i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid FunctionParameterMode: {:?}; expected one of: IN",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::functions::v1::FunctionParameterMode as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyFunctionParameterMode {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "FunctionParameterMode")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "FunctionParameterMode value {} has no Python member",
                <super::functions::v1::FunctionParameterMode as ::buffa::Enumeration>::to_i32(
                    &self.0
                )
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyFunctionParameterMode {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.FunctionParameterMode".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.FunctionParameterMode | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `FunctionParameterType` of Python: a member of `unitycatalog._enums.FunctionParameterType`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyFunctionParameterType(pub super::functions::v1::FunctionParameterType);
impl PyFunctionParameterType {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::functions::v1::FunctionParameterType as ::buffa::Enumeration>::to_i32(&self.0)
        {
            1i32 => ::core::option::Option::Some("PARAM"),
            2i32 => ::core::option::Option::Some("COLUMN"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::functions::v1::FunctionParameterType>
    for PyFunctionParameterType
{
    fn from(value: super::functions::v1::FunctionParameterType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyFunctionParameterType>
    for super::functions::v1::FunctionParameterType
{
    fn from(value: PyFunctionParameterType) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyFunctionParameterType {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "PARAM" => 1i32,
            "COLUMN" => 2i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid FunctionParameterType: {:?}; expected one of: PARAM, COLUMN",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::functions::v1::FunctionParameterType as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyFunctionParameterType {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "FunctionParameterType")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "FunctionParameterType value {} has no Python member",
                <super::functions::v1::FunctionParameterType as ::buffa::Enumeration>::to_i32(
                    &self.0
                )
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyFunctionParameterType {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.FunctionParameterType".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.FunctionParameterType | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `ParameterStyle` of Python: a member of `unitycatalog._enums.ParameterStyle`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyParameterStyle(pub super::functions::v1::ParameterStyle);
impl PyParameterStyle {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::functions::v1::ParameterStyle as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("S"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::functions::v1::ParameterStyle> for PyParameterStyle {
    fn from(value: super::functions::v1::ParameterStyle) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyParameterStyle> for super::functions::v1::ParameterStyle {
    fn from(value: PyParameterStyle) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyParameterStyle {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "S" => 1i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!("invalid ParameterStyle: {:?}; expected one of: S", other),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::functions::v1::ParameterStyle as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyParameterStyle {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "ParameterStyle")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "ParameterStyle value {} has no Python member",
                <super::functions::v1::ParameterStyle as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyParameterStyle {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ParameterStyle".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ParameterStyle | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `RoutineBody` of Python: a member of `unitycatalog._enums.RoutineBody`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyRoutineBody(pub super::functions::v1::RoutineBody);
impl PyRoutineBody {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::functions::v1::RoutineBody as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("SQL"),
            2i32 => ::core::option::Option::Some("EXTERNAL"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::functions::v1::RoutineBody> for PyRoutineBody {
    fn from(value: super::functions::v1::RoutineBody) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyRoutineBody> for super::functions::v1::RoutineBody {
    fn from(value: PyRoutineBody) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyRoutineBody {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "SQL" => 1i32,
            "EXTERNAL" => 2i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid RoutineBody: {:?}; expected one of: SQL, EXTERNAL",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::functions::v1::RoutineBody as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyRoutineBody {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "RoutineBody")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "RoutineBody value {} has no Python member",
                <super::functions::v1::RoutineBody as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyRoutineBody {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.RoutineBody".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.RoutineBody | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `SecurityType` of Python: a member of `unitycatalog._enums.SecurityType`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PySecurityType(pub super::functions::v1::SecurityType);
impl PySecurityType {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::functions::v1::SecurityType as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("DEFINER"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::functions::v1::SecurityType> for PySecurityType {
    fn from(value: super::functions::v1::SecurityType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySecurityType> for super::functions::v1::SecurityType {
    fn from(value: PySecurityType) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PySecurityType {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "DEFINER" => 1i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid SecurityType: {:?}; expected one of: DEFINER",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::functions::v1::SecurityType as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PySecurityType {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "SecurityType")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "SecurityType value {} has no Python member",
                <super::functions::v1::SecurityType as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PySecurityType {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.SecurityType".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.SecurityType | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `SqlDataAccess` of Python: a member of `unitycatalog._enums.SqlDataAccess`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PySqlDataAccess(pub super::functions::v1::SqlDataAccess);
impl PySqlDataAccess {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::functions::v1::SqlDataAccess as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("CONTAINS_SQL"),
            2i32 => ::core::option::Option::Some("READS_SQL_DATA"),
            3i32 => ::core::option::Option::Some("NO_SQL"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::functions::v1::SqlDataAccess> for PySqlDataAccess {
    fn from(value: super::functions::v1::SqlDataAccess) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySqlDataAccess> for super::functions::v1::SqlDataAccess {
    fn from(value: PySqlDataAccess) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PySqlDataAccess {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "CONTAINS_SQL" => 1i32,
            "READS_SQL_DATA" => 2i32,
            "NO_SQL" => 3i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid SqlDataAccess: {:?}; expected one of: CONTAINS_SQL, READS_SQL_DATA, NO_SQL",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::functions::v1::SqlDataAccess as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PySqlDataAccess {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "SqlDataAccess")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "SqlDataAccess value {} has no Python member",
                <super::functions::v1::SqlDataAccess as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PySqlDataAccess {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.SqlDataAccess".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.SqlDataAccess | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `ModelVersionStatus` of Python: a member of `unitycatalog._enums.ModelVersionStatus`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyModelVersionStatus(pub super::models::v1::ModelVersionStatus);
impl PyModelVersionStatus {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::models::v1::ModelVersionStatus as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("MODEL_VERSION_STATUS_UNKNOWN"),
            2i32 => ::core::option::Option::Some("PENDING_REGISTRATION"),
            3i32 => ::core::option::Option::Some("FAILED_REGISTRATION"),
            4i32 => ::core::option::Option::Some("READY"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::models::v1::ModelVersionStatus> for PyModelVersionStatus {
    fn from(value: super::models::v1::ModelVersionStatus) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyModelVersionStatus> for super::models::v1::ModelVersionStatus {
    fn from(value: PyModelVersionStatus) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyModelVersionStatus {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "MODEL_VERSION_STATUS_UNKNOWN" => 1i32,
            "PENDING_REGISTRATION" => 2i32,
            "FAILED_REGISTRATION" => 3i32,
            "READY" => 4i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid ModelVersionStatus: {:?}; expected one of: MODEL_VERSION_STATUS_UNKNOWN, PENDING_REGISTRATION, FAILED_REGISTRATION, READY",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::models::v1::ModelVersionStatus as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyModelVersionStatus {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "ModelVersionStatus")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "ModelVersionStatus value {} has no Python member",
                <super::models::v1::ModelVersionStatus as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyModelVersionStatus {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ModelVersionStatus".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ModelVersionStatus | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `AuthenticationType` of Python: a member of `unitycatalog._enums.AuthenticationType`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyAuthenticationType(pub super::recipients::v1::AuthenticationType);
impl PyAuthenticationType {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::recipients::v1::AuthenticationType as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("TOKEN"),
            2i32 => ::core::option::Option::Some("OAUTH_CLIENT_CREDENTIALS"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::recipients::v1::AuthenticationType> for PyAuthenticationType {
    fn from(value: super::recipients::v1::AuthenticationType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAuthenticationType> for super::recipients::v1::AuthenticationType {
    fn from(value: PyAuthenticationType) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyAuthenticationType {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "TOKEN" => 1i32,
            "OAUTH_CLIENT_CREDENTIALS" => 2i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid AuthenticationType: {:?}; expected one of: TOKEN, OAUTH_CLIENT_CREDENTIALS",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::recipients::v1::AuthenticationType as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyAuthenticationType {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "AuthenticationType")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "AuthenticationType value {} has no Python member",
                <super::recipients::v1::AuthenticationType as ::buffa::Enumeration>::to_i32(
                    &self.0
                )
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyAuthenticationType {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.AuthenticationType".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.AuthenticationType | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `ColumnTypeName` of Python: a member of `unitycatalog._enums.ColumnTypeName`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyColumnTypeName(pub super::tables::v1::ColumnTypeName);
impl PyColumnTypeName {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::tables::v1::ColumnTypeName as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("BOOLEAN"),
            2i32 => ::core::option::Option::Some("BYTE"),
            3i32 => ::core::option::Option::Some("SHORT"),
            4i32 => ::core::option::Option::Some("INT"),
            5i32 => ::core::option::Option::Some("LONG"),
            6i32 => ::core::option::Option::Some("FLOAT"),
            7i32 => ::core::option::Option::Some("DOUBLE"),
            8i32 => ::core::option::Option::Some("DATE"),
            9i32 => ::core::option::Option::Some("TIMESTAMP"),
            10i32 => ::core::option::Option::Some("TIMESTAMP_NTZ"),
            11i32 => ::core::option::Option::Some("STRING"),
            12i32 => ::core::option::Option::Some("BINARY"),
            13i32 => ::core::option::Option::Some("DECIMAL"),
            14i32 => ::core::option::Option::Some("INTERVAL"),
            15i32 => ::core::option::Option::Some("ARRAY"),
            16i32 => ::core::option::Option::Some("STRUCT"),
            17i32 => ::core::option::Option::Some("MAP"),
            18i32 => ::core::option::Option::Some("VARIANT"),
            19i32 => ::core::option::Option::Some("CHAR"),
            20i32 => ::core::option::Option::Some("NULL"),
            21i32 => ::core::option::Option::Some("USER_DEFINED_TYPE"),
            22i32 => ::core::option::Option::Some("TABLE_TYPE"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::tables::v1::ColumnTypeName> for PyColumnTypeName {
    fn from(value: super::tables::v1::ColumnTypeName) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyColumnTypeName> for super::tables::v1::ColumnTypeName {
    fn from(value: PyColumnTypeName) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyColumnTypeName {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "BOOLEAN" => 1i32,
            "BYTE" => 2i32,
            "SHORT" => 3i32,
            "INT" => 4i32,
            "LONG" => 5i32,
            "FLOAT" => 6i32,
            "DOUBLE" => 7i32,
            "DATE" => 8i32,
            "TIMESTAMP" => 9i32,
            "TIMESTAMP_NTZ" => 10i32,
            "STRING" => 11i32,
            "BINARY" => 12i32,
            "DECIMAL" => 13i32,
            "INTERVAL" => 14i32,
            "ARRAY" => 15i32,
            "STRUCT" => 16i32,
            "MAP" => 17i32,
            "VARIANT" => 18i32,
            "CHAR" => 19i32,
            "NULL" => 20i32,
            "USER_DEFINED_TYPE" => 21i32,
            "TABLE_TYPE" => 22i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid ColumnTypeName: {:?}; expected one of: BOOLEAN, BYTE, SHORT, INT, LONG, FLOAT, DOUBLE, DATE, TIMESTAMP, TIMESTAMP_NTZ, STRING, BINARY, DECIMAL, INTERVAL, ARRAY, STRUCT, MAP, VARIANT, CHAR, NULL, USER_DEFINED_TYPE, TABLE_TYPE",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::tables::v1::ColumnTypeName as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyColumnTypeName {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "ColumnTypeName")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "ColumnTypeName value {} has no Python member",
                <super::tables::v1::ColumnTypeName as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyColumnTypeName {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ColumnTypeName".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.ColumnTypeName | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `DataSourceFormat` of Python: a member of `unitycatalog._enums.DataSourceFormat`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyDataSourceFormat(pub super::tables::v1::DataSourceFormat);
impl PyDataSourceFormat {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::tables::v1::DataSourceFormat as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("DELTA"),
            2i32 => ::core::option::Option::Some("ICEBERG"),
            3i32 => ::core::option::Option::Some("CSV"),
            4i32 => ::core::option::Option::Some("JSON"),
            5i32 => ::core::option::Option::Some("AVRO"),
            6i32 => ::core::option::Option::Some("PARQUET"),
            7i32 => ::core::option::Option::Some("ORC"),
            8i32 => ::core::option::Option::Some("TEXT"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::tables::v1::DataSourceFormat> for PyDataSourceFormat {
    fn from(value: super::tables::v1::DataSourceFormat) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDataSourceFormat> for super::tables::v1::DataSourceFormat {
    fn from(value: PyDataSourceFormat) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyDataSourceFormat {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "DELTA" => 1i32,
            "ICEBERG" => 2i32,
            "CSV" => 3i32,
            "JSON" => 4i32,
            "AVRO" => 5i32,
            "PARQUET" => 6i32,
            "ORC" => 7i32,
            "TEXT" => 8i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid DataSourceFormat: {:?}; expected one of: DELTA, ICEBERG, CSV, JSON, AVRO, PARQUET, ORC, TEXT",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::tables::v1::DataSourceFormat as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyDataSourceFormat {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "DataSourceFormat")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "DataSourceFormat value {} has no Python member",
                <super::tables::v1::DataSourceFormat as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyDataSourceFormat {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DataSourceFormat".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.DataSourceFormat | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `TableType` of Python: a member of `unitycatalog._enums.TableType`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyTableType(pub super::tables::v1::TableType);
impl PyTableType {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::tables::v1::TableType as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("MANAGED"),
            2i32 => ::core::option::Option::Some("EXTERNAL"),
            3i32 => ::core::option::Option::Some("STREAMING_TABLE"),
            4i32 => ::core::option::Option::Some("MATERIALIZED_VIEW"),
            5i32 => ::core::option::Option::Some("METRIC_VIEW"),
            6i32 => ::core::option::Option::Some("VIEW"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::tables::v1::TableType> for PyTableType {
    fn from(value: super::tables::v1::TableType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyTableType> for super::tables::v1::TableType {
    fn from(value: PyTableType) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyTableType {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "MANAGED" => 1i32,
            "EXTERNAL" => 2i32,
            "STREAMING_TABLE" => 3i32,
            "MATERIALIZED_VIEW" => 4i32,
            "METRIC_VIEW" => 5i32,
            "VIEW" => 6i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid TableType: {:?}; expected one of: MANAGED, EXTERNAL, STREAMING_TABLE, MATERIALIZED_VIEW, METRIC_VIEW, VIEW",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::tables::v1::TableType as ::buffa::Enumeration>::from_i32(n).unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyTableType {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "TableType")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "TableType value {} has no Python member",
                <super::tables::v1::TableType as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyTableType {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.TableType".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.TableType | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// `VolumeType` of Python: a member of `unitycatalog._enums.VolumeType`, or its name.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PyVolumeType(pub super::volumes::v1::VolumeType);
impl PyVolumeType {
    /// The name of the value, or `None` for a value without a Python member.
    fn __name(&self) -> ::core::option::Option<&'static str> {
        match <super::volumes::v1::VolumeType as ::buffa::Enumeration>::to_i32(&self.0) {
            1i32 => ::core::option::Option::Some("MANAGED"),
            2i32 => ::core::option::Option::Some("EXTERNAL"),
            _ => ::core::option::Option::None,
        }
    }
}

impl ::core::convert::From<super::volumes::v1::VolumeType> for PyVolumeType {
    fn from(value: super::volumes::v1::VolumeType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyVolumeType> for super::volumes::v1::VolumeType {
    fn from(value: PyVolumeType) -> Self {
        value.0
    }
}

impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for PyVolumeType {
    type Error = ::pyo3::PyErr;
    fn extract(obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
        let name: ::pyo3::pybacked::PyBackedStr = ::pyo3::types::PyAnyMethods::extract(&*obj)?;
        let n: i32 = match &*name {
            "MANAGED" => 1i32,
            "EXTERNAL" => 2i32,
            other => {
                return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                    ::std::format!(
                        "invalid VolumeType: {:?}; expected one of: MANAGED, EXTERNAL",
                        other
                    ),
                ));
            }
        };
        ::core::result::Result::Ok(Self(
            <super::volumes::v1::VolumeType as ::buffa::Enumeration>::from_i32(n)
                .unwrap_or_default(),
        ))
    }
}

impl<'py> ::pyo3::IntoPyObject<'py> for PyVolumeType {
    type Target = ::pyo3::PyAny;
    type Output = ::pyo3::Bound<'py, ::pyo3::PyAny>;
    type Error = ::pyo3::PyErr;
    fn into_pyobject(self, py: ::pyo3::Python<'py>) -> ::pyo3::PyResult<Self::Output> {
        static CLASS: ::pyo3::sync::PyOnceLock<::pyo3::Py<::pyo3::types::PyType>> =
            ::pyo3::sync::PyOnceLock::new();
        let class = CLASS.import(py, "unitycatalog._enums", "VolumeType")?;
        let name = self.__name().ok_or_else(|| {
            ::pyo3::exceptions::PyValueError::new_err(::std::format!(
                "VolumeType value {} has no Python member",
                <super::volumes::v1::VolumeType as ::buffa::Enumeration>::to_i32(&self.0)
            ))
        })?;
        ::pyo3::types::PyAnyMethods::getattr(class.as_any(), name)
    }
}

impl ::pyo3_stub_gen::PyStubType for PyVolumeType {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.VolumeType".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }

    fn type_input() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo {
            name: "_enums.VolumeType | str".to_string(),
            source_module: None,
            import: ::std::collections::HashSet::from(["unitycatalog._enums".into()]),
            type_refs: ::std::collections::HashMap::new(),
        }
    }
}

/// A snapshot of the whole catalog: the rows of each table of the store. The audit log is not in
/// it.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "BackupResponse",
    eq,
    frozen,
    module = "unitycatalog._core.admin",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyBackupResponse(pub super::admin::v1::BackupResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyBackupResponse {
    /// Version of the snapshot format. A restore takes only version 1.
    #[getter]
    fn version(&self) -> i64 {
        self.0.version
    }

    /// The rows of each table, by table name. A row is a JSON object of its columns, as the table
    /// of the store has them.
    #[gen_stub(
        override_return_type(
            type_repr = "dict[str, list[typing.Any]]",
            imports = ("typing")
        )
    )]
    #[getter]
    fn tables(&self, py: ::pyo3::Python<'_>) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
        let dict = ::pyo3::types::PyDict::new(py);
        for (key, value) in &self.0.tables {
            ::pyo3::types::PyDictMethods::set_item(&dict, key, json_list_to_py(py, value)?)?;
        }
        ::core::result::Result::Ok(dict.into_any().unbind())
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("version", false), ("tables", false)])
    }
}

impl ::core::convert::From<super::admin::v1::BackupResponse> for PyBackupResponse {
    fn from(value: super::admin::v1::BackupResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyBackupResponse> for super::admin::v1::BackupResponse {
    fn from(value: PyBackupResponse) -> Self {
        value.0
    }
}

/// What a restore imported.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "RestoreResponse",
    eq,
    frozen,
    module = "unitycatalog._core.admin",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyRestoreResponse(pub super::admin::v1::RestoreResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyRestoreResponse {
    /// Number of rows imported, by table name.
    #[getter]
    fn imported(&self) -> ::std::collections::HashMap<::std::string::String, i32> {
        self.0.imported.clone()
    }

    /// The rows left out, each as `<table>: <reason>`, such as a row without its key.
    #[getter]
    fn skipped(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.skipped.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("imported", false), ("skipped", false)])
    }
}

impl ::core::convert::From<super::admin::v1::RestoreResponse> for PyRestoreResponse {
    fn from(value: super::admin::v1::RestoreResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyRestoreResponse> for super::admin::v1::RestoreResponse {
    fn from(value: PyRestoreResponse) -> Self {
        value.0
    }
}

/// One request that could change the catalog: a `POST`, `PATCH`, `PUT` or `DELETE`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AuditEvent",
    eq,
    frozen,
    module = "unitycatalog._core.audit",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAuditEvent(pub super::audit::v1::AuditEvent);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAuditEvent {
    /// Unique identifier of the event.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Username of the principal that sent the request, or `anonymous`.
    #[getter]
    fn principal(&self) -> ::std::string::String {
        self.0.principal.clone()
    }

    /// HTTP method of the request, such as `POST` or `DELETE`.
    #[getter]
    fn method(&self) -> ::std::string::String {
        self.0.method.clone()
    }

    /// Path of the request, with the query string.
    #[getter]
    fn path(&self) -> ::std::string::String {
        self.0.path.clone()
    }

    /// HTTP status code of the response.
    #[getter]
    fn status(&self) -> i32 {
        self.0.status
    }

    /// Time at which the response was sent, in epoch milliseconds.
    #[getter]
    fn timestamp(&self) -> i64 {
        self.0.timestamp
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("id", false),
                ("principal", false),
                ("method", false),
                ("path", false),
                ("status", false),
                ("timestamp", false),
            ],
        )
    }
}

impl ::core::convert::From<super::audit::v1::AuditEvent> for PyAuditEvent {
    fn from(value: super::audit::v1::AuditEvent) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAuditEvent> for super::audit::v1::AuditEvent {
    fn from(value: PyAuditEvent) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListAuditEventsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.audit",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListAuditEventsResponse(pub super::audit::v1::ListAuditEventsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListAuditEventsResponse {
    /// The events, newest first.
    #[getter]
    fn events(&self) -> ::std::vec::Vec<PyAuditEvent> {
        self.0
            .events
            .iter()
            .cloned()
            .map(PyAuditEvent::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("events", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::audit::v1::ListAuditEventsResponse>
    for PyListAuditEventsResponse
{
    fn from(value: super::audit::v1::ListAuditEventsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListAuditEventsResponse>
    for super::audit::v1::ListAuditEventsResponse
{
    fn from(value: PyListAuditEventsResponse) -> Self {
        value.0
    }
}

/// How the web UI signs in, and where its "Log out" goes.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AuthMode",
    eq,
    frozen,
    module = "unitycatalog._core.auth",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAuthMode(pub super::auth::v1::AuthMode);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAuthMode {
    /// How the web UI signs in: `oidc` (with the OpenID Connect provider of the server), `forward`
    /// (through an auth proxy in front of the server), `none` (no sign-in in the browser) or `off`
    /// (authorization is off, and no one signs in). `uc web` answers `token`: the browser uses the
    /// token of the `uc` commands.
    #[getter]
    fn mode(&self) -> ::std::string::String {
        self.0.mode.clone()
    }

    /// Where "Log out" goes. Absent: the web UI hides "Log out".
    #[getter]
    fn logout_url(&self) -> ::core::option::Option<::std::string::String> {
        self.0.logout_url.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("mode", false), ("logout_url", false)])
    }
}

impl ::core::convert::From<super::auth::v1::AuthMode> for PyAuthMode {
    fn from(value: super::auth::v1::AuthMode) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAuthMode> for super::auth::v1::AuthMode {
    fn from(value: PyAuthMode) -> Self {
        value.0
    }
}

/// An OAuth 2.0 token exchange response (RFC 8693): a Unity Catalog access token.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "OAuthTokenExchangeInfo",
    eq,
    frozen,
    module = "unitycatalog._core.auth",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyOAuthTokenExchangeInfo(pub super::auth::v1::OAuthTokenExchangeInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyOAuthTokenExchangeInfo {
    /// The Unity Catalog access token.
    #[getter]
    fn access_token(&self) -> ::std::string::String {
        self.0.access_token.clone()
    }

    /// The type of the issued token: `urn:ietf:params:oauth:token-type:access_token`.
    #[getter]
    fn issued_token_type(&self) -> ::std::string::String {
        self.0.issued_token_type.clone()
    }

    /// How to send the token: `Bearer`.
    #[getter]
    fn token_type(&self) -> ::std::string::String {
        self.0.token_type.clone()
    }

    /// Lifetime of the access token, in seconds.
    #[getter]
    fn expires_in(&self) -> i64 {
        self.0.expires_in
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("access_token", true),
                ("issued_token_type", false),
                ("token_type", false),
                ("expires_in", false),
            ],
        )
    }
}

impl ::core::convert::From<super::auth::v1::OAuthTokenExchangeInfo> for PyOAuthTokenExchangeInfo {
    fn from(value: super::auth::v1::OAuthTokenExchangeInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyOAuthTokenExchangeInfo> for super::auth::v1::OAuthTokenExchangeInfo {
    fn from(value: PyOAuthTokenExchangeInfo) -> Self {
        value.0
    }
}

/// An OAuth 2.0 token response (RFC 6749, section 5.1). The access token is the personal access
/// token itself.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "TokenResponse",
    eq,
    frozen,
    module = "unitycatalog._core.auth",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTokenResponse(pub super::auth::v1::TokenResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyTokenResponse {
    /// The access token: the personal access token of the request.
    #[getter]
    fn access_token(&self) -> ::std::string::String {
        self.0.access_token.clone()
    }

    /// How to send the token: `Bearer`.
    #[getter]
    fn token_type(&self) -> ::std::string::String {
        self.0.token_type.clone()
    }

    /// Seconds until the token expires: the time until the token of `client_id` expires, else 3600.
    #[getter]
    fn expires_in(&self) -> i64 {
        self.0.expires_in
    }

    /// The scope of the request.
    #[getter]
    fn scope(&self) -> ::core::option::Option<::std::string::String> {
        self.0.scope.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("access_token", true),
                ("token_type", false),
                ("expires_in", false),
                ("scope", false),
            ],
        )
    }
}

impl ::core::convert::From<super::auth::v1::TokenResponse> for PyTokenResponse {
    fn from(value: super::auth::v1::TokenResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyTokenResponse> for super::auth::v1::TokenResponse {
    fn from(value: PyTokenResponse) -> Self {
        value.0
    }
}

/// A catalog: the first layer of the three-level namespace. It holds schemas.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "CatalogInfo",
    eq,
    frozen,
    module = "unitycatalog._core.catalogs",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyCatalogInfo(pub super::catalogs::v1::CatalogInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyCatalogInfo {
    /// Name of the catalog.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Key-value properties of the catalog.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    /// Username of the owner of the catalog.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the catalog was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the catalog.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the catalog was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the catalog.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the catalog.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Storage root URL for the managed storage of the catalog, such as `s3://bucket/ucroot`.
    #[getter]
    fn storage_root(&self) -> ::core::option::Option<::std::string::String> {
        self.0.storage_root.clone()
    }

    /// Storage location URL (full path) of the managed storage of the catalog.
    #[getter]
    fn storage_location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.storage_location.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("comment", false),
                ("properties", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("id", false),
                ("storage_root", false),
                ("storage_location", false),
            ],
        )
    }
}

impl ::core::convert::From<super::catalogs::v1::CatalogInfo> for PyCatalogInfo {
    fn from(value: super::catalogs::v1::CatalogInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyCatalogInfo> for super::catalogs::v1::CatalogInfo {
    fn from(value: PyCatalogInfo) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListCatalogsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.catalogs",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListCatalogsResponse(pub super::catalogs::v1::ListCatalogsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListCatalogsResponse {
    /// The catalogs.
    #[getter]
    fn catalogs(&self) -> ::std::vec::Vec<PyCatalogInfo> {
        self.0
            .catalogs
            .iter()
            .cloned()
            .map(PyCatalogInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("catalogs", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::catalogs::v1::ListCatalogsResponse> for PyListCatalogsResponse {
    fn from(value: super::catalogs::v1::ListCatalogsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListCatalogsResponse> for super::catalogs::v1::ListCatalogsResponse {
    fn from(value: PyListCatalogsResponse) -> Self {
        value.0
    }
}

/// Temporary AWS credentials.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AwsCredentials",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAwsCredentials(pub super::credentials::v1::AwsCredentials);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAwsCredentials {
    /// The access key ID of the temporary credentials.
    #[getter]
    fn access_key_id(&self) -> ::std::string::String {
        self.0.access_key_id.clone()
    }

    /// The secret access key of the temporary credentials.
    #[getter]
    fn secret_access_key(&self) -> ::std::string::String {
        self.0.secret_access_key.clone()
    }

    /// The session token of the temporary credentials.
    #[getter]
    fn session_token(&self) -> ::std::string::String {
        self.0.session_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("access_key_id", false),
                ("secret_access_key", true),
                ("session_token", true),
            ],
        )
    }
}

impl ::core::convert::From<super::credentials::v1::AwsCredentials> for PyAwsCredentials {
    fn from(value: super::credentials::v1::AwsCredentials) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAwsCredentials> for super::credentials::v1::AwsCredentials {
    fn from(value: PyAwsCredentials) -> Self {
        value.0
    }
}

/// The AWS IAM role of a new or changed credential.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AwsIamRoleRequest",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAwsIamRoleRequest(pub super::credentials::v1::AwsIamRoleRequest);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAwsIamRoleRequest {
    #[new]
    #[pyo3(signature = (role_arn))]
    fn new(role_arn: ::std::string::String) -> Self {
        let mut inner =
            <super::credentials::v1::AwsIamRoleRequest as ::core::default::Default>::default();
        {
            let value = role_arn;
            inner.role_arn = value;
        }
        Self(inner)
    }

    /// ARN of the AWS IAM role that the server assumes to make temporary credentials.
    #[getter]
    fn role_arn(&self) -> ::std::string::String {
        self.0.role_arn.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("role_arn", false)])
    }
}

impl ::core::convert::From<super::credentials::v1::AwsIamRoleRequest> for PyAwsIamRoleRequest {
    fn from(value: super::credentials::v1::AwsIamRoleRequest) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAwsIamRoleRequest> for super::credentials::v1::AwsIamRoleRequest {
    fn from(value: PyAwsIamRoleRequest) -> Self {
        value.0
    }
}

/// The AWS IAM role of a credential.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AwsIamRoleResponse",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAwsIamRoleResponse(pub super::credentials::v1::AwsIamRoleResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAwsIamRoleResponse {
    /// ARN of the AWS IAM role that the server assumes to make temporary credentials.
    #[getter]
    fn role_arn(&self) -> ::std::string::String {
        self.0.role_arn.clone()
    }

    /// ARN of the server's own AWS role, which assumes `role_arn`. Absent when the server has no
    /// role of its own.
    #[getter]
    fn unity_catalog_iam_arn(&self) -> ::core::option::Option<::std::string::String> {
        self.0.unity_catalog_iam_arn.clone()
    }

    /// External ID that the server sends when it assumes the role. The trust policy of the role
    /// should require it. Each new role gets a new external ID.
    #[getter]
    fn external_id(&self) -> ::std::string::String {
        self.0.external_id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("role_arn", false),
                ("unity_catalog_iam_arn", false),
                ("external_id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::credentials::v1::AwsIamRoleResponse> for PyAwsIamRoleResponse {
    fn from(value: super::credentials::v1::AwsIamRoleResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAwsIamRoleResponse> for super::credentials::v1::AwsIamRoleResponse {
    fn from(value: PyAwsIamRoleResponse) -> Self {
        value.0
    }
}

/// The Azure managed identity of a credential: the server's own Azure identity.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AzureManagedIdentity",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAzureManagedIdentity(pub super::credentials::v1::AzureManagedIdentity);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAzureManagedIdentity {
    #[new]
    #[pyo3(signature = (*, managed_identity_id = None))]
    fn new(managed_identity_id: ::core::option::Option<::std::string::String>) -> Self {
        let mut inner =
            <super::credentials::v1::AzureManagedIdentity as ::core::default::Default>::default();
        {
            let value = managed_identity_id;
            inner.managed_identity_id = value;
        }
        Self(inner)
    }

    /// Azure resource ID of a user-assigned managed identity, which the server gets from the
    /// instance metadata service. Absent: the identity that the server's environment gives, such as
    /// AKS workload identity or the managed identity of the VM.
    #[getter]
    fn managed_identity_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.managed_identity_id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("managed_identity_id", false)])
    }
}

impl ::core::convert::From<super::credentials::v1::AzureManagedIdentity>
    for PyAzureManagedIdentity
{
    fn from(value: super::credentials::v1::AzureManagedIdentity) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAzureManagedIdentity>
    for super::credentials::v1::AzureManagedIdentity
{
    fn from(value: PyAzureManagedIdentity) -> Self {
        value.0
    }
}

/// The Azure service principal of a new or changed credential.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AzureServicePrincipalRequest",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAzureServicePrincipalRequest(pub super::credentials::v1::AzureServicePrincipalRequest);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAzureServicePrincipalRequest {
    #[new]
    #[pyo3(signature = (directory_id, application_id, client_secret))]
    fn new(
        directory_id: ::std::string::String,
        application_id: ::std::string::String,
        client_secret: ::std::string::String,
    ) -> Self {
        let mut inner = <super::credentials::v1::AzureServicePrincipalRequest as ::core::default::Default>::default();
        {
            let value = directory_id;
            inner.directory_id = value;
        }
        {
            let value = application_id;
            inner.application_id = value;
        }
        {
            let value = client_secret;
            inner.client_secret = value;
        }
        Self(inner)
    }

    /// ID of the Microsoft Entra ID tenant (directory) of the application.
    #[getter]
    fn directory_id(&self) -> ::std::string::String {
        self.0.directory_id.clone()
    }

    /// Application (client) ID of the service principal.
    #[getter]
    fn application_id(&self) -> ::std::string::String {
        self.0.application_id.clone()
    }

    /// Client secret of the application. The server stores it and never returns it.
    #[getter]
    fn client_secret(&self) -> ::std::string::String {
        self.0.client_secret.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("directory_id", false),
                ("application_id", false),
                ("client_secret", true),
            ],
        )
    }
}

impl ::core::convert::From<super::credentials::v1::AzureServicePrincipalRequest>
    for PyAzureServicePrincipalRequest
{
    fn from(value: super::credentials::v1::AzureServicePrincipalRequest) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAzureServicePrincipalRequest>
    for super::credentials::v1::AzureServicePrincipalRequest
{
    fn from(value: PyAzureServicePrincipalRequest) -> Self {
        value.0
    }
}

/// The Azure service principal of a credential. The client secret is not returned.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AzureServicePrincipalResponse",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAzureServicePrincipalResponse(
    pub super::credentials::v1::AzureServicePrincipalResponse,
);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAzureServicePrincipalResponse {
    /// ID of the Microsoft Entra ID tenant (directory) of the application.
    #[getter]
    fn directory_id(&self) -> ::std::string::String {
        self.0.directory_id.clone()
    }

    /// Application (client) ID of the service principal.
    #[getter]
    fn application_id(&self) -> ::std::string::String {
        self.0.application_id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("directory_id", false), ("application_id", false)],
        )
    }
}

impl ::core::convert::From<super::credentials::v1::AzureServicePrincipalResponse>
    for PyAzureServicePrincipalResponse
{
    fn from(value: super::credentials::v1::AzureServicePrincipalResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAzureServicePrincipalResponse>
    for super::credentials::v1::AzureServicePrincipalResponse
{
    fn from(value: PyAzureServicePrincipalResponse) -> Self {
        value.0
    }
}

/// A shared access signature (SAS) for Azure storage, signed with a user delegation key.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "AzureUserDelegationSAS",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAzureUserDelegationSAS(pub super::credentials::v1::AzureUserDelegationSAS);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyAzureUserDelegationSAS {
    /// The SAS token.
    #[getter]
    fn sas_token(&self) -> ::std::string::String {
        self.0.sas_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("sas_token", true)])
    }
}

impl ::core::convert::From<super::credentials::v1::AzureUserDelegationSAS>
    for PyAzureUserDelegationSAS
{
    fn from(value: super::credentials::v1::AzureUserDelegationSAS) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyAzureUserDelegationSAS>
    for super::credentials::v1::AzureUserDelegationSAS
{
    fn from(value: PyAzureUserDelegationSAS) -> Self {
        value.0
    }
}

/// A credential that gives access to storage or to external services.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "CredentialInfo",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyCredentialInfo(pub super::credentials::v1::CredentialInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyCredentialInfo {
    /// Name of the credential.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// The AWS IAM role of the credential.
    #[getter]
    fn aws_iam_role(&self) -> ::core::option::Option<PyAwsIamRoleResponse> {
        self.0
            .aws_iam_role
            .clone()
            .into_option()
            .map(PyAwsIamRoleResponse::from)
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Username of the owner of the credential.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Full name of the credential: the same as `name`.
    #[getter]
    fn full_name(&self) -> ::std::string::String {
        self.0.full_name.clone()
    }

    /// Unique identifier of the credential.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Time at which the credential was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the credential.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the credential was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the credential.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// What the credential is used for. Absent when the credential was created without one.
    #[getter]
    fn purpose(&self) -> ::core::option::Option<PyCredentialPurpose> {
        self.0
            .purpose
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| {
                <super::credentials::v1::CredentialPurpose as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PyCredentialPurpose::from)
    }

    /// The Azure service principal of the credential.
    #[getter]
    fn azure_service_principal(&self) -> ::core::option::Option<PyAzureServicePrincipalResponse> {
        self.0
            .azure_service_principal
            .clone()
            .into_option()
            .map(PyAzureServicePrincipalResponse::from)
    }

    /// The Azure managed identity of the credential.
    #[getter]
    fn azure_managed_identity(&self) -> ::core::option::Option<PyAzureManagedIdentity> {
        self.0
            .azure_managed_identity
            .clone()
            .into_option()
            .map(PyAzureManagedIdentity::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("aws_iam_role", false),
                ("comment", false),
                ("owner", false),
                ("full_name", false),
                ("id", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("purpose", false),
                ("azure_service_principal", false),
                ("azure_managed_identity", false),
            ],
        )
    }
}

impl ::core::convert::From<super::credentials::v1::CredentialInfo> for PyCredentialInfo {
    fn from(value: super::credentials::v1::CredentialInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyCredentialInfo> for super::credentials::v1::CredentialInfo {
    fn from(value: PyCredentialInfo) -> Self {
        value.0
    }
}

/// An OAuth token for Google Cloud Storage.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "GcpOauthToken",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyGcpOauthToken(pub super::credentials::v1::GcpOauthToken);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyGcpOauthToken {
    /// The OAuth token.
    #[getter]
    fn oauth_token(&self) -> ::std::string::String {
        self.0.oauth_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("oauth_token", true)])
    }
}

impl ::core::convert::From<super::credentials::v1::GcpOauthToken> for PyGcpOauthToken {
    fn from(value: super::credentials::v1::GcpOauthToken) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyGcpOauthToken> for super::credentials::v1::GcpOauthToken {
    fn from(value: PyGcpOauthToken) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListCredentialsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListCredentialsResponse(pub super::credentials::v1::ListCredentialsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListCredentialsResponse {
    /// The credentials.
    #[getter]
    fn credentials(&self) -> ::std::vec::Vec<PyCredentialInfo> {
        self.0
            .credentials
            .iter()
            .cloned()
            .map(PyCredentialInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("credentials", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::credentials::v1::ListCredentialsResponse>
    for PyListCredentialsResponse
{
    fn from(value: super::credentials::v1::ListCredentialsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListCredentialsResponse>
    for super::credentials::v1::ListCredentialsResponse
{
    fn from(value: PyListCredentialsResponse) -> Self {
        value.0
    }
}

/// Temporary credentials for the cloud of a storage location.
/// Empty for a local `file:` location.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "TemporaryCredentials",
    eq,
    frozen,
    module = "unitycatalog._core.credentials",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTemporaryCredentials(pub super::credentials::v1::TemporaryCredentials);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyTemporaryCredentials {
    /// Temporary credentials for AWS S3.
    #[getter]
    fn aws_temp_credentials(&self) -> ::core::option::Option<PyAwsCredentials> {
        self.0
            .aws_temp_credentials
            .clone()
            .into_option()
            .map(PyAwsCredentials::from)
    }

    /// A SAS token for Azure storage.
    #[getter]
    fn azure_user_delegation_sas(&self) -> ::core::option::Option<PyAzureUserDelegationSAS> {
        self.0
            .azure_user_delegation_sas
            .clone()
            .into_option()
            .map(PyAzureUserDelegationSAS::from)
    }

    /// An OAuth token for Google Cloud Storage.
    #[getter]
    fn gcp_oauth_token(&self) -> ::core::option::Option<PyGcpOauthToken> {
        self.0
            .gcp_oauth_token
            .clone()
            .into_option()
            .map(PyGcpOauthToken::from)
    }

    /// Time at which the credentials expire, in epoch milliseconds. Absent when the credentials do
    /// not expire.
    #[getter]
    fn expiration_time(&self) -> ::core::option::Option<i64> {
        self.0.expiration_time
    }

    /// The storage path that the credentials give access to.
    #[getter]
    fn url(&self) -> ::std::string::String {
        self.0.url.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("aws_temp_credentials", false),
                ("azure_user_delegation_sas", false),
                ("gcp_oauth_token", false),
                ("expiration_time", false),
                ("url", false),
            ],
        )
    }
}

impl ::core::convert::From<super::credentials::v1::TemporaryCredentials>
    for PyTemporaryCredentials
{
    fn from(value: super::credentials::v1::TemporaryCredentials) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyTemporaryCredentials>
    for super::credentials::v1::TemporaryCredentials
{
    fn from(value: PyTemporaryCredentials) -> Self {
        value.0
    }
}

/// The columns of a Delta metadata change.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ColumnInfos",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyColumnInfos(pub super::delta::v1::ColumnInfos);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyColumnInfos {
    #[new]
    #[pyo3(signature = (*, columns = None))]
    fn new(columns: ::core::option::Option<::std::vec::Vec<PyColumnInfo>>) -> Self {
        let mut inner = <super::delta::v1::ColumnInfos as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = columns {
            inner.columns = value.into_iter().map(::core::convert::Into::into).collect();
        }
        Self(inner)
    }

    /// The columns of the table.
    #[getter]
    fn columns(&self) -> ::std::vec::Vec<PyColumnInfo> {
        self.0
            .columns
            .iter()
            .cloned()
            .map(PyColumnInfo::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("columns", false)])
    }
}

impl ::core::convert::From<super::delta::v1::ColumnInfos> for PyColumnInfos {
    fn from(value: super::delta::v1::ColumnInfos) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyColumnInfos> for super::delta::v1::ColumnInfos {
    fn from(value: PyColumnInfos) -> Self {
        value.0
    }
}

/// A commit of a catalog-managed table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaApiCommit",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaApiCommit(pub super::delta::v1::DeltaApiCommit);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaApiCommit {
    #[new]
    #[pyo3(
        signature = (
            version,
            timestamp,
            file_name,
            file_size,
            file_modification_timestamp
        )
    )]
    fn new(
        version: i64,
        timestamp: i64,
        file_name: ::std::string::String,
        file_size: i64,
        file_modification_timestamp: i64,
    ) -> Self {
        let mut inner = <super::delta::v1::DeltaApiCommit as ::core::default::Default>::default();
        {
            let value = version;
            inner.version = value;
        }
        {
            let value = timestamp;
            inner.timestamp = value;
        }
        {
            let value = file_name;
            inner.file_name = value;
        }
        {
            let value = file_size;
            inner.file_size = value;
        }
        {
            let value = file_modification_timestamp;
            inner.file_modification_timestamp = value;
        }
        Self(inner)
    }

    /// Version of the commit.
    #[getter]
    fn version(&self) -> i64 {
        self.0.version
    }

    /// The in-commit timestamp, in epoch milliseconds.
    #[getter]
    fn timestamp(&self) -> i64 {
        self.0.timestamp
    }

    /// Name of the staged commit file under `_delta_log/_staged_commits`, without a path.
    #[getter]
    fn file_name(&self) -> ::std::string::String {
        self.0.file_name.clone()
    }

    /// Size of the commit file, in bytes.
    #[getter]
    fn file_size(&self) -> i64 {
        self.0.file_size
    }

    /// Time at which the commit file was last changed, in epoch milliseconds.
    #[getter]
    fn file_modification_timestamp(&self) -> i64 {
        self.0.file_modification_timestamp
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("version", false),
                ("timestamp", false),
                ("file_name", false),
                ("file_size", false),
                ("file_modification_timestamp", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaApiCommit> for PyDeltaApiCommit {
    fn from(value: super::delta::v1::DeltaApiCommit) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaApiCommit> for super::delta::v1::DeltaApiCommit {
    fn from(value: PyDeltaApiCommit) -> Self {
        value.0
    }
}

/// The configuration of the Delta API for a catalog.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaCatalogConfig",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaCatalogConfig(pub super::delta::v1::DeltaCatalogConfig);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaCatalogConfig {
    /// The endpoints that the server supports, such as
    /// `GET /v1/catalogs/main/schemas/{schema}/tables/{table}`.
    #[getter]
    fn endpoints(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.endpoints.clone()
    }

    /// The protocol version of the endpoints, such as `1.0`.
    #[getter]
    fn protocol_version(&self) -> ::std::string::String {
        self.0.protocol_version.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("endpoints", false), ("protocol_version", false)],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaCatalogConfig> for PyDeltaCatalogConfig {
    fn from(value: super::delta::v1::DeltaCatalogConfig) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaCatalogConfig> for super::delta::v1::DeltaCatalogConfig {
    fn from(value: PyDeltaCatalogConfig) -> Self {
        value.0
    }
}

/// The domain metadata of a clustered table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaClusteringDomainMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaClusteringDomainMetadata(pub super::delta::v1::DeltaClusteringDomainMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaClusteringDomainMetadata {
    #[new]
    #[pyo3(signature = (*, clustering_columns = None))]
    fn new(
        clustering_columns: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
    ) -> ::pyo3::PyResult<Self> {
        let mut inner =
            <super::delta::v1::DeltaClusteringDomainMetadata as ::core::default::Default>::default(
            );
        if let ::core::option::Option::Some(value) = clustering_columns {
            inner.clustering_columns = {
                let items: ::std::vec::Vec<::pyo3::Bound<'_, ::pyo3::PyAny>> =
                    ::pyo3::types::PyAnyMethods::extract(&value)?;
                items
                    .iter()
                    .map(json_list_from_py)
                    .collect::<::pyo3::PyResult<_>>()?
            };
        }
        ::core::result::Result::Ok(Self(inner))
    }

    /// The clustering columns. Each one is a list of strings: the path from the root of the schema to
    /// the column, with physical names when column mapping is on. An empty list is sent too: the
    /// table is clustered by no columns.
    #[gen_stub(
        override_return_type(type_repr = "list[list[typing.Any]]", imports = ("typing"))
    )]
    #[getter]
    fn clustering_columns(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
        let items = self
            .0
            .clustering_columns
            .iter()
            .map(|value| json_list_to_py(py, value))
            .collect::<::pyo3::PyResult<::std::vec::Vec<_>>>()?;
        ::core::result::Result::Ok(::pyo3::types::PyList::new(py, items)?.into_any().unbind())
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("clustering_columns", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaClusteringDomainMetadata>
    for PyDeltaClusteringDomainMetadata
{
    fn from(value: super::delta::v1::DeltaClusteringDomainMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaClusteringDomainMetadata>
    for super::delta::v1::DeltaClusteringDomainMetadata
{
    fn from(value: PyDeltaClusteringDomainMetadata) -> Self {
        value.0
    }
}

/// One Delta commit file.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaCommitInfo",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaCommitInfo(pub super::delta::v1::DeltaCommitInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaCommitInfo {
    #[new]
    #[pyo3(
        signature = (
            version,
            timestamp,
            file_name,
            file_size,
            file_modification_timestamp
        )
    )]
    fn new(
        version: i64,
        timestamp: i64,
        file_name: ::std::string::String,
        file_size: i64,
        file_modification_timestamp: i64,
    ) -> Self {
        let mut inner = <super::delta::v1::DeltaCommitInfo as ::core::default::Default>::default();
        {
            let value = version;
            inner.version = value;
        }
        {
            let value = timestamp;
            inner.timestamp = value;
        }
        {
            let value = file_name;
            inner.file_name = value;
        }
        {
            let value = file_size;
            inner.file_size = value;
        }
        {
            let value = file_modification_timestamp;
            inner.file_modification_timestamp = value;
        }
        Self(inner)
    }

    /// Version of the commit. It must be positive.
    #[getter]
    fn version(&self) -> i64 {
        self.0.version
    }

    /// The in-commit timestamp that the client wrote, in epoch milliseconds. It must be positive.
    #[getter]
    fn timestamp(&self) -> i64 {
        self.0.timestamp
    }

    /// Name of the commit file under `_delta_log/_staged_commits`, without a path.
    #[getter]
    fn file_name(&self) -> ::std::string::String {
        self.0.file_name.clone()
    }

    /// Size of the commit file, in bytes. It must be positive.
    #[getter]
    fn file_size(&self) -> i64 {
        self.0.file_size
    }

    /// Time at which the commit file was last changed, in epoch milliseconds. It must be positive.
    #[getter]
    fn file_modification_timestamp(&self) -> i64 {
        self.0.file_modification_timestamp
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("version", false),
                ("timestamp", false),
                ("file_name", false),
                ("file_size", false),
                ("file_modification_timestamp", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaCommitInfo> for PyDeltaCommitInfo {
    fn from(value: super::delta::v1::DeltaCommitInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaCommitInfo> for super::delta::v1::DeltaCommitInfo {
    fn from(value: PyDeltaCommitInfo) -> Self {
        value.0
    }
}

/// The properties of a Delta metadata change.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaCommitMetadataProperties",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaCommitMetadataProperties(pub super::delta::v1::DeltaCommitMetadataProperties);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaCommitMetadataProperties {
    #[new]
    #[pyo3(signature = (*, properties = None))]
    fn new(
        properties: ::core::option::Option<
            ::std::collections::HashMap<::std::string::String, ::std::string::String>,
        >,
    ) -> Self {
        let mut inner =
            <super::delta::v1::DeltaCommitMetadataProperties as ::core::default::Default>::default(
            );
        if let ::core::option::Option::Some(value) = properties {
            inner.properties = value;
        }
        Self(inner)
    }

    /// The properties of the table. They replace the properties of the table.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("properties", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaCommitMetadataProperties>
    for PyDeltaCommitMetadataProperties
{
    fn from(value: super::delta::v1::DeltaCommitMetadataProperties) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaCommitMetadataProperties>
    for super::delta::v1::DeltaCommitMetadataProperties
{
    fn from(value: PyDeltaCommitMetadataProperties) -> Self {
        value.0
    }
}

/// The metrics of a commit.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaCommitReport",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaCommitReport(pub super::delta::v1::DeltaCommitReport);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaCommitReport {
    #[new]
    #[pyo3(
        signature = (
            *,
            num_files_added = None,
            num_bytes_added = None,
            num_files_removed = None,
            num_bytes_removed = None,
            num_rows_inserted = None,
            num_rows_removed = None,
            num_rows_updated = None,
            file_size_histogram = None
        )
    )]
    fn new(
        num_files_added: ::core::option::Option<i64>,
        num_bytes_added: ::core::option::Option<i64>,
        num_files_removed: ::core::option::Option<i64>,
        num_bytes_removed: ::core::option::Option<i64>,
        num_rows_inserted: ::core::option::Option<i64>,
        num_rows_removed: ::core::option::Option<i64>,
        num_rows_updated: ::core::option::Option<i64>,
        file_size_histogram: ::core::option::Option<PyDeltaFileSizeHistogram>,
    ) -> Self {
        let mut inner =
            <super::delta::v1::DeltaCommitReport as ::core::default::Default>::default();
        {
            let value = num_files_added;
            inner.num_files_added = value;
        }
        {
            let value = num_bytes_added;
            inner.num_bytes_added = value;
        }
        {
            let value = num_files_removed;
            inner.num_files_removed = value;
        }
        {
            let value = num_bytes_removed;
            inner.num_bytes_removed = value;
        }
        {
            let value = num_rows_inserted;
            inner.num_rows_inserted = value;
        }
        {
            let value = num_rows_removed;
            inner.num_rows_removed = value;
        }
        {
            let value = num_rows_updated;
            inner.num_rows_updated = value;
        }
        {
            let value = file_size_histogram;
            inner.file_size_histogram = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        Self(inner)
    }

    /// Number of files that the commit added.
    #[getter]
    fn num_files_added(&self) -> ::core::option::Option<i64> {
        self.0.num_files_added
    }

    /// Number of bytes that the commit added.
    #[getter]
    fn num_bytes_added(&self) -> ::core::option::Option<i64> {
        self.0.num_bytes_added
    }

    /// Number of files that the commit removed.
    #[getter]
    fn num_files_removed(&self) -> ::core::option::Option<i64> {
        self.0.num_files_removed
    }

    /// Number of bytes that the commit removed.
    #[getter]
    fn num_bytes_removed(&self) -> ::core::option::Option<i64> {
        self.0.num_bytes_removed
    }

    /// Number of rows that the commit inserted.
    #[getter]
    fn num_rows_inserted(&self) -> ::core::option::Option<i64> {
        self.0.num_rows_inserted
    }

    /// Number of rows that the commit removed.
    #[getter]
    fn num_rows_removed(&self) -> ::core::option::Option<i64> {
        self.0.num_rows_removed
    }

    /// Number of rows that the commit updated.
    #[getter]
    fn num_rows_updated(&self) -> ::core::option::Option<i64> {
        self.0.num_rows_updated
    }

    /// File counts and total bytes by file size.
    #[getter]
    fn file_size_histogram(&self) -> ::core::option::Option<PyDeltaFileSizeHistogram> {
        self.0
            .file_size_histogram
            .clone()
            .into_option()
            .map(PyDeltaFileSizeHistogram::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("num_files_added", false),
                ("num_bytes_added", false),
                ("num_files_removed", false),
                ("num_bytes_removed", false),
                ("num_rows_inserted", false),
                ("num_rows_removed", false),
                ("num_rows_updated", false),
                ("file_size_histogram", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaCommitReport> for PyDeltaCommitReport {
    fn from(value: super::delta::v1::DeltaCommitReport) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaCommitReport> for super::delta::v1::DeltaCommitReport {
    fn from(value: PyDeltaCommitReport) -> Self {
        value.0
    }
}

/// Temporary credentials for the data of a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaCredentialsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaCredentialsResponse(pub super::delta::v1::DeltaCredentialsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaCredentialsResponse {
    /// The temporary cloud storage credentials.
    #[getter]
    fn storage_credentials(&self) -> ::std::vec::Vec<PyDeltaStorageCredential> {
        self.0
            .storage_credentials
            .iter()
            .cloned()
            .map(PyDeltaStorageCredential::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("storage_credentials", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaCredentialsResponse>
    for PyDeltaCredentialsResponse
{
    fn from(value: super::delta::v1::DeltaCredentialsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaCredentialsResponse>
    for super::delta::v1::DeltaCredentialsResponse
{
    fn from(value: PyDeltaCredentialsResponse) -> Self {
        value.0
    }
}

/// Domain metadata entries, by domain name.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaDomainMetadataUpdates",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaDomainMetadataUpdates(pub super::delta::v1::DeltaDomainMetadataUpdates);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaDomainMetadataUpdates {
    #[new]
    #[pyo3(signature = (*, delta_clustering = None, delta_row_tracking = None))]
    fn new(
        delta_clustering: ::core::option::Option<PyDeltaClusteringDomainMetadata>,
        delta_row_tracking: ::core::option::Option<PyDeltaRowTrackingDomainMetadata>,
    ) -> Self {
        let mut inner =
            <super::delta::v1::DeltaDomainMetadataUpdates as ::core::default::Default>::default();
        {
            let value = delta_clustering;
            inner.delta_clustering = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = delta_row_tracking;
            inner.delta_row_tracking = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        Self(inner)
    }

    /// The `delta.clustering` domain.
    #[getter]
    fn delta_clustering(&self) -> ::core::option::Option<PyDeltaClusteringDomainMetadata> {
        self.0
            .delta_clustering
            .clone()
            .into_option()
            .map(PyDeltaClusteringDomainMetadata::from)
    }

    /// The `delta.rowTracking` domain.
    #[getter]
    fn delta_row_tracking(&self) -> ::core::option::Option<PyDeltaRowTrackingDomainMetadata> {
        self.0
            .delta_row_tracking
            .clone()
            .into_option()
            .map(PyDeltaRowTrackingDomainMetadata::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("delta_clustering", false), ("delta_row_tracking", false)],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaDomainMetadataUpdates>
    for PyDeltaDomainMetadataUpdates
{
    fn from(value: super::delta::v1::DeltaDomainMetadataUpdates) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaDomainMetadataUpdates>
    for super::delta::v1::DeltaDomainMetadataUpdates
{
    fn from(value: PyDeltaDomainMetadataUpdates) -> Self {
        value.0
    }
}

/// A file size histogram of a commit.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaFileSizeHistogram",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaFileSizeHistogram(pub super::delta::v1::DeltaFileSizeHistogram);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaFileSizeHistogram {
    #[new]
    #[pyo3(
        signature = (
            *,
            sorted_bin_boundaries = None,
            file_counts = None,
            total_bytes = None,
            commit_version = None
        )
    )]
    fn new(
        sorted_bin_boundaries: ::core::option::Option<::std::vec::Vec<i64>>,
        file_counts: ::core::option::Option<::std::vec::Vec<i64>>,
        total_bytes: ::core::option::Option<::std::vec::Vec<i64>>,
        commit_version: ::core::option::Option<i64>,
    ) -> Self {
        let mut inner =
            <super::delta::v1::DeltaFileSizeHistogram as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = sorted_bin_boundaries {
            inner.sorted_bin_boundaries = value;
        }
        if let ::core::option::Option::Some(value) = file_counts {
            inner.file_counts = value;
        }
        if let ::core::option::Option::Some(value) = total_bytes {
            inner.total_bytes = value;
        }
        {
            let value = commit_version;
            inner.commit_version = value;
        }
        Self(inner)
    }

    /// Start of each bin (inclusive), in bytes, sorted. The first one is 0.
    #[getter]
    fn sorted_bin_boundaries(&self) -> ::std::vec::Vec<i64> {
        self.0.sorted_bin_boundaries.clone()
    }

    /// Number of files in each bin.
    #[getter]
    fn file_counts(&self) -> ::std::vec::Vec<i64> {
        self.0.file_counts.clone()
    }

    /// Total bytes in each bin.
    #[getter]
    fn total_bytes(&self) -> ::std::vec::Vec<i64> {
        self.0.total_bytes.clone()
    }

    /// Version of the commit that the histogram is for.
    #[getter]
    fn commit_version(&self) -> ::core::option::Option<i64> {
        self.0.commit_version
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("sorted_bin_boundaries", false),
                ("file_counts", false),
                ("total_bytes", false),
                ("commit_version", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaFileSizeHistogram> for PyDeltaFileSizeHistogram {
    fn from(value: super::delta::v1::DeltaFileSizeHistogram) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaFileSizeHistogram> for super::delta::v1::DeltaFileSizeHistogram {
    fn from(value: PyDeltaFileSizeHistogram) -> Self {
        value.0
    }
}

/// The commits that the server tracks for a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaGetCommitsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaGetCommitsResponse(pub super::delta::v1::DeltaGetCommitsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaGetCommitsResponse {
    /// The commits that are not backfilled yet, newest first.
    #[getter]
    fn commits(&self) -> ::std::vec::Vec<PyDeltaCommitInfo> {
        self.0
            .commits
            .iter()
            .cloned()
            .map(PyDeltaCommitInfo::from)
            .collect()
    }

    /// The latest version that the server tracks; 0 before the first commit.
    #[getter]
    fn latest_table_version(&self) -> i64 {
        self.0.latest_table_version
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("commits", false), ("latest_table_version", false)],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaGetCommitsResponse>
    for PyDeltaGetCommitsResponse
{
    fn from(value: super::delta::v1::DeltaGetCommitsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaGetCommitsResponse>
    for super::delta::v1::DeltaGetCommitsResponse
{
    fn from(value: PyDeltaGetCommitsResponse) -> Self {
        value.0
    }
}

/// A Delta table: its metadata and, for a MANAGED table, its commits that are not backfilled yet.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaLoadTableResponse",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaLoadTableResponse(pub super::delta::v1::DeltaLoadTableResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaLoadTableResponse {
    /// The metadata of the table.
    #[getter]
    fn metadata(&self) -> ::core::option::Option<PyDeltaTableMetadata> {
        self.0
            .metadata
            .clone()
            .into_option()
            .map(PyDeltaTableMetadata::from)
    }

    /// The commits that are not backfilled yet, newest first. Only for MANAGED tables.
    #[getter]
    fn commits(&self) -> ::std::vec::Vec<PyDeltaApiCommit> {
        self.0
            .commits
            .iter()
            .cloned()
            .map(PyDeltaApiCommit::from)
            .collect()
    }

    /// UniForm conversion metadata. Only for UniForm tables.
    #[getter]
    fn uniform(&self) -> ::core::option::Option<PyDeltaUniformMetadata> {
        self.0
            .uniform
            .clone()
            .into_option()
            .map(PyDeltaUniformMetadata::from)
    }

    /// The latest version that the server tracks, data-only commits included. Only for MANAGED
    /// tables.
    #[getter]
    fn latest_table_version(&self) -> ::core::option::Option<i64> {
        self.0.latest_table_version
    }

    /// Maintenance operations that the catalog allows in addition to the Delta protocol defaults.
    #[getter]
    fn allowed_maintenance_operations(&self) -> ::std::vec::Vec<PyDeltaMaintenanceOperation> {
        self.0
            .allowed_maintenance_operations
            .iter()
            .filter_map(|e| e.as_known())
            .filter(|e| {
                <super::delta::v1::DeltaMaintenanceOperation as ::buffa::Enumeration>::to_i32(e)
                    != 0
            })
            .map(PyDeltaMaintenanceOperation::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("metadata", false),
                ("commits", false),
                ("uniform", false),
                ("latest_table_version", false),
                ("allowed_maintenance_operations", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaLoadTableResponse> for PyDeltaLoadTableResponse {
    fn from(value: super::delta::v1::DeltaLoadTableResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaLoadTableResponse> for super::delta::v1::DeltaLoadTableResponse {
    fn from(value: PyDeltaLoadTableResponse) -> Self {
        value.0
    }
}

/// A Delta metadata change to apply to the table with the commit. At least one field must be
/// set.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaMetadata(pub super::delta::v1::DeltaMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaMetadata {
    #[new]
    #[pyo3(signature = (*, description = None, schema = None, properties = None))]
    fn new(
        description: ::core::option::Option<::std::string::String>,
        schema: ::core::option::Option<PyColumnInfos>,
        properties: ::core::option::Option<PyDeltaCommitMetadataProperties>,
    ) -> Self {
        let mut inner = <super::delta::v1::DeltaMetadata as ::core::default::Default>::default();
        {
            let value = description;
            inner.description = value;
        }
        {
            let value = schema;
            inner.schema = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = properties;
            inner.properties = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        Self(inner)
    }

    /// The comment of the table.
    #[getter]
    fn description(&self) -> ::core::option::Option<::std::string::String> {
        self.0.description.clone()
    }

    /// The columns of the table. They replace the columns of the table.
    #[getter]
    fn schema(&self) -> ::core::option::Option<PyColumnInfos> {
        self.0.schema.clone().into_option().map(PyColumnInfos::from)
    }

    /// The properties of the table.
    #[getter]
    fn properties(&self) -> ::core::option::Option<PyDeltaCommitMetadataProperties> {
        self.0
            .properties
            .clone()
            .into_option()
            .map(PyDeltaCommitMetadataProperties::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("description", false),
                ("schema", false),
                ("properties", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaMetadata> for PyDeltaMetadata {
    fn from(value: super::delta::v1::DeltaMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaMetadata> for super::delta::v1::DeltaMetadata {
    fn from(value: PyDeltaMetadata) -> Self {
        value.0
    }
}

/// The reported metrics.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaMetricsReport",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaMetricsReport(pub super::delta::v1::DeltaMetricsReport);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaMetricsReport {
    #[new]
    #[pyo3(signature = (*, commit_report = None))]
    fn new(commit_report: ::core::option::Option<PyDeltaCommitReport>) -> Self {
        let mut inner =
            <super::delta::v1::DeltaMetricsReport as ::core::default::Default>::default();
        {
            let value = commit_report;
            inner.commit_report = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        Self(inner)
    }

    /// The metrics of a commit.
    #[getter]
    fn commit_report(&self) -> ::core::option::Option<PyDeltaCommitReport> {
        self.0
            .commit_report
            .clone()
            .into_option()
            .map(PyDeltaCommitReport::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("commit_report", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaMetricsReport> for PyDeltaMetricsReport {
    fn from(value: super::delta::v1::DeltaMetricsReport) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaMetricsReport> for super::delta::v1::DeltaMetricsReport {
    fn from(value: PyDeltaMetricsReport) -> Self {
        value.0
    }
}

/// Delta protocol versions and features.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaProtocol",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaProtocol(pub super::delta::v1::DeltaProtocol);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaProtocol {
    #[new]
    #[pyo3(
        signature = (
            *,
            min_reader_version = None,
            min_writer_version = None,
            reader_features = None,
            writer_features = None
        )
    )]
    fn new(
        min_reader_version: ::core::option::Option<i32>,
        min_writer_version: ::core::option::Option<i32>,
        reader_features: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
        writer_features: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
    ) -> Self {
        let mut inner = <super::delta::v1::DeltaProtocol as ::core::default::Default>::default();
        {
            let value = min_reader_version;
            inner.min_reader_version = value;
        }
        {
            let value = min_writer_version;
            inner.min_writer_version = value;
        }
        if let ::core::option::Option::Some(value) = reader_features {
            inner.reader_features = value;
        }
        if let ::core::option::Option::Some(value) = writer_features {
            inner.writer_features = value;
        }
        Self(inner)
    }

    /// Minimum reader version.
    #[getter]
    fn min_reader_version(&self) -> ::core::option::Option<i32> {
        self.0.min_reader_version
    }

    /// Minimum writer version.
    #[getter]
    fn min_writer_version(&self) -> ::core::option::Option<i32> {
        self.0.min_writer_version
    }

    /// The reader features, such as `deletionVectors`.
    #[getter]
    fn reader_features(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.reader_features.clone()
    }

    /// The writer features, such as `catalogManaged`.
    #[getter]
    fn writer_features(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.writer_features.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("min_reader_version", false),
                ("min_writer_version", false),
                ("reader_features", false),
                ("writer_features", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaProtocol> for PyDeltaProtocol {
    fn from(value: super::delta::v1::DeltaProtocol) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaProtocol> for super::delta::v1::DeltaProtocol {
    fn from(value: PyDeltaProtocol) -> Self {
        value.0
    }
}

/// The domain metadata of a table with row tracking.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaRowTrackingDomainMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaRowTrackingDomainMetadata(pub super::delta::v1::DeltaRowTrackingDomainMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaRowTrackingDomainMetadata {
    #[new]
    #[pyo3(signature = (*, row_id_high_water_mark = None))]
    fn new(row_id_high_water_mark: ::core::option::Option<i64>) -> Self {
        let mut inner =
            <super::delta::v1::DeltaRowTrackingDomainMetadata as ::core::default::Default>::default(
            );
        {
            let value = row_id_high_water_mark;
            inner.row_id_high_water_mark = value;
        }
        Self(inner)
    }

    /// The highest row id given so far.
    #[getter]
    fn row_id_high_water_mark(&self) -> ::core::option::Option<i64> {
        self.0.row_id_high_water_mark
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("row_id_high_water_mark", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaRowTrackingDomainMetadata>
    for PyDeltaRowTrackingDomainMetadata
{
    fn from(value: super::delta::v1::DeltaRowTrackingDomainMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaRowTrackingDomainMetadata>
    for super::delta::v1::DeltaRowTrackingDomainMetadata
{
    fn from(value: PyDeltaRowTrackingDomainMetadata) -> Self {
        value.0
    }
}

/// A staging table: the table id and storage location that the server reserves for a new MANAGED
/// table, and what the first commit must contain.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaStagingTableResponse",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaStagingTableResponse(pub super::delta::v1::DeltaStagingTableResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaStagingTableResponse {
    /// Unique identifier of the table.
    #[getter]
    fn table_id(&self) -> ::std::string::String {
        self.0.table_id.clone()
    }

    /// Type of the table. Always `MANAGED`.
    #[getter]
    fn table_type(&self) -> ::core::option::Option<PyDeltaTableType> {
        self.0
            .table_type
            .as_known()
            .filter(|e| <super::delta::v1::DeltaTableType as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyDeltaTableType::from)
    }

    /// The storage location of the table.
    #[getter]
    fn location(&self) -> ::std::string::String {
        self.0.location.clone()
    }

    /// Temporary credentials to write the first commit.
    #[getter]
    fn storage_credentials(&self) -> ::std::vec::Vec<PyDeltaStorageCredential> {
        self.0
            .storage_credentials
            .iter()
            .cloned()
            .map(PyDeltaStorageCredential::from)
            .collect()
    }

    /// The protocol that the first commit must have at least.
    #[getter]
    fn required_protocol(&self) -> ::core::option::Option<PyDeltaProtocol> {
        self.0
            .required_protocol
            .clone()
            .into_option()
            .map(PyDeltaProtocol::from)
    }

    /// Features that the client should enable when it can.
    #[getter]
    fn suggested_protocol(&self) -> ::core::option::Option<PyDeltaSuggestedProtocol> {
        self.0
            .suggested_protocol
            .clone()
            .into_option()
            .map(PyDeltaSuggestedProtocol::from)
    }

    /// Table properties that the table must have. Each value is a string, or `null` when any valid
    /// value is allowed.
    #[gen_stub(
        override_return_type(type_repr = "dict[str, typing.Any]", imports = ("typing"))
    )]
    #[getter]
    fn required_properties(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
        let dict = ::pyo3::types::PyDict::new(py);
        for (key, value) in &self.0.required_properties {
            ::pyo3::types::PyDictMethods::set_item(&dict, key, json_value_to_py(py, value)?)?;
        }
        ::core::result::Result::Ok(dict.into_any().unbind())
    }

    /// Table properties that the table should have. Each value is a string, or `null` when the client
    /// makes the value.
    #[gen_stub(
        override_return_type(type_repr = "dict[str, typing.Any]", imports = ("typing"))
    )]
    #[getter]
    fn suggested_properties(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
        let dict = ::pyo3::types::PyDict::new(py);
        for (key, value) in &self.0.suggested_properties {
            ::pyo3::types::PyDictMethods::set_item(&dict, key, json_value_to_py(py, value)?)?;
        }
        ::core::result::Result::Ok(dict.into_any().unbind())
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("table_id", false),
                ("table_type", false),
                ("location", false),
                ("storage_credentials", false),
                ("required_protocol", false),
                ("suggested_protocol", false),
                ("required_properties", false),
                ("suggested_properties", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaStagingTableResponse>
    for PyDeltaStagingTableResponse
{
    fn from(value: super::delta::v1::DeltaStagingTableResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaStagingTableResponse>
    for super::delta::v1::DeltaStagingTableResponse
{
    fn from(value: PyDeltaStagingTableResponse) -> Self {
        value.0
    }
}

/// A temporary credential for a storage prefix. When more than one credential applies, use the
/// one with the longest prefix.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaStorageCredential",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaStorageCredential(pub super::delta::v1::DeltaStorageCredential);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaStorageCredential {
    /// The storage path prefix that the credential is for.
    #[getter]
    fn prefix(&self) -> ::std::string::String {
        self.0.prefix.clone()
    }

    /// The access level of the credential.
    #[getter]
    fn operation(&self) -> ::core::option::Option<PyDeltaCredentialOperation> {
        self.0
            .operation
            .as_known()
            .filter(|e| {
                <super::delta::v1::DeltaCredentialOperation as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PyDeltaCredentialOperation::from)
    }

    /// The cloud credential settings.
    #[getter]
    fn config(&self) -> ::core::option::Option<PyDeltaStorageCredentialConfig> {
        self.0
            .config
            .clone()
            .into_option()
            .map(PyDeltaStorageCredentialConfig::from)
    }

    /// Time at which the credential expires, in epoch milliseconds.
    #[getter]
    fn expiration_time_ms(&self) -> ::core::option::Option<i64> {
        self.0.expiration_time_ms
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("prefix", false),
                ("operation", false),
                ("config", false),
                ("expiration_time_ms", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaStorageCredential> for PyDeltaStorageCredential {
    fn from(value: super::delta::v1::DeltaStorageCredential) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaStorageCredential> for super::delta::v1::DeltaStorageCredential {
    fn from(value: PyDeltaStorageCredential) -> Self {
        value.0
    }
}

/// Cloud credential settings. Only the keys of one cloud provider are set.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaStorageCredentialConfig",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaStorageCredentialConfig(pub super::delta::v1::DeltaStorageCredentialConfig);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaStorageCredentialConfig {
    /// AWS access key id.
    #[getter]
    fn s3_access_key_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.s3_access_key_id.clone()
    }

    /// AWS secret access key.
    #[getter]
    fn s3_secret_access_key(&self) -> ::core::option::Option<::std::string::String> {
        self.0.s3_secret_access_key.clone()
    }

    /// AWS session token of the temporary credentials.
    #[getter]
    fn s3_session_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.s3_session_token.clone()
    }

    /// Azure SAS (shared access signature) token.
    #[getter]
    fn azure_sas_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.azure_sas_token.clone()
    }

    /// GCP OAuth token.
    #[getter]
    fn gcs_oauth_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.gcs_oauth_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("s3_access_key_id", false),
                ("s3_secret_access_key", false),
                ("s3_session_token", false),
                ("azure_sas_token", false),
                ("gcs_oauth_token", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaStorageCredentialConfig>
    for PyDeltaStorageCredentialConfig
{
    fn from(value: super::delta::v1::DeltaStorageCredentialConfig) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaStorageCredentialConfig>
    for super::delta::v1::DeltaStorageCredentialConfig
{
    fn from(value: PyDeltaStorageCredentialConfig) -> Self {
        value.0
    }
}

/// A field of a Delta struct type, as in the Delta protocol schema format.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaStructField",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaStructField(pub super::delta::v1::DeltaStructField);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaStructField {
    #[new]
    #[pyo3(signature = (name, data_type, nullable, metadata))]
    fn new(
        name: ::std::string::String,
        data_type: ::pyo3::Bound<'_, ::pyo3::PyAny>,
        nullable: bool,
        metadata: ::pyo3::Bound<'_, ::pyo3::PyAny>,
    ) -> ::pyo3::PyResult<Self> {
        let mut inner = <super::delta::v1::DeltaStructField as ::core::default::Default>::default();
        {
            let value = name;
            inner.name = value;
        }
        {
            let value = data_type;
            inner.data_type = ::buffa::MessageField::some(json_value_from_py(&value)?);
        }
        {
            let value = nullable;
            inner.nullable = value;
        }
        {
            let value = metadata;
            inner.metadata = ::buffa::MessageField::some(json_struct_from_py(&value)?);
        }
        ::core::result::Result::Ok(Self(inner))
    }

    /// Name of the field. It is unique in its struct, case-insensitively.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Data type of the field: a string for a primitive type (`long`, `decimal(10,2)`), or an object
    /// with a `type` of `array` (`element-type`, `contains-null`), `map` (`key-type`, `value-type`,
    /// `value-contains-null`), `struct` (`fields`) or `decimal` (`precision`, `scale`).
    #[gen_stub(override_return_type(type_repr = "typing.Any", imports = ("typing")))]
    #[getter]
    fn data_type(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .data_type
            .as_option()
            .map(|value| json_value_to_py(py, value))
            .transpose()
    }

    /// Whether the field can hold null values.
    #[getter]
    fn nullable(&self) -> bool {
        self.0.nullable
    }

    /// Metadata of the field, such as `comment` or `delta.columnMapping.id`. An empty object when
    /// there is none.
    #[gen_stub(
        override_return_type(
            type_repr = "typing.Optional[dict[str, typing.Any]]",
            imports = ("typing")
        )
    )]
    #[getter]
    fn metadata(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .metadata
            .as_option()
            .map(|value| json_struct_to_py(py, value))
            .transpose()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("data_type", false),
                ("nullable", false),
                ("metadata", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaStructField> for PyDeltaStructField {
    fn from(value: super::delta::v1::DeltaStructField) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaStructField> for super::delta::v1::DeltaStructField {
    fn from(value: PyDeltaStructField) -> Self {
        value.0
    }
}

/// A Delta struct type, such as the columns of a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaStructType",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaStructType(pub super::delta::v1::DeltaStructType);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaStructType {
    #[new]
    #[pyo3(signature = (fields, *, type_name = None))]
    fn new(
        fields: ::std::vec::Vec<PyDeltaStructField>,
        type_name: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner = <super::delta::v1::DeltaStructType as ::core::default::Default>::default();
        {
            let value = fields;
            inner.fields = value.into_iter().map(::core::convert::Into::into).collect();
        }
        if let ::core::option::Option::Some(value) = type_name {
            inner.type_name = value;
        }
        Self(inner)
    }

    /// The type name. Always `struct`.
    #[getter]
    fn type_name(&self) -> ::std::string::String {
        self.0.type_name.clone()
    }

    /// The fields of the struct. At least one.
    #[getter]
    fn fields(&self) -> ::std::vec::Vec<PyDeltaStructField> {
        self.0
            .fields
            .iter()
            .cloned()
            .map(PyDeltaStructField::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("type_name", false), ("fields", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaStructType> for PyDeltaStructType {
    fn from(value: super::delta::v1::DeltaStructType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaStructType> for super::delta::v1::DeltaStructType {
    fn from(value: PyDeltaStructType) -> Self {
        value.0
    }
}

/// Features that a client should enable when it can.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaSuggestedProtocol",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaSuggestedProtocol(pub super::delta::v1::DeltaSuggestedProtocol);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaSuggestedProtocol {
    /// The suggested reader features.
    #[getter]
    fn reader_features(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.reader_features.clone()
    }

    /// The suggested writer features.
    #[getter]
    fn writer_features(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.writer_features.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("reader_features", false), ("writer_features", false)],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaSuggestedProtocol> for PyDeltaSuggestedProtocol {
    fn from(value: super::delta::v1::DeltaSuggestedProtocol) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaSuggestedProtocol> for super::delta::v1::DeltaSuggestedProtocol {
    fn from(value: PyDeltaSuggestedProtocol) -> Self {
        value.0
    }
}

/// The metadata of a Delta table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaTableMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaTableMetadata(pub super::delta::v1::DeltaTableMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaTableMetadata {
    /// Entity tag of the metadata. It changes when the metadata changes.
    #[getter]
    fn etag(&self) -> ::std::string::String {
        self.0.etag.clone()
    }

    /// Type of the table.
    #[getter]
    fn table_type(&self) -> ::core::option::Option<PyDeltaTableType> {
        self.0
            .table_type
            .as_known()
            .filter(|e| <super::delta::v1::DeltaTableType as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyDeltaTableType::from)
    }

    /// Unique identifier of the table.
    #[getter]
    fn table_uuid(&self) -> ::std::string::String {
        self.0.table_uuid.clone()
    }

    /// Storage location of the table.
    #[getter]
    fn location(&self) -> ::std::string::String {
        self.0.location.clone()
    }

    /// Time at which the table was created, in epoch milliseconds.
    #[getter]
    fn created_time(&self) -> i64 {
        self.0.created_time
    }

    /// Time at which the table was last changed, in epoch milliseconds.
    #[getter]
    fn updated_time(&self) -> i64 {
        self.0.updated_time
    }

    /// The columns of the table.
    #[getter]
    fn columns(&self) -> ::core::option::Option<PyDeltaStructType> {
        self.0
            .columns
            .clone()
            .into_option()
            .map(PyDeltaStructType::from)
    }

    /// Names of the partition columns.
    #[getter]
    fn partition_columns(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.partition_columns.clone()
    }

    /// The table properties, with keys that the server derives (`delta.minReaderVersion`,
    /// `delta.feature.*`, `delta.lastUpdateVersion`, ...). Clients must not write those back.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    /// Version of the last commit that changed the metadata.
    #[getter]
    fn last_commit_version(&self) -> ::core::option::Option<i64> {
        self.0.last_commit_version
    }

    /// Timestamp of the last commit that changed the metadata, in epoch milliseconds.
    #[getter]
    fn last_commit_timestamp_ms(&self) -> ::core::option::Option<i64> {
        self.0.last_commit_timestamp_ms
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("etag", false),
                ("table_type", false),
                ("table_uuid", false),
                ("location", false),
                ("created_time", false),
                ("updated_time", false),
                ("columns", false),
                ("partition_columns", false),
                ("properties", false),
                ("last_commit_version", false),
                ("last_commit_timestamp_ms", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaTableMetadata> for PyDeltaTableMetadata {
    fn from(value: super::delta::v1::DeltaTableMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaTableMetadata> for super::delta::v1::DeltaTableMetadata {
    fn from(value: PyDeltaTableMetadata) -> Self {
        value.0
    }
}

/// A precondition of a table update. The `type` says which field is set:
/// - `assert-table-uuid`: `uuid`, the expected table id.
/// - `assert-etag`: `etag`, the expected entity tag of the metadata.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaTableRequirement",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaTableRequirement(pub super::delta::v1::DeltaTableRequirement);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaTableRequirement {
    #[new]
    #[pyo3(signature = (requirement_type, *, uuid = None, etag = None))]
    fn new(
        requirement_type: ::std::string::String,
        uuid: ::core::option::Option<::std::string::String>,
        etag: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner =
            <super::delta::v1::DeltaTableRequirement as ::core::default::Default>::default();
        {
            let value = requirement_type;
            inner.requirement_type = value;
        }
        {
            let value = uuid;
            inner.uuid = value;
        }
        {
            let value = etag;
            inner.etag = value;
        }
        Self(inner)
    }

    /// Kind of the requirement, the JSON `type`: `assert-table-uuid` or `assert-etag`.
    #[getter]
    fn requirement_type(&self) -> ::std::string::String {
        self.0.requirement_type.clone()
    }

    /// The expected unique identifier of the table. For `assert-table-uuid`.
    #[getter]
    fn uuid(&self) -> ::core::option::Option<::std::string::String> {
        self.0.uuid.clone()
    }

    /// The expected entity tag of the table metadata. For `assert-etag`.
    #[getter]
    fn etag(&self) -> ::core::option::Option<::std::string::String> {
        self.0.etag.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("requirement_type", false),
                ("uuid", false),
                ("etag", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaTableRequirement> for PyDeltaTableRequirement {
    fn from(value: super::delta::v1::DeltaTableRequirement) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaTableRequirement> for super::delta::v1::DeltaTableRequirement {
    fn from(value: PyDeltaTableRequirement) -> Self {
        value.0
    }
}

/// One update action. The `action` says which fields are set:
/// - `set-properties`: `updates`, property names to string values.
/// - `remove-properties`: `removals`.
/// - `set-columns`: `columns`.
/// - `set-table-comment`: `comment`.
/// - `add-commit`: `commit` and `uniform`.
/// - `set-latest-backfilled-version`: `latest_published_version`.
/// - `set-protocol`: `protocol`.
/// - `set-domain-metadata`: `updates`, domain names to their metadata:
/// `delta.clustering` to `{"clusteringColumns": [["a"], ["b", "c"]]}`, `delta.rowTracking` to
/// `{"rowIdHighWaterMark": 9}`.
/// - `remove-domain-metadata`: `domains`.
/// - `set-partition-columns`: `partition_columns`.
/// - `update-metadata-snapshot-version`: `last_commit_version` and `last_commit_timestamp_ms`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaTableUpdate",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaTableUpdate(pub super::delta::v1::DeltaTableUpdate);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaTableUpdate {
    #[new]
    #[pyo3(
        signature = (
            action,
            *,
            updates = None,
            removals = None,
            columns = None,
            comment = None,
            commit = None,
            uniform = None,
            latest_published_version = None,
            protocol = None,
            domains = None,
            partition_columns = None,
            last_commit_version = None,
            last_commit_timestamp_ms = None
        )
    )]
    fn new(
        action: ::std::string::String,
        updates: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
        removals: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
        columns: ::core::option::Option<PyDeltaStructType>,
        comment: ::core::option::Option<::std::string::String>,
        commit: ::core::option::Option<PyDeltaApiCommit>,
        uniform: ::core::option::Option<PyDeltaUniformMetadata>,
        latest_published_version: ::core::option::Option<i64>,
        protocol: ::core::option::Option<PyDeltaProtocol>,
        domains: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
        partition_columns: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
        last_commit_version: ::core::option::Option<i64>,
        last_commit_timestamp_ms: ::core::option::Option<i64>,
    ) -> ::pyo3::PyResult<Self> {
        let mut inner = <super::delta::v1::DeltaTableUpdate as ::core::default::Default>::default();
        {
            let value = action;
            inner.action = value;
        }
        if let ::core::option::Option::Some(value) = updates {
            inner.updates = {
                let items: ::std::collections::HashMap<
                    ::std::string::String,
                    ::pyo3::Bound<'_, ::pyo3::PyAny>,
                > = ::pyo3::types::PyAnyMethods::extract(&value)?;
                items
                    .iter()
                    .map(|(key, item)| {
                        ::core::result::Result::Ok((key.clone(), json_value_from_py(item)?))
                    })
                    .collect::<::pyo3::PyResult<_>>()?
            };
        }
        if let ::core::option::Option::Some(value) = removals {
            inner.removals = value;
        }
        {
            let value = columns;
            inner.columns = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = comment;
            inner.comment = value;
        }
        {
            let value = commit;
            inner.commit = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = uniform;
            inner.uniform = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = latest_published_version;
            inner.latest_published_version = value;
        }
        {
            let value = protocol;
            inner.protocol = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        if let ::core::option::Option::Some(value) = domains {
            inner.domains = value;
        }
        if let ::core::option::Option::Some(value) = partition_columns {
            inner.partition_columns = value;
        }
        {
            let value = last_commit_version;
            inner.last_commit_version = value;
        }
        {
            let value = last_commit_timestamp_ms;
            inner.last_commit_timestamp_ms = value;
        }
        ::core::result::Result::Ok(Self(inner))
    }

    /// Kind of the update: one of the actions above.
    #[getter]
    fn action(&self) -> ::std::string::String {
        self.0.action.clone()
    }

    /// For `set-properties`: property names to string values. For `set-domain-metadata`: domain
    /// names to their metadata objects.
    #[gen_stub(
        override_return_type(type_repr = "dict[str, typing.Any]", imports = ("typing"))
    )]
    #[getter]
    fn updates(&self, py: ::pyo3::Python<'_>) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
        let dict = ::pyo3::types::PyDict::new(py);
        for (key, value) in &self.0.updates {
            ::pyo3::types::PyDictMethods::set_item(&dict, key, json_value_to_py(py, value)?)?;
        }
        ::core::result::Result::Ok(dict.into_any().unbind())
    }

    /// For `remove-properties`: the property keys to remove.
    #[getter]
    fn removals(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.removals.clone()
    }

    /// For `set-columns`: the new columns of the table.
    #[getter]
    fn columns(&self) -> ::core::option::Option<PyDeltaStructType> {
        self.0
            .columns
            .clone()
            .into_option()
            .map(PyDeltaStructType::from)
    }

    /// For `set-table-comment`: the new comment of the table.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// For `add-commit`: the commit to add.
    #[getter]
    fn commit(&self) -> ::core::option::Option<PyDeltaApiCommit> {
        self.0
            .commit
            .clone()
            .into_option()
            .map(PyDeltaApiCommit::from)
    }

    /// For `add-commit`: UniForm metadata to commit with the commit.
    #[getter]
    fn uniform(&self) -> ::core::option::Option<PyDeltaUniformMetadata> {
        self.0
            .uniform
            .clone()
            .into_option()
            .map(PyDeltaUniformMetadata::from)
    }

    /// For `set-latest-backfilled-version`: the commits up to this version are backfilled.
    #[getter]
    fn latest_published_version(&self) -> ::core::option::Option<i64> {
        self.0.latest_published_version
    }

    /// For `set-protocol`: the new protocol. It replaces the protocol of the table.
    #[getter]
    fn protocol(&self) -> ::core::option::Option<PyDeltaProtocol> {
        self.0
            .protocol
            .clone()
            .into_option()
            .map(PyDeltaProtocol::from)
    }

    /// For `remove-domain-metadata`: the domain names to remove.
    #[getter]
    fn domains(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.domains.clone()
    }

    /// For `set-partition-columns`: names of the partition columns. An empty list is sent too: it
    /// removes the partitioning.
    #[getter]
    fn partition_columns(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.partition_columns.clone()
    }

    /// For `update-metadata-snapshot-version`: version of the last commit
    /// (`delta.lastUpdateVersion`).
    #[getter]
    fn last_commit_version(&self) -> ::core::option::Option<i64> {
        self.0.last_commit_version
    }

    /// For `update-metadata-snapshot-version`: timestamp of the last commit, in epoch milliseconds
    /// (`delta.lastCommitTimestamp`).
    #[getter]
    fn last_commit_timestamp_ms(&self) -> ::core::option::Option<i64> {
        self.0.last_commit_timestamp_ms
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("action", false),
                ("updates", false),
                ("removals", false),
                ("columns", false),
                ("comment", false),
                ("commit", false),
                ("uniform", false),
                ("latest_published_version", false),
                ("protocol", false),
                ("domains", false),
                ("partition_columns", false),
                ("last_commit_version", false),
                ("last_commit_timestamp_ms", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaTableUpdate> for PyDeltaTableUpdate {
    fn from(value: super::delta::v1::DeltaTableUpdate) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaTableUpdate> for super::delta::v1::DeltaTableUpdate {
    fn from(value: PyDeltaTableUpdate) -> Self {
        value.0
    }
}

/// UniForm metadata committed with a Delta commit.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaUniform",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaUniform(pub super::delta::v1::DeltaUniform);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaUniform {
    #[new]
    #[pyo3(signature = (*, iceberg = None))]
    fn new(iceberg: ::core::option::Option<PyDeltaUniformIceberg>) -> Self {
        let mut inner = <super::delta::v1::DeltaUniform as ::core::default::Default>::default();
        {
            let value = iceberg;
            inner.iceberg = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        Self(inner)
    }

    /// The Iceberg metadata of the table.
    #[getter]
    fn iceberg(&self) -> ::core::option::Option<PyDeltaUniformIceberg> {
        self.0
            .iceberg
            .clone()
            .into_option()
            .map(PyDeltaUniformIceberg::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("iceberg", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaUniform> for PyDeltaUniform {
    fn from(value: super::delta::v1::DeltaUniform) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaUniform> for super::delta::v1::DeltaUniform {
    fn from(value: PyDeltaUniform) -> Self {
        value.0
    }
}

/// The Iceberg metadata of a UniForm table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaUniformIceberg",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaUniformIceberg(pub super::delta::v1::DeltaUniformIceberg);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaUniformIceberg {
    #[new]
    #[pyo3(
        signature = (
            *,
            metadata_location = None,
            converted_delta_version = None,
            converted_delta_timestamp = None,
            base_converted_delta_version = None
        )
    )]
    fn new(
        metadata_location: ::core::option::Option<::std::string::String>,
        converted_delta_version: ::core::option::Option<i64>,
        converted_delta_timestamp: ::core::option::Option<::std::string::String>,
        base_converted_delta_version: ::core::option::Option<i64>,
    ) -> Self {
        let mut inner =
            <super::delta::v1::DeltaUniformIceberg as ::core::default::Default>::default();
        {
            let value = metadata_location;
            inner.metadata_location = value;
        }
        {
            let value = converted_delta_version;
            inner.converted_delta_version = value;
        }
        {
            let value = converted_delta_timestamp;
            inner.converted_delta_timestamp = value;
        }
        {
            let value = base_converted_delta_version;
            inner.base_converted_delta_version = value;
        }
        Self(inner)
    }

    /// URL of the Iceberg metadata file. It must be inside the table.
    #[getter]
    fn metadata_location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.metadata_location.clone()
    }

    /// The Delta version that the Iceberg metadata was converted from.
    #[getter]
    fn converted_delta_version(&self) -> ::core::option::Option<i64> {
        self.0.converted_delta_version
    }

    /// When the Delta version was converted, as an ISO-8601 instant in UTC.
    #[getter]
    fn converted_delta_timestamp(&self) -> ::core::option::Option<::std::string::String> {
        self.0.converted_delta_timestamp.clone()
    }

    /// The Delta version of the Iceberg metadata that the conversion started from.
    #[getter]
    fn base_converted_delta_version(&self) -> ::core::option::Option<i64> {
        self.0.base_converted_delta_version
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("metadata_location", false),
                ("converted_delta_version", false),
                ("converted_delta_timestamp", false),
                ("base_converted_delta_version", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaUniformIceberg> for PyDeltaUniformIceberg {
    fn from(value: super::delta::v1::DeltaUniformIceberg) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaUniformIceberg> for super::delta::v1::DeltaUniformIceberg {
    fn from(value: PyDeltaUniformIceberg) -> Self {
        value.0
    }
}

/// UniForm conversion metadata. Only UniForm (Delta and Iceberg) tables have it.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaUniformMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaUniformMetadata(pub super::delta::v1::DeltaUniformMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaUniformMetadata {
    #[new]
    #[pyo3(signature = (*, iceberg = None))]
    fn new(iceberg: ::core::option::Option<PyDeltaUniformMetadataIceberg>) -> Self {
        let mut inner =
            <super::delta::v1::DeltaUniformMetadata as ::core::default::Default>::default();
        {
            let value = iceberg;
            inner.iceberg = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        Self(inner)
    }

    /// The Iceberg metadata of the table.
    #[getter]
    fn iceberg(&self) -> ::core::option::Option<PyDeltaUniformMetadataIceberg> {
        self.0
            .iceberg
            .clone()
            .into_option()
            .map(PyDeltaUniformMetadataIceberg::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("iceberg", false)])
    }
}

impl ::core::convert::From<super::delta::v1::DeltaUniformMetadata> for PyDeltaUniformMetadata {
    fn from(value: super::delta::v1::DeltaUniformMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaUniformMetadata> for super::delta::v1::DeltaUniformMetadata {
    fn from(value: PyDeltaUniformMetadata) -> Self {
        value.0
    }
}

/// The Iceberg metadata of a UniForm table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DeltaUniformMetadataIceberg",
    eq,
    frozen,
    module = "unitycatalog._core.delta",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDeltaUniformMetadataIceberg(pub super::delta::v1::DeltaUniformMetadataIceberg);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDeltaUniformMetadataIceberg {
    #[new]
    #[pyo3(
        signature = (
            *,
            metadata_location = None,
            converted_delta_version = None,
            converted_delta_timestamp = None,
            base_converted_delta_version = None
        )
    )]
    fn new(
        metadata_location: ::core::option::Option<::std::string::String>,
        converted_delta_version: ::core::option::Option<i64>,
        converted_delta_timestamp: ::core::option::Option<i64>,
        base_converted_delta_version: ::core::option::Option<i64>,
    ) -> Self {
        let mut inner =
            <super::delta::v1::DeltaUniformMetadataIceberg as ::core::default::Default>::default();
        {
            let value = metadata_location;
            inner.metadata_location = value;
        }
        {
            let value = converted_delta_version;
            inner.converted_delta_version = value;
        }
        {
            let value = converted_delta_timestamp;
            inner.converted_delta_timestamp = value;
        }
        {
            let value = base_converted_delta_version;
            inner.base_converted_delta_version = value;
        }
        Self(inner)
    }

    /// URL of the Iceberg metadata file.
    #[getter]
    fn metadata_location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.metadata_location.clone()
    }

    /// The Delta version that was converted to Iceberg.
    #[getter]
    fn converted_delta_version(&self) -> ::core::option::Option<i64> {
        self.0.converted_delta_version
    }

    /// When the Delta version was converted, in epoch milliseconds.
    #[getter]
    fn converted_delta_timestamp(&self) -> ::core::option::Option<i64> {
        self.0.converted_delta_timestamp
    }

    /// The Delta version that the incremental conversion started from.
    #[getter]
    fn base_converted_delta_version(&self) -> ::core::option::Option<i64> {
        self.0.base_converted_delta_version
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("metadata_location", false),
                ("converted_delta_version", false),
                ("converted_delta_timestamp", false),
                ("base_converted_delta_version", false),
            ],
        )
    }
}

impl ::core::convert::From<super::delta::v1::DeltaUniformMetadataIceberg>
    for PyDeltaUniformMetadataIceberg
{
    fn from(value: super::delta::v1::DeltaUniformMetadataIceberg) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDeltaUniformMetadataIceberg>
    for super::delta::v1::DeltaUniformMetadataIceberg
{
    fn from(value: PyDeltaUniformMetadataIceberg) -> Self {
        value.0
    }
}

/// An external location: a storage path and the credential that gives access to it.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ExternalLocationInfo",
    eq,
    frozen,
    module = "unitycatalog._core.external_locations",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyExternalLocationInfo(pub super::external_locations::v1::ExternalLocationInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyExternalLocationInfo {
    /// Name of the external location.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// URL of the storage path, such as `s3://bucket/path`.
    #[getter]
    fn url(&self) -> ::std::string::String {
        self.0.url.clone()
    }

    /// Name of the credential that gives access to the path. Absent when the credential was
    /// deleted with `force`.
    #[getter]
    fn credential_name(&self) -> ::core::option::Option<::std::string::String> {
        self.0.credential_name.clone()
    }

    /// Username of the owner of the external location.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Time at which the external location was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the external location.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the external location was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the external location.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the external location.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Unique identifier of the credential that gives access to the path.
    #[getter]
    fn credential_id(&self) -> ::std::string::String {
        self.0.credential_id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("url", false),
                ("credential_name", false),
                ("owner", false),
                ("comment", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("id", false),
                ("credential_id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::external_locations::v1::ExternalLocationInfo>
    for PyExternalLocationInfo
{
    fn from(value: super::external_locations::v1::ExternalLocationInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyExternalLocationInfo>
    for super::external_locations::v1::ExternalLocationInfo
{
    fn from(value: PyExternalLocationInfo) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListExternalLocationsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.external_locations",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListExternalLocationsResponse(
    pub super::external_locations::v1::ListExternalLocationsResponse,
);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListExternalLocationsResponse {
    /// The external locations.
    #[getter]
    fn external_locations(&self) -> ::std::vec::Vec<PyExternalLocationInfo> {
        self.0
            .external_locations
            .iter()
            .cloned()
            .map(PyExternalLocationInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("external_locations", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::external_locations::v1::ListExternalLocationsResponse>
    for PyListExternalLocationsResponse
{
    fn from(value: super::external_locations::v1::ListExternalLocationsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListExternalLocationsResponse>
    for super::external_locations::v1::ListExternalLocationsResponse
{
    fn from(value: PyListExternalLocationsResponse) -> Self {
        value.0
    }
}

/// The function to create.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "CreateFunction",
    eq,
    frozen,
    module = "unitycatalog._core.functions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyCreateFunction(pub super::functions::v1::CreateFunction);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyCreateFunction {
    #[new]
    #[pyo3(
        signature = (
            name,
            catalog_name,
            schema_name,
            routine_body,
            parameter_style,
            security_type,
            *,
            input_params = None,
            data_type = None,
            full_data_type = None,
            return_params = None,
            routine_definition = None,
            routine_dependencies = None,
            is_deterministic = None,
            sql_data_access = None,
            is_null_call = None,
            specific_name = None,
            comment = None,
            properties = None,
            external_language = None
        )
    )]
    fn new(
        name: ::std::string::String,
        catalog_name: ::std::string::String,
        schema_name: ::std::string::String,
        routine_body: PyRoutineBody,
        parameter_style: PyParameterStyle,
        security_type: PySecurityType,
        input_params: ::core::option::Option<PyFunctionParameterInfos>,
        data_type: ::core::option::Option<PyColumnTypeName>,
        full_data_type: ::core::option::Option<::std::string::String>,
        return_params: ::core::option::Option<PyFunctionParameterInfos>,
        routine_definition: ::core::option::Option<::std::string::String>,
        routine_dependencies: ::core::option::Option<PyDependencyList>,
        is_deterministic: ::core::option::Option<bool>,
        sql_data_access: ::core::option::Option<PySqlDataAccess>,
        is_null_call: ::core::option::Option<bool>,
        specific_name: ::core::option::Option<::std::string::String>,
        comment: ::core::option::Option<::std::string::String>,
        properties: ::core::option::Option<::std::string::String>,
        external_language: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner =
            <super::functions::v1::CreateFunction as ::core::default::Default>::default();
        {
            let value = name;
            inner.name = value;
        }
        {
            let value = catalog_name;
            inner.catalog_name = value;
        }
        {
            let value = schema_name;
            inner.schema_name = value;
        }
        {
            let value = ::core::option::Option::Some(routine_body);
            inner.routine_body = value
                .map(|e| {
                    ::buffa::EnumValue::Known(
                        <super::functions::v1::RoutineBody as ::core::convert::From<_>>::from(e),
                    )
                })
                .unwrap_or_default();
        }
        {
            let value = ::core::option::Option::Some(parameter_style);
            inner.parameter_style = value
                .map(|e| {
                    ::buffa::EnumValue::Known(
                        <super::functions::v1::ParameterStyle as ::core::convert::From<_>>::from(e),
                    )
                })
                .unwrap_or_default();
        }
        {
            let value = ::core::option::Option::Some(security_type);
            inner.security_type = value
                .map(|e| {
                    ::buffa::EnumValue::Known(
                        <super::functions::v1::SecurityType as ::core::convert::From<_>>::from(e),
                    )
                })
                .unwrap_or_default();
        }
        {
            let value = input_params;
            inner.input_params = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = data_type;
            inner.data_type = value.map(|e| {
                ::buffa::EnumValue::Known(
                    <super::tables::v1::ColumnTypeName as ::core::convert::From<_>>::from(e),
                )
            });
        }
        {
            let value = full_data_type;
            inner.full_data_type = value;
        }
        {
            let value = return_params;
            inner.return_params = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = routine_definition;
            inner.routine_definition = value;
        }
        {
            let value = routine_dependencies;
            inner.routine_dependencies = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = is_deterministic;
            inner.is_deterministic = value;
        }
        {
            let value = sql_data_access;
            inner.sql_data_access = value.map(|e| {
                ::buffa::EnumValue::Known(
                    <super::functions::v1::SqlDataAccess as ::core::convert::From<_>>::from(e),
                )
            });
        }
        {
            let value = is_null_call;
            inner.is_null_call = value;
        }
        {
            let value = specific_name;
            inner.specific_name = value;
        }
        {
            let value = comment;
            inner.comment = value;
        }
        {
            let value = properties;
            inner.properties = value;
        }
        {
            let value = external_language;
            inner.external_language = value;
        }
        Self(inner)
    }

    /// Name of the function, relative to the parent schema.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// Name of the parent schema.
    #[getter]
    fn schema_name(&self) -> ::std::string::String {
        self.0.schema_name.clone()
    }

    /// The input parameters of the function.
    #[getter]
    fn input_params(&self) -> ::core::option::Option<PyFunctionParameterInfos> {
        self.0
            .input_params
            .clone()
            .into_option()
            .map(PyFunctionParameterInfos::from)
    }

    /// Name of the data type of the result.
    #[getter]
    fn data_type(&self) -> ::core::option::Option<PyColumnTypeName> {
        self.0
            .data_type
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| <super::tables::v1::ColumnTypeName as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyColumnTypeName::from)
    }

    /// Full data type of the result as SQL text.
    #[getter]
    fn full_data_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.full_data_type.clone()
    }

    /// The result columns of a table function.
    #[getter]
    fn return_params(&self) -> ::core::option::Option<PyFunctionParameterInfos> {
        self.0
            .return_params
            .clone()
            .into_option()
            .map(PyFunctionParameterInfos::from)
    }

    /// Language of the body of the function.
    #[getter]
    fn routine_body(&self) -> ::core::option::Option<PyRoutineBody> {
        self.0
            .routine_body
            .as_known()
            .filter(|e| <super::functions::v1::RoutineBody as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyRoutineBody::from)
    }

    /// Body of the function.
    #[getter]
    fn routine_definition(&self) -> ::core::option::Option<::std::string::String> {
        self.0.routine_definition.clone()
    }

    /// The tables and functions that the function reads.
    #[getter]
    fn routine_dependencies(&self) -> ::core::option::Option<PyDependencyList> {
        self.0
            .routine_dependencies
            .clone()
            .into_option()
            .map(PyDependencyList::from)
    }

    /// Parameter style of the function.
    #[getter]
    fn parameter_style(&self) -> ::core::option::Option<PyParameterStyle> {
        self.0
            .parameter_style
            .as_known()
            .filter(|e| {
                <super::functions::v1::ParameterStyle as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PyParameterStyle::from)
    }

    /// Whether the function is deterministic.
    #[getter]
    fn is_deterministic(&self) -> ::core::option::Option<bool> {
        self.0.is_deterministic
    }

    /// How the function accesses SQL data.
    #[getter]
    fn sql_data_access(&self) -> ::core::option::Option<PySqlDataAccess> {
        self.0
            .sql_data_access
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| {
                <super::functions::v1::SqlDataAccess as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PySqlDataAccess::from)
    }

    /// Whether the function returns null when an argument is null.
    #[getter]
    fn is_null_call(&self) -> ::core::option::Option<bool> {
        self.0.is_null_call
    }

    /// Security type of the function.
    #[getter]
    fn security_type(&self) -> ::core::option::Option<PySecurityType> {
        self.0
            .security_type
            .as_known()
            .filter(|e| {
                <super::functions::v1::SecurityType as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PySecurityType::from)
    }

    /// Specific name of the function.
    #[getter]
    fn specific_name(&self) -> ::core::option::Option<::std::string::String> {
        self.0.specific_name.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Key-value properties of the function, as a JSON-encoded object.
    #[getter]
    fn properties(&self) -> ::core::option::Option<::std::string::String> {
        self.0.properties.clone()
    }

    /// Name of the external language of the body, such as `python`.
    #[getter]
    fn external_language(&self) -> ::core::option::Option<::std::string::String> {
        self.0.external_language.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("catalog_name", false),
                ("schema_name", false),
                ("input_params", false),
                ("data_type", false),
                ("full_data_type", false),
                ("return_params", false),
                ("routine_body", false),
                ("routine_definition", false),
                ("routine_dependencies", false),
                ("parameter_style", false),
                ("is_deterministic", false),
                ("sql_data_access", false),
                ("is_null_call", false),
                ("security_type", false),
                ("specific_name", false),
                ("comment", false),
                ("properties", false),
                ("external_language", false),
            ],
        )
    }
}

impl ::core::convert::From<super::functions::v1::CreateFunction> for PyCreateFunction {
    fn from(value: super::functions::v1::CreateFunction) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyCreateFunction> for super::functions::v1::CreateFunction {
    fn from(value: PyCreateFunction) -> Self {
        value.0
    }
}

/// A function: a user-defined function in the third layer of the three-level namespace, below a
/// schema.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "FunctionInfo",
    eq,
    frozen,
    module = "unitycatalog._core.functions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyFunctionInfo(pub super::functions::v1::FunctionInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyFunctionInfo {
    /// Name of the function, relative to the parent schema.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// Name of the parent schema.
    #[getter]
    fn schema_name(&self) -> ::std::string::String {
        self.0.schema_name.clone()
    }

    /// The input parameters of the function. Absent when there are none.
    #[getter]
    fn input_params(&self) -> ::core::option::Option<PyFunctionParameterInfos> {
        self.0
            .input_params
            .clone()
            .into_option()
            .map(PyFunctionParameterInfos::from)
    }

    /// Name of the data type of the result.
    #[getter]
    fn data_type(&self) -> ::core::option::Option<PyColumnTypeName> {
        self.0
            .data_type
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| <super::tables::v1::ColumnTypeName as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyColumnTypeName::from)
    }

    /// Full data type of the result as SQL text.
    #[getter]
    fn full_data_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.full_data_type.clone()
    }

    /// The result columns of a table function. Absent when there are none.
    #[getter]
    fn return_params(&self) -> ::core::option::Option<PyFunctionParameterInfos> {
        self.0
            .return_params
            .clone()
            .into_option()
            .map(PyFunctionParameterInfos::from)
    }

    /// Language of the body of the function.
    #[getter]
    fn routine_body(&self) -> ::core::option::Option<PyRoutineBody> {
        self.0
            .routine_body
            .as_known()
            .filter(|e| <super::functions::v1::RoutineBody as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyRoutineBody::from)
    }

    /// Body of the function.
    #[getter]
    fn routine_definition(&self) -> ::core::option::Option<::std::string::String> {
        self.0.routine_definition.clone()
    }

    /// The tables and functions that the function reads.
    #[getter]
    fn routine_dependencies(&self) -> ::core::option::Option<PyDependencyList> {
        self.0
            .routine_dependencies
            .clone()
            .into_option()
            .map(PyDependencyList::from)
    }

    /// Parameter style of the function.
    #[getter]
    fn parameter_style(&self) -> ::core::option::Option<PyParameterStyle> {
        self.0
            .parameter_style
            .as_known()
            .filter(|e| {
                <super::functions::v1::ParameterStyle as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PyParameterStyle::from)
    }

    /// Whether the function is deterministic.
    #[getter]
    fn is_deterministic(&self) -> ::core::option::Option<bool> {
        self.0.is_deterministic
    }

    /// How the function accesses SQL data.
    #[getter]
    fn sql_data_access(&self) -> ::core::option::Option<PySqlDataAccess> {
        self.0
            .sql_data_access
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| {
                <super::functions::v1::SqlDataAccess as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PySqlDataAccess::from)
    }

    /// Whether the function returns null when an argument is null.
    #[getter]
    fn is_null_call(&self) -> ::core::option::Option<bool> {
        self.0.is_null_call
    }

    /// Security type of the function.
    #[getter]
    fn security_type(&self) -> ::core::option::Option<PySecurityType> {
        self.0
            .security_type
            .as_known()
            .filter(|e| {
                <super::functions::v1::SecurityType as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PySecurityType::from)
    }

    /// Specific name of the function.
    #[getter]
    fn specific_name(&self) -> ::core::option::Option<::std::string::String> {
        self.0.specific_name.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Key-value properties of the function, as a JSON-encoded object.
    #[getter]
    fn properties(&self) -> ::core::option::Option<::std::string::String> {
        self.0.properties.clone()
    }

    /// Full name of the function, in the form `<catalog>.<schema>.<function>`.
    #[getter]
    fn full_name(&self) -> ::std::string::String {
        self.0.full_name.clone()
    }

    /// Username of the owner of the function.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the function was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the function.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the function was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the function.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the function.
    #[getter]
    fn function_id(&self) -> ::std::string::String {
        self.0.function_id.clone()
    }

    /// Name of the external language of the body, such as `python`.
    #[getter]
    fn external_language(&self) -> ::core::option::Option<::std::string::String> {
        self.0.external_language.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("catalog_name", false),
                ("schema_name", false),
                ("input_params", false),
                ("data_type", false),
                ("full_data_type", false),
                ("return_params", false),
                ("routine_body", false),
                ("routine_definition", false),
                ("routine_dependencies", false),
                ("parameter_style", false),
                ("is_deterministic", false),
                ("sql_data_access", false),
                ("is_null_call", false),
                ("security_type", false),
                ("specific_name", false),
                ("comment", false),
                ("properties", false),
                ("full_name", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("function_id", false),
                ("external_language", false),
            ],
        )
    }
}

impl ::core::convert::From<super::functions::v1::FunctionInfo> for PyFunctionInfo {
    fn from(value: super::functions::v1::FunctionInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyFunctionInfo> for super::functions::v1::FunctionInfo {
    fn from(value: PyFunctionInfo) -> Self {
        value.0
    }
}

/// A parameter or result column of a function.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "FunctionParameterInfo",
    eq,
    frozen,
    module = "unitycatalog._core.functions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyFunctionParameterInfo(pub super::functions::v1::FunctionParameterInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyFunctionParameterInfo {
    #[new]
    #[pyo3(
        signature = (
            name,
            type_text,
            type_name,
            *,
            type_json = None,
            type_precision = None,
            type_scale = None,
            type_interval_type = None,
            position = None,
            parameter_mode = None,
            parameter_type = None,
            parameter_default = None,
            comment = None
        )
    )]
    fn new(
        name: ::std::string::String,
        type_text: ::std::string::String,
        type_name: PyColumnTypeName,
        type_json: ::core::option::Option<::std::string::String>,
        type_precision: ::core::option::Option<i32>,
        type_scale: ::core::option::Option<i32>,
        type_interval_type: ::core::option::Option<::std::string::String>,
        position: ::core::option::Option<i32>,
        parameter_mode: ::core::option::Option<PyFunctionParameterMode>,
        parameter_type: ::core::option::Option<PyFunctionParameterType>,
        parameter_default: ::core::option::Option<::std::string::String>,
        comment: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner =
            <super::functions::v1::FunctionParameterInfo as ::core::default::Default>::default();
        {
            let value = name;
            inner.name = value;
        }
        {
            let value = type_text;
            inner.type_text = value;
        }
        {
            let value = ::core::option::Option::Some(type_name);
            inner.type_name = value
                .map(|e| {
                    ::buffa::EnumValue::Known(
                        <super::tables::v1::ColumnTypeName as ::core::convert::From<_>>::from(e),
                    )
                })
                .unwrap_or_default();
        }
        {
            let value = type_json;
            inner.type_json = value;
        }
        {
            let value = type_precision;
            inner.type_precision = value;
        }
        {
            let value = type_scale;
            inner.type_scale = value;
        }
        {
            let value = type_interval_type;
            inner.type_interval_type = value;
        }
        {
            let value = position;
            inner.position = value;
        }
        {
            let value = parameter_mode;
            inner.parameter_mode = value.map(|e| {
                ::buffa::EnumValue::Known(
                    <super::functions::v1::FunctionParameterMode as ::core::convert::From<_>>::from(
                        e,
                    ),
                )
            });
        }
        {
            let value = parameter_type;
            inner.parameter_type = value.map(|e| {
                ::buffa::EnumValue::Known(
                    <super::functions::v1::FunctionParameterType as ::core::convert::From<_>>::from(
                        e,
                    ),
                )
            });
        }
        {
            let value = parameter_default;
            inner.parameter_default = value;
        }
        {
            let value = comment;
            inner.comment = value;
        }
        Self(inner)
    }

    /// Name of the parameter.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Full data type of the parameter as SQL text.
    #[getter]
    fn type_text(&self) -> ::std::string::String {
        self.0.type_text.clone()
    }

    /// Full data type of the parameter as JSON.
    #[getter]
    fn type_json(&self) -> ::core::option::Option<::std::string::String> {
        self.0.type_json.clone()
    }

    /// Name of the data type of the parameter.
    #[getter]
    fn type_name(&self) -> ::core::option::Option<PyColumnTypeName> {
        self.0
            .type_name
            .as_known()
            .filter(|e| <super::tables::v1::ColumnTypeName as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyColumnTypeName::from)
    }

    /// Digits of precision. Required for decimal types.
    #[getter]
    fn type_precision(&self) -> ::core::option::Option<i32> {
        self.0.type_precision
    }

    /// Digits to the right of the decimal point. Required for decimal types.
    #[getter]
    fn type_scale(&self) -> ::core::option::Option<i32> {
        self.0.type_scale
    }

    /// Format of an interval type.
    #[getter]
    fn type_interval_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.type_interval_type.clone()
    }

    /// Position of the parameter, starting at 0.
    #[getter]
    fn position(&self) -> ::core::option::Option<i32> {
        self.0.position
    }

    /// Mode of the parameter.
    #[getter]
    fn parameter_mode(&self) -> ::core::option::Option<PyFunctionParameterMode> {
        self.0
            .parameter_mode
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| {
                <super::functions::v1::FunctionParameterMode as ::buffa::Enumeration>::to_i32(e)
                    != 0
            })
            .map(PyFunctionParameterMode::from)
    }

    /// Kind of the parameter.
    #[getter]
    fn parameter_type(&self) -> ::core::option::Option<PyFunctionParameterType> {
        self.0
            .parameter_type
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| {
                <super::functions::v1::FunctionParameterType as ::buffa::Enumeration>::to_i32(e)
                    != 0
            })
            .map(PyFunctionParameterType::from)
    }

    /// Default value of the parameter.
    #[getter]
    fn parameter_default(&self) -> ::core::option::Option<::std::string::String> {
        self.0.parameter_default.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("type_text", false),
                ("type_json", false),
                ("type_name", false),
                ("type_precision", false),
                ("type_scale", false),
                ("type_interval_type", false),
                ("position", false),
                ("parameter_mode", false),
                ("parameter_type", false),
                ("parameter_default", false),
                ("comment", false),
            ],
        )
    }
}

impl ::core::convert::From<super::functions::v1::FunctionParameterInfo>
    for PyFunctionParameterInfo
{
    fn from(value: super::functions::v1::FunctionParameterInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyFunctionParameterInfo>
    for super::functions::v1::FunctionParameterInfo
{
    fn from(value: PyFunctionParameterInfo) -> Self {
        value.0
    }
}

/// The parameters of a function.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "FunctionParameterInfos",
    eq,
    frozen,
    module = "unitycatalog._core.functions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyFunctionParameterInfos(pub super::functions::v1::FunctionParameterInfos);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyFunctionParameterInfos {
    #[new]
    #[pyo3(signature = (*, parameters = None))]
    fn new(parameters: ::core::option::Option<::std::vec::Vec<PyFunctionParameterInfo>>) -> Self {
        let mut inner =
            <super::functions::v1::FunctionParameterInfos as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = parameters {
            inner.parameters = value.into_iter().map(::core::convert::Into::into).collect();
        }
        Self(inner)
    }

    /// The parameters.
    #[getter]
    fn parameters(&self) -> ::std::vec::Vec<PyFunctionParameterInfo> {
        self.0
            .parameters
            .iter()
            .cloned()
            .map(PyFunctionParameterInfo::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("parameters", false)])
    }
}

impl ::core::convert::From<super::functions::v1::FunctionParameterInfos>
    for PyFunctionParameterInfos
{
    fn from(value: super::functions::v1::FunctionParameterInfos) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyFunctionParameterInfos>
    for super::functions::v1::FunctionParameterInfos
{
    fn from(value: PyFunctionParameterInfos) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListFunctionsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.functions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListFunctionsResponse(pub super::functions::v1::ListFunctionsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListFunctionsResponse {
    /// The functions.
    #[getter]
    fn functions(&self) -> ::std::vec::Vec<PyFunctionInfo> {
        self.0
            .functions
            .iter()
            .cloned()
            .map(PyFunctionInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("functions", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::functions::v1::ListFunctionsResponse>
    for PyListFunctionsResponse
{
    fn from(value: super::functions::v1::ListFunctionsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListFunctionsResponse>
    for super::functions::v1::ListFunctionsResponse
{
    fn from(value: PyListFunctionsResponse) -> Self {
        value.0
    }
}

/// The state of the server.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "HealthResponse",
    eq,
    frozen,
    module = "unitycatalog._core.health",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyHealthResponse(pub super::health::v1::HealthResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyHealthResponse {
    /// `ok`: the server answers requests.
    #[getter]
    fn status(&self) -> ::std::string::String {
        self.0.status.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("status", false)])
    }
}

impl ::core::convert::From<super::health::v1::HealthResponse> for PyHealthResponse {
    fn from(value: super::health::v1::HealthResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyHealthResponse> for super::health::v1::HealthResponse {
    fn from(value: PyHealthResponse) -> Self {
        value.0
    }
}

/// The server and its versions.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "VersionResponse",
    eq,
    frozen,
    module = "unitycatalog._core.health",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyVersionResponse(pub super::health::v1::VersionResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyVersionResponse {
    /// Name of the server: `unitycatalog-rs`.
    #[getter]
    fn server(&self) -> ::std::string::String {
        self.0.server.clone()
    }

    /// Version of the server, such as `0.1.0`.
    #[getter]
    fn version(&self) -> ::std::string::String {
        self.0.version.clone()
    }

    /// Version of the Unity Catalog API that the server implements: `2.1`.
    #[getter]
    fn api_version(&self) -> ::std::string::String {
        self.0.api_version.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("server", false),
                ("version", false),
                ("api_version", false),
            ],
        )
    }
}

impl ::core::convert::From<super::health::v1::VersionResponse> for PyVersionResponse {
    fn from(value: super::health::v1::VersionResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyVersionResponse> for super::health::v1::VersionResponse {
    fn from(value: PyVersionResponse) -> Self {
        value.0
    }
}

/// A blob of a statistics file.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergBlobMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergBlobMetadata(pub super::iceberg::v1::IcebergBlobMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergBlobMetadata {
    #[new]
    #[pyo3(
        signature = (
            *,
            blob_type = None,
            snapshot_id = None,
            sequence_number = None,
            fields = None,
            properties = None
        )
    )]
    fn new(
        blob_type: ::core::option::Option<::std::string::String>,
        snapshot_id: ::core::option::Option<i64>,
        sequence_number: ::core::option::Option<i64>,
        fields: ::core::option::Option<::std::vec::Vec<i32>>,
        properties: ::core::option::Option<
            ::std::collections::HashMap<::std::string::String, ::std::string::String>,
        >,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergBlobMetadata as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = blob_type {
            inner.blob_type = value;
        }
        if let ::core::option::Option::Some(value) = snapshot_id {
            inner.snapshot_id = value;
        }
        if let ::core::option::Option::Some(value) = sequence_number {
            inner.sequence_number = value;
        }
        if let ::core::option::Option::Some(value) = fields {
            inner.fields = value;
        }
        if let ::core::option::Option::Some(value) = properties {
            inner.properties = value;
        }
        Self(inner)
    }

    /// The type of the blob (JSON `type`), such as `apache-datasketches-theta-v1`.
    #[getter]
    fn blob_type(&self) -> ::std::string::String {
        self.0.blob_type.clone()
    }

    /// Id of the snapshot that the blob was computed from.
    #[getter]
    fn snapshot_id(&self) -> i64 {
        self.0.snapshot_id
    }

    /// Sequence number of the snapshot that the blob was computed from.
    #[getter]
    fn sequence_number(&self) -> i64 {
        self.0.sequence_number
    }

    /// Ids of the fields that the blob was computed from.
    #[getter]
    fn fields(&self) -> ::std::vec::Vec<i32> {
        self.0.fields.clone()
    }

    /// Properties of the blob.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("blob_type", false),
                ("snapshot_id", false),
                ("sequence_number", false),
                ("fields", false),
                ("properties", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergBlobMetadata> for PyIcebergBlobMetadata {
    fn from(value: super::iceberg::v1::IcebergBlobMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergBlobMetadata> for super::iceberg::v1::IcebergBlobMetadata {
    fn from(value: PyIcebergBlobMetadata) -> Self {
        value.0
    }
}

/// The client configuration of a catalog.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergConfigResponse",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergConfigResponse(pub super::iceberg::v1::IcebergConfigResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergConfigResponse {
    /// Properties that a client uses unless its own configuration sets them. Always empty.
    #[getter]
    fn defaults(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.defaults.clone()
    }

    /// Properties that replace the client's own configuration: `prefix`, the path prefix of the
    /// routes of the catalog (`catalogs/<catalog>`).
    #[getter]
    fn overrides(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.overrides.clone()
    }

    /// The routes that the server serves, such as `GET /v1/{prefix}/namespaces`. The write routes
    /// are in the list only when Iceberg writes are on.
    #[getter]
    fn endpoints(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.endpoints.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("defaults", false),
                ("overrides", false),
                ("endpoints", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergConfigResponse> for PyIcebergConfigResponse {
    fn from(value: super::iceberg::v1::IcebergConfigResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergConfigResponse> for super::iceberg::v1::IcebergConfigResponse {
    fn from(value: PyIcebergConfigResponse) -> Self {
        value.0
    }
}

/// An encryption key of a table, itself encrypted.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergEncryptedKey",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergEncryptedKey(pub super::iceberg::v1::IcebergEncryptedKey);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergEncryptedKey {
    #[new]
    #[pyo3(
        signature = (
            *,
            key_id = None,
            encrypted_key_metadata = None,
            encrypted_by_id = None,
            properties = None
        )
    )]
    fn new(
        key_id: ::core::option::Option<::std::string::String>,
        encrypted_key_metadata: ::core::option::Option<::std::string::String>,
        encrypted_by_id: ::core::option::Option<::std::string::String>,
        properties: ::core::option::Option<
            ::std::collections::HashMap<::std::string::String, ::std::string::String>,
        >,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergEncryptedKey as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = key_id {
            inner.key_id = value;
        }
        if let ::core::option::Option::Some(value) = encrypted_key_metadata {
            inner.encrypted_key_metadata = value;
        }
        {
            let value = encrypted_by_id;
            inner.encrypted_by_id = value;
        }
        if let ::core::option::Option::Some(value) = properties {
            inner.properties = value;
        }
        Self(inner)
    }

    /// Id of the key.
    #[getter]
    fn key_id(&self) -> ::std::string::String {
        self.0.key_id.clone()
    }

    /// The encrypted key metadata, Base64-encoded.
    #[getter]
    fn encrypted_key_metadata(&self) -> ::std::string::String {
        self.0.encrypted_key_metadata.clone()
    }

    /// Id of the key that encrypted this key.
    #[getter]
    fn encrypted_by_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.encrypted_by_id.clone()
    }

    /// Properties of the key.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("key_id", false),
                ("encrypted_key_metadata", false),
                ("encrypted_by_id", false),
                ("properties", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergEncryptedKey> for PyIcebergEncryptedKey {
    fn from(value: super::iceberg::v1::IcebergEncryptedKey) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergEncryptedKey> for super::iceberg::v1::IcebergEncryptedKey {
    fn from(value: PyIcebergEncryptedKey) -> Self {
        value.0
    }
}

/// A page of namespaces.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergListNamespacesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergListNamespacesResponse(pub super::iceberg::v1::IcebergListNamespacesResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergListNamespacesResponse {
    /// The namespaces, each as the list of its levels: one level, the schema name.
    #[gen_stub(
        override_return_type(type_repr = "list[list[typing.Any]]", imports = ("typing"))
    )]
    #[getter]
    fn namespaces(&self, py: ::pyo3::Python<'_>) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
        let items = self
            .0
            .namespaces
            .iter()
            .map(|value| json_list_to_py(py, value))
            .collect::<::pyo3::PyResult<::std::vec::Vec<_>>>()?;
        ::core::result::Result::Ok(::pyo3::types::PyList::new(py, items)?.into_any().unbind())
    }

    /// Opaque token of the next page. Absent on the last page and when no page was asked for.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("namespaces", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergListNamespacesResponse>
    for PyIcebergListNamespacesResponse
{
    fn from(value: super::iceberg::v1::IcebergListNamespacesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergListNamespacesResponse>
    for super::iceberg::v1::IcebergListNamespacesResponse
{
    fn from(value: PyIcebergListNamespacesResponse) -> Self {
        value.0
    }
}

/// A page of tables: the tables of a namespace that have Iceberg metadata.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergListTablesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergListTablesResponse(pub super::iceberg::v1::IcebergListTablesResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergListTablesResponse {
    /// The tables.
    #[getter]
    fn identifiers(&self) -> ::std::vec::Vec<PyIcebergTableIdentifier> {
        self.0
            .identifiers
            .iter()
            .cloned()
            .map(PyIcebergTableIdentifier::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page and when no page was asked for.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("identifiers", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergListTablesResponse>
    for PyIcebergListTablesResponse
{
    fn from(value: super::iceberg::v1::IcebergListTablesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergListTablesResponse>
    for super::iceberg::v1::IcebergListTablesResponse
{
    fn from(value: PyIcebergListTablesResponse) -> Self {
        value.0
    }
}

/// Fresh credentials for the files of a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergLoadCredentialsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergLoadCredentialsResponse(pub super::iceberg::v1::IcebergLoadCredentialsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergLoadCredentialsResponse {
    /// The credentials. Empty for a local table.
    #[getter]
    fn storage_credentials(&self) -> ::std::vec::Vec<PyIcebergStorageCredential> {
        self.0
            .storage_credentials
            .iter()
            .cloned()
            .map(PyIcebergStorageCredential::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("storage_credentials", false)])
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergLoadCredentialsResponse>
    for PyIcebergLoadCredentialsResponse
{
    fn from(value: super::iceberg::v1::IcebergLoadCredentialsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergLoadCredentialsResponse>
    for super::iceberg::v1::IcebergLoadCredentialsResponse
{
    fn from(value: PyIcebergLoadCredentialsResponse) -> Self {
        value.0
    }
}

/// The metadata of a table, with the properties a client needs to read and write its files.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergLoadTableResponse",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergLoadTableResponse(pub super::iceberg::v1::IcebergLoadTableResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergLoadTableResponse {
    /// Location of the current metadata file of the table. Absent for a staged create.
    #[getter]
    fn metadata_location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.metadata_location.clone()
    }

    /// The table metadata.
    #[getter]
    fn metadata(&self) -> ::core::option::Option<PyIcebergTableMetadata> {
        self.0
            .metadata
            .clone()
            .into_option()
            .map(PyIcebergTableMetadata::from)
    }

    /// FileIO properties for the location of the table: vended credentials under Iceberg's
    /// property names, and the endpoint that renews them. Empty for a local table.
    #[getter]
    fn config(&self) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.config.clone()
    }

    /// Weak tag of the metadata version and the snapshots mode, from the `ETag` header. Only a
    /// table load sends it.
    #[getter]
    fn etag(&self) -> ::core::option::Option<::std::string::String> {
        self.0.etag.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("metadata_location", false),
                ("metadata", false),
                ("config", false),
                ("etag", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergLoadTableResponse>
    for PyIcebergLoadTableResponse
{
    fn from(value: super::iceberg::v1::IcebergLoadTableResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergLoadTableResponse>
    for super::iceberg::v1::IcebergLoadTableResponse
{
    fn from(value: PyIcebergLoadTableResponse) -> Self {
        value.0
    }
}

/// An entry of the metadata log: a previous metadata file.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergMetadataLogEntry",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergMetadataLogEntry(pub super::iceberg::v1::IcebergMetadataLogEntry);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergMetadataLogEntry {
    /// Location of the metadata file.
    #[getter]
    fn metadata_file(&self) -> ::std::string::String {
        self.0.metadata_file.clone()
    }

    /// Time at which the metadata file was written, in epoch milliseconds.
    #[getter]
    fn timestamp_ms(&self) -> i64 {
        self.0.timestamp_ms
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("metadata_file", false), ("timestamp_ms", false)],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergMetadataLogEntry>
    for PyIcebergMetadataLogEntry
{
    fn from(value: super::iceberg::v1::IcebergMetadataLogEntry) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergMetadataLogEntry>
    for super::iceberg::v1::IcebergMetadataLogEntry
{
    fn from(value: PyIcebergMetadataLogEntry) -> Self {
        value.0
    }
}

/// A namespace and its properties.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergNamespaceResponse",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergNamespaceResponse(pub super::iceberg::v1::IcebergNamespaceResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergNamespaceResponse {
    /// The levels of the namespace: one level, the schema name.
    #[getter]
    fn namespace(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.namespace.clone()
    }

    /// Properties of the namespace.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("namespace", false), ("properties", false)])
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergNamespaceResponse>
    for PyIcebergNamespaceResponse
{
    fn from(value: super::iceberg::v1::IcebergNamespaceResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergNamespaceResponse>
    for super::iceberg::v1::IcebergNamespaceResponse
{
    fn from(value: PyIcebergNamespaceResponse) -> Self {
        value.0
    }
}

/// A partition field: a transform of a source column.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergPartitionField",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergPartitionField(pub super::iceberg::v1::IcebergPartitionField);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergPartitionField {
    #[new]
    #[pyo3(
        signature = (*, source_id = None, field_id = None, name = None, transform = None)
    )]
    fn new(
        source_id: ::core::option::Option<i32>,
        field_id: ::core::option::Option<i32>,
        name: ::core::option::Option<::std::string::String>,
        transform: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergPartitionField as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = source_id {
            inner.source_id = value;
        }
        if let ::core::option::Option::Some(value) = field_id {
            inner.field_id = value;
        }
        if let ::core::option::Option::Some(value) = name {
            inner.name = value;
        }
        if let ::core::option::Option::Some(value) = transform {
            inner.transform = value;
        }
        Self(inner)
    }

    /// Id of the source column.
    #[getter]
    fn source_id(&self) -> i32 {
        self.0.source_id
    }

    /// Id of the partition field. In requests, 0 lets the server assign it.
    #[getter]
    fn field_id(&self) -> i32 {
        self.0.field_id
    }

    /// Name of the partition field.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// The transform: `identity`, `year`, `month`, `day`, `hour`, `void`, `bucket[N]` or
    /// `truncate[W]`.
    #[getter]
    fn transform(&self) -> ::std::string::String {
        self.0.transform.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("source_id", false),
                ("field_id", false),
                ("name", false),
                ("transform", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergPartitionField> for PyIcebergPartitionField {
    fn from(value: super::iceberg::v1::IcebergPartitionField) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergPartitionField> for super::iceberg::v1::IcebergPartitionField {
    fn from(value: PyIcebergPartitionField) -> Self {
        value.0
    }
}

/// How a table is partitioned.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergPartitionSpec",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergPartitionSpec(pub super::iceberg::v1::IcebergPartitionSpec);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergPartitionSpec {
    #[new]
    #[pyo3(signature = (*, spec_id = None, fields = None))]
    fn new(
        spec_id: ::core::option::Option<i32>,
        fields: ::core::option::Option<::std::vec::Vec<PyIcebergPartitionField>>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergPartitionSpec as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = spec_id {
            inner.spec_id = value;
        }
        if let ::core::option::Option::Some(value) = fields {
            inner.fields = value.into_iter().map(::core::convert::Into::into).collect();
        }
        Self(inner)
    }

    /// Id of the spec. In requests, the server assigns it.
    #[getter]
    fn spec_id(&self) -> i32 {
        self.0.spec_id
    }

    /// The partition fields. None for an unpartitioned table.
    #[getter]
    fn fields(&self) -> ::std::vec::Vec<PyIcebergPartitionField> {
        self.0
            .fields
            .iter()
            .cloned()
            .map(PyIcebergPartitionField::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("spec_id", false), ("fields", false)])
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergPartitionSpec> for PyIcebergPartitionSpec {
    fn from(value: super::iceberg::v1::IcebergPartitionSpec) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergPartitionSpec> for super::iceberg::v1::IcebergPartitionSpec {
    fn from(value: PyIcebergPartitionSpec) -> Self {
        value.0
    }
}

/// A file of partition statistics for a snapshot.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergPartitionStatisticsFile",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergPartitionStatisticsFile(pub super::iceberg::v1::IcebergPartitionStatisticsFile);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergPartitionStatisticsFile {
    #[new]
    #[pyo3(
        signature = (
            *,
            snapshot_id = None,
            statistics_path = None,
            file_size_in_bytes = None
        )
    )]
    fn new(
        snapshot_id: ::core::option::Option<i64>,
        statistics_path: ::core::option::Option<::std::string::String>,
        file_size_in_bytes: ::core::option::Option<i64>,
    ) -> Self {
        let mut inner = <super::iceberg::v1::IcebergPartitionStatisticsFile as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = snapshot_id {
            inner.snapshot_id = value;
        }
        if let ::core::option::Option::Some(value) = statistics_path {
            inner.statistics_path = value;
        }
        if let ::core::option::Option::Some(value) = file_size_in_bytes {
            inner.file_size_in_bytes = value;
        }
        Self(inner)
    }

    /// Id of the snapshot that the statistics describe.
    #[getter]
    fn snapshot_id(&self) -> i64 {
        self.0.snapshot_id
    }

    /// Location of the statistics file.
    #[getter]
    fn statistics_path(&self) -> ::std::string::String {
        self.0.statistics_path.clone()
    }

    /// Size of the file, in bytes.
    #[getter]
    fn file_size_in_bytes(&self) -> i64 {
        self.0.file_size_in_bytes
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("snapshot_id", false),
                ("statistics_path", false),
                ("file_size_in_bytes", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergPartitionStatisticsFile>
    for PyIcebergPartitionStatisticsFile
{
    fn from(value: super::iceberg::v1::IcebergPartitionStatisticsFile) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergPartitionStatisticsFile>
    for super::iceberg::v1::IcebergPartitionStatisticsFile
{
    fn from(value: PyIcebergPartitionStatisticsFile) -> Self {
        value.0
    }
}

/// An Iceberg table schema: a struct type with an id.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergSchema",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergSchema(pub super::iceberg::v1::IcebergSchema);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergSchema {
    #[new]
    #[pyo3(
        signature = (
            schema_type,
            fields,
            *,
            schema_id = None,
            identifier_field_ids = None
        )
    )]
    fn new(
        schema_type: ::std::string::String,
        fields: ::std::vec::Vec<PyIcebergStructField>,
        schema_id: ::core::option::Option<i32>,
        identifier_field_ids: ::core::option::Option<::std::vec::Vec<i32>>,
    ) -> Self {
        let mut inner = <super::iceberg::v1::IcebergSchema as ::core::default::Default>::default();
        {
            let value = schema_type;
            inner.schema_type = value;
        }
        {
            let value = fields;
            inner.fields = value.into_iter().map(::core::convert::Into::into).collect();
        }
        if let ::core::option::Option::Some(value) = schema_id {
            inner.schema_id = value;
        }
        if let ::core::option::Option::Some(value) = identifier_field_ids {
            inner.identifier_field_ids = value;
        }
        Self(inner)
    }

    /// The type of the schema (JSON `type`): always `struct`.
    #[getter]
    fn schema_type(&self) -> ::std::string::String {
        self.0.schema_type.clone()
    }

    /// Id of the schema. In requests, the server assigns the ids.
    #[getter]
    fn schema_id(&self) -> i32 {
        self.0.schema_id
    }

    /// Ids of the fields that identify a row.
    #[getter]
    fn identifier_field_ids(&self) -> ::std::vec::Vec<i32> {
        self.0.identifier_field_ids.clone()
    }

    /// The fields of the schema.
    #[getter]
    fn fields(&self) -> ::std::vec::Vec<PyIcebergStructField> {
        self.0
            .fields
            .iter()
            .cloned()
            .map(PyIcebergStructField::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schema_type", false),
                ("schema_id", false),
                ("identifier_field_ids", false),
                ("fields", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergSchema> for PyIcebergSchema {
    fn from(value: super::iceberg::v1::IcebergSchema) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergSchema> for super::iceberg::v1::IcebergSchema {
    fn from(value: PyIcebergSchema) -> Self {
        value.0
    }
}

/// A snapshot: the state of a table at a point in time.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergSnapshot",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergSnapshot(pub super::iceberg::v1::IcebergSnapshot);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergSnapshot {
    #[new]
    #[pyo3(
        signature = (
            *,
            snapshot_id = None,
            parent_snapshot_id = None,
            sequence_number = None,
            timestamp_ms = None,
            manifest_list = None,
            summary = None,
            schema_id = None,
            first_row_id = None,
            added_rows = None,
            key_id = None
        )
    )]
    fn new(
        snapshot_id: ::core::option::Option<i64>,
        parent_snapshot_id: ::core::option::Option<i64>,
        sequence_number: ::core::option::Option<i64>,
        timestamp_ms: ::core::option::Option<i64>,
        manifest_list: ::core::option::Option<::std::string::String>,
        summary: ::core::option::Option<
            ::std::collections::HashMap<::std::string::String, ::std::string::String>,
        >,
        schema_id: ::core::option::Option<i32>,
        first_row_id: ::core::option::Option<i64>,
        added_rows: ::core::option::Option<i64>,
        key_id: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergSnapshot as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = snapshot_id {
            inner.snapshot_id = value;
        }
        {
            let value = parent_snapshot_id;
            inner.parent_snapshot_id = value;
        }
        if let ::core::option::Option::Some(value) = sequence_number {
            inner.sequence_number = value;
        }
        if let ::core::option::Option::Some(value) = timestamp_ms {
            inner.timestamp_ms = value;
        }
        if let ::core::option::Option::Some(value) = manifest_list {
            inner.manifest_list = value;
        }
        if let ::core::option::Option::Some(value) = summary {
            inner.summary = value;
        }
        {
            let value = schema_id;
            inner.schema_id = value;
        }
        {
            let value = first_row_id;
            inner.first_row_id = value;
        }
        {
            let value = added_rows;
            inner.added_rows = value;
        }
        {
            let value = key_id;
            inner.key_id = value;
        }
        Self(inner)
    }

    /// Id of the snapshot.
    #[getter]
    fn snapshot_id(&self) -> i64 {
        self.0.snapshot_id
    }

    /// Id of the parent snapshot. Absent for a snapshot without a parent.
    #[getter]
    fn parent_snapshot_id(&self) -> ::core::option::Option<i64> {
        self.0.parent_snapshot_id
    }

    /// Sequence number of the snapshot. Absent (0) in format version 1.
    #[getter]
    fn sequence_number(&self) -> i64 {
        self.0.sequence_number
    }

    /// Time at which the snapshot was made, in epoch milliseconds.
    #[getter]
    fn timestamp_ms(&self) -> i64 {
        self.0.timestamp_ms
    }

    /// Location of the manifest list of the snapshot.
    #[getter]
    fn manifest_list(&self) -> ::std::string::String {
        self.0.manifest_list.clone()
    }

    /// Summary of the changes: `operation` (`append`, `replace`, `overwrite` or `delete`) and other
    /// properties. Requests must include `operation`.
    #[getter]
    fn summary(&self) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.summary.clone()
    }

    /// Id of the current schema when the snapshot was made.
    #[getter]
    fn schema_id(&self) -> ::core::option::Option<i32> {
        self.0.schema_id
    }

    /// First row id of the rows the snapshot adds. Format version 3 only.
    #[getter]
    fn first_row_id(&self) -> ::core::option::Option<i64> {
        self.0.first_row_id
    }

    /// Number of rows the snapshot adds. Format version 3 only.
    #[getter]
    fn added_rows(&self) -> ::core::option::Option<i64> {
        self.0.added_rows
    }

    /// Id of the encryption key of the manifest list. Format version 3 only.
    #[getter]
    fn key_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.key_id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("snapshot_id", false),
                ("parent_snapshot_id", false),
                ("sequence_number", false),
                ("timestamp_ms", false),
                ("manifest_list", false),
                ("summary", false),
                ("schema_id", false),
                ("first_row_id", false),
                ("added_rows", false),
                ("key_id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergSnapshot> for PyIcebergSnapshot {
    fn from(value: super::iceberg::v1::IcebergSnapshot) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergSnapshot> for super::iceberg::v1::IcebergSnapshot {
    fn from(value: PyIcebergSnapshot) -> Self {
        value.0
    }
}

/// An entry of the snapshot log: when a snapshot became current.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergSnapshotLogEntry",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergSnapshotLogEntry(pub super::iceberg::v1::IcebergSnapshotLogEntry);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergSnapshotLogEntry {
    /// Id of the snapshot.
    #[getter]
    fn snapshot_id(&self) -> i64 {
        self.0.snapshot_id
    }

    /// Time at which the snapshot became current, in epoch milliseconds.
    #[getter]
    fn timestamp_ms(&self) -> i64 {
        self.0.timestamp_ms
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("snapshot_id", false), ("timestamp_ms", false)],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergSnapshotLogEntry>
    for PyIcebergSnapshotLogEntry
{
    fn from(value: super::iceberg::v1::IcebergSnapshotLogEntry) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergSnapshotLogEntry>
    for super::iceberg::v1::IcebergSnapshotLogEntry
{
    fn from(value: PyIcebergSnapshotLogEntry) -> Self {
        value.0
    }
}

/// A named reference to a snapshot: a branch or a tag.
///
/// `type` is `branch` (with `min-snapshots-to-keep`, `max-snapshot-age-ms` and `max-ref-age-ms`)
/// or `tag` (with `max-ref-age-ms`).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergSnapshotReference",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergSnapshotReference(pub super::iceberg::v1::IcebergSnapshotReference);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergSnapshotReference {
    /// Id of the snapshot that the reference points at.
    #[getter]
    fn snapshot_id(&self) -> i64 {
        self.0.snapshot_id
    }

    /// The kind of reference: `branch` or `tag`.
    #[getter]
    fn get_type(&self) -> ::std::string::String {
        self.0.r#type.clone()
    }

    /// Branch only: the minimum number of snapshots to keep when snapshots expire.
    #[getter]
    fn min_snapshots_to_keep(&self) -> ::core::option::Option<i32> {
        self.0.min_snapshots_to_keep
    }

    /// Branch only: the maximum age of the snapshots to keep, in milliseconds.
    #[getter]
    fn max_snapshot_age_ms(&self) -> ::core::option::Option<i64> {
        self.0.max_snapshot_age_ms
    }

    /// The maximum age of the reference, in milliseconds. The main branch never expires.
    #[getter]
    fn max_ref_age_ms(&self) -> ::core::option::Option<i64> {
        self.0.max_ref_age_ms
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("snapshot_id", false),
                ("type", false),
                ("min_snapshots_to_keep", false),
                ("max_snapshot_age_ms", false),
                ("max_ref_age_ms", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergSnapshotReference>
    for PyIcebergSnapshotReference
{
    fn from(value: super::iceberg::v1::IcebergSnapshotReference) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergSnapshotReference>
    for super::iceberg::v1::IcebergSnapshotReference
{
    fn from(value: PyIcebergSnapshotReference) -> Self {
        value.0
    }
}

/// A sort field: a transform of a source column and a direction.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergSortField",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergSortField(pub super::iceberg::v1::IcebergSortField);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergSortField {
    #[new]
    #[pyo3(
        signature = (
            *,
            source_id = None,
            transform = None,
            direction = None,
            null_order = None
        )
    )]
    fn new(
        source_id: ::core::option::Option<i32>,
        transform: ::core::option::Option<::std::string::String>,
        direction: ::core::option::Option<::std::string::String>,
        null_order: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergSortField as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = source_id {
            inner.source_id = value;
        }
        if let ::core::option::Option::Some(value) = transform {
            inner.transform = value;
        }
        if let ::core::option::Option::Some(value) = direction {
            inner.direction = value;
        }
        if let ::core::option::Option::Some(value) = null_order {
            inner.null_order = value;
        }
        Self(inner)
    }

    /// Id of the source column.
    #[getter]
    fn source_id(&self) -> i32 {
        self.0.source_id
    }

    /// The transform, as in a partition field, such as `identity`.
    #[getter]
    fn transform(&self) -> ::std::string::String {
        self.0.transform.clone()
    }

    /// The direction: `asc` or `desc`.
    #[getter]
    fn direction(&self) -> ::std::string::String {
        self.0.direction.clone()
    }

    /// Where nulls go: `nulls-first` or `nulls-last`.
    #[getter]
    fn null_order(&self) -> ::std::string::String {
        self.0.null_order.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("source_id", false),
                ("transform", false),
                ("direction", false),
                ("null_order", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergSortField> for PyIcebergSortField {
    fn from(value: super::iceberg::v1::IcebergSortField) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergSortField> for super::iceberg::v1::IcebergSortField {
    fn from(value: PyIcebergSortField) -> Self {
        value.0
    }
}

/// How the data files of a table are sorted.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergSortOrder",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergSortOrder(pub super::iceberg::v1::IcebergSortOrder);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergSortOrder {
    #[new]
    #[pyo3(signature = (*, order_id = None, fields = None))]
    fn new(
        order_id: ::core::option::Option<i64>,
        fields: ::core::option::Option<::std::vec::Vec<PyIcebergSortField>>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergSortOrder as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = order_id {
            inner.order_id = value;
        }
        if let ::core::option::Option::Some(value) = fields {
            inner.fields = value.into_iter().map(::core::convert::Into::into).collect();
        }
        Self(inner)
    }

    /// Id of the sort order. 0 is the unsorted order.
    #[getter]
    fn order_id(&self) -> i64 {
        self.0.order_id
    }

    /// The sort fields, in order. None for the unsorted order.
    #[getter]
    fn fields(&self) -> ::std::vec::Vec<PyIcebergSortField> {
        self.0
            .fields
            .iter()
            .cloned()
            .map(PyIcebergSortField::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("order_id", false), ("fields", false)])
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergSortOrder> for PyIcebergSortOrder {
    fn from(value: super::iceberg::v1::IcebergSortOrder) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergSortOrder> for super::iceberg::v1::IcebergSortOrder {
    fn from(value: PyIcebergSortOrder) -> Self {
        value.0
    }
}

/// A Puffin file of table statistics for a snapshot.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergStatisticsFile",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergStatisticsFile(pub super::iceberg::v1::IcebergStatisticsFile);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergStatisticsFile {
    #[new]
    #[pyo3(
        signature = (
            *,
            snapshot_id = None,
            statistics_path = None,
            file_size_in_bytes = None,
            file_footer_size_in_bytes = None,
            key_metadata = None,
            blob_metadata = None
        )
    )]
    fn new(
        snapshot_id: ::core::option::Option<i64>,
        statistics_path: ::core::option::Option<::std::string::String>,
        file_size_in_bytes: ::core::option::Option<i64>,
        file_footer_size_in_bytes: ::core::option::Option<i64>,
        key_metadata: ::core::option::Option<::std::string::String>,
        blob_metadata: ::core::option::Option<::std::vec::Vec<PyIcebergBlobMetadata>>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergStatisticsFile as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = snapshot_id {
            inner.snapshot_id = value;
        }
        if let ::core::option::Option::Some(value) = statistics_path {
            inner.statistics_path = value;
        }
        if let ::core::option::Option::Some(value) = file_size_in_bytes {
            inner.file_size_in_bytes = value;
        }
        if let ::core::option::Option::Some(value) = file_footer_size_in_bytes {
            inner.file_footer_size_in_bytes = value;
        }
        {
            let value = key_metadata;
            inner.key_metadata = value;
        }
        if let ::core::option::Option::Some(value) = blob_metadata {
            inner.blob_metadata = value.into_iter().map(::core::convert::Into::into).collect();
        }
        Self(inner)
    }

    /// Id of the snapshot that the statistics describe.
    #[getter]
    fn snapshot_id(&self) -> i64 {
        self.0.snapshot_id
    }

    /// Location of the statistics file.
    #[getter]
    fn statistics_path(&self) -> ::std::string::String {
        self.0.statistics_path.clone()
    }

    /// Size of the file, in bytes.
    #[getter]
    fn file_size_in_bytes(&self) -> i64 {
        self.0.file_size_in_bytes
    }

    /// Size of the file footer, in bytes.
    #[getter]
    fn file_footer_size_in_bytes(&self) -> i64 {
        self.0.file_footer_size_in_bytes
    }

    /// Base64-encoded key metadata of an encrypted file.
    #[getter]
    fn key_metadata(&self) -> ::core::option::Option<::std::string::String> {
        self.0.key_metadata.clone()
    }

    /// The blobs of the file.
    #[getter]
    fn blob_metadata(&self) -> ::std::vec::Vec<PyIcebergBlobMetadata> {
        self.0
            .blob_metadata
            .iter()
            .cloned()
            .map(PyIcebergBlobMetadata::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("snapshot_id", false),
                ("statistics_path", false),
                ("file_size_in_bytes", false),
                ("file_footer_size_in_bytes", false),
                ("key_metadata", false),
                ("blob_metadata", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergStatisticsFile> for PyIcebergStatisticsFile {
    fn from(value: super::iceberg::v1::IcebergStatisticsFile) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergStatisticsFile> for super::iceberg::v1::IcebergStatisticsFile {
    fn from(value: PyIcebergStatisticsFile) -> Self {
        value.0
    }
}

/// Credentials for the files below a location prefix.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergStorageCredential",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergStorageCredential(pub super::iceberg::v1::IcebergStorageCredential);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergStorageCredential {
    /// The location prefix that the credentials are for.
    #[getter]
    fn prefix(&self) -> ::std::string::String {
        self.0.prefix.clone()
    }

    /// FileIO properties: the credentials under Iceberg's property names.
    #[getter]
    fn config(&self) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.config.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("prefix", false), ("config", false)])
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergStorageCredential>
    for PyIcebergStorageCredential
{
    fn from(value: super::iceberg::v1::IcebergStorageCredential) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergStorageCredential>
    for super::iceberg::v1::IcebergStorageCredential
{
    fn from(value: PyIcebergStorageCredential) -> Self {
        value.0
    }
}

/// A field of a struct type, such as a column of a schema.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergStructField",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergStructField(pub super::iceberg::v1::IcebergStructField);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergStructField {
    #[new]
    #[pyo3(
        signature = (
            id,
            name,
            required,
            field_type,
            *,
            doc = None,
            initial_default = None,
            write_default = None
        )
    )]
    fn new(
        id: i32,
        name: ::std::string::String,
        required: bool,
        field_type: ::pyo3::Bound<'_, ::pyo3::PyAny>,
        doc: ::core::option::Option<::std::string::String>,
        initial_default: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
        write_default: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
    ) -> ::pyo3::PyResult<Self> {
        let mut inner =
            <super::iceberg::v1::IcebergStructField as ::core::default::Default>::default();
        {
            let value = id;
            inner.id = value;
        }
        {
            let value = name;
            inner.name = value;
        }
        {
            let value = required;
            inner.required = value;
        }
        {
            let value = field_type;
            inner.field_type = ::buffa::MessageField::some(json_value_from_py(&value)?);
        }
        {
            let value = doc;
            inner.doc = value;
        }
        if let ::core::option::Option::Some(value) = initial_default {
            inner.initial_default = ::buffa::MessageField::some(json_value_from_py(&value)?);
        }
        if let ::core::option::Option::Some(value) = write_default {
            inner.write_default = ::buffa::MessageField::some(json_value_from_py(&value)?);
        }
        ::core::result::Result::Ok(Self(inner))
    }

    /// Id of the field, unique in the table schema.
    #[getter]
    fn id(&self) -> i32 {
        self.0.id
    }

    /// Name of the field.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Whether the field cannot be null.
    #[getter]
    fn required(&self) -> bool {
        self.0.required
    }

    /// The Iceberg type of the field (JSON `type`): a string for a primitive type (`long`,
    /// `decimal(10,2)`, `fixed[16]`), an object for a `struct`, `list` or `map` type.
    #[gen_stub(override_return_type(type_repr = "typing.Any", imports = ("typing")))]
    #[getter]
    fn field_type(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .field_type
            .as_option()
            .map(|value| json_value_to_py(py, value))
            .transpose()
    }

    /// Description of the field.
    #[getter]
    fn doc(&self) -> ::core::option::Option<::std::string::String> {
        self.0.doc.clone()
    }

    /// Value of the field in the rows written before the field was added, in Iceberg's JSON form
    /// for single values.
    #[gen_stub(override_return_type(type_repr = "typing.Any", imports = ("typing")))]
    #[getter]
    fn initial_default(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .initial_default
            .as_option()
            .map(|value| json_value_to_py(py, value))
            .transpose()
    }

    /// Value of the field when a writer does not give one, in Iceberg's JSON form for single
    /// values.
    #[gen_stub(override_return_type(type_repr = "typing.Any", imports = ("typing")))]
    #[getter]
    fn write_default(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .write_default
            .as_option()
            .map(|value| json_value_to_py(py, value))
            .transpose()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("id", false),
                ("name", false),
                ("required", false),
                ("field_type", false),
                ("doc", false),
                ("initial_default", false),
                ("write_default", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergStructField> for PyIcebergStructField {
    fn from(value: super::iceberg::v1::IcebergStructField) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergStructField> for super::iceberg::v1::IcebergStructField {
    fn from(value: PyIcebergStructField) -> Self {
        value.0
    }
}

/// A table by its namespace and name.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergTableIdentifier",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergTableIdentifier(pub super::iceberg::v1::IcebergTableIdentifier);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergTableIdentifier {
    #[new]
    #[pyo3(signature = (namespace, name))]
    fn new(namespace: ::std::vec::Vec<::std::string::String>, name: ::std::string::String) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergTableIdentifier as ::core::default::Default>::default();
        {
            let value = namespace;
            inner.namespace = value;
        }
        {
            let value = name;
            inner.name = value;
        }
        Self(inner)
    }

    /// The levels of the namespace: one level, the schema name.
    #[getter]
    fn namespace(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.namespace.clone()
    }

    /// Name of the table.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("namespace", false), ("name", false)])
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergTableIdentifier>
    for PyIcebergTableIdentifier
{
    fn from(value: super::iceberg::v1::IcebergTableIdentifier) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergTableIdentifier>
    for super::iceberg::v1::IcebergTableIdentifier
{
    fn from(value: PyIcebergTableIdentifier) -> Self {
        value.0
    }
}

/// Iceberg table metadata, in format version 1, 2 or 3.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergTableMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergTableMetadata(pub super::iceberg::v1::IcebergTableMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergTableMetadata {
    /// The format version: 1, 2 or 3.
    #[getter]
    fn format_version(&self) -> i32 {
        self.0.format_version
    }

    /// UUID of the table.
    #[getter]
    fn table_uuid(&self) -> ::std::string::String {
        self.0.table_uuid.clone()
    }

    /// Base location of the table.
    #[getter]
    fn location(&self) -> ::std::string::String {
        self.0.location.clone()
    }

    /// The highest sequence number of the table. Absent (0) in format version 1.
    #[getter]
    fn last_sequence_number(&self) -> i64 {
        self.0.last_sequence_number
    }

    /// Time at which the table was last changed, in epoch milliseconds.
    #[getter]
    fn last_updated_ms(&self) -> i64 {
        self.0.last_updated_ms
    }

    /// The highest field id of the table.
    #[getter]
    fn last_column_id(&self) -> i32 {
        self.0.last_column_id
    }

    /// Format version 1 only: the current schema.
    #[getter]
    fn schema(&self) -> ::core::option::Option<PyIcebergSchema> {
        self.0
            .schema
            .clone()
            .into_option()
            .map(PyIcebergSchema::from)
    }

    /// The schemas of the table.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<PyIcebergSchema> {
        self.0
            .schemas
            .iter()
            .cloned()
            .map(PyIcebergSchema::from)
            .collect()
    }

    /// Id of the current schema.
    #[getter]
    fn current_schema_id(&self) -> i32 {
        self.0.current_schema_id
    }

    /// Format version 1 only: the fields of the default partition spec.
    #[getter]
    fn partition_spec(&self) -> ::std::vec::Vec<PyIcebergPartitionField> {
        self.0
            .partition_spec
            .iter()
            .cloned()
            .map(PyIcebergPartitionField::from)
            .collect()
    }

    /// The partition specs of the table.
    #[getter]
    fn partition_specs(&self) -> ::std::vec::Vec<PyIcebergPartitionSpec> {
        self.0
            .partition_specs
            .iter()
            .cloned()
            .map(PyIcebergPartitionSpec::from)
            .collect()
    }

    /// Id of the default partition spec.
    #[getter]
    fn default_spec_id(&self) -> i32 {
        self.0.default_spec_id
    }

    /// The highest partition field id of the table.
    #[getter]
    fn last_partition_id(&self) -> i32 {
        self.0.last_partition_id
    }

    /// Properties of the table.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    /// Id of the current snapshot. Absent when the table has no snapshot.
    #[getter]
    fn current_snapshot_id(&self) -> ::core::option::Option<i64> {
        self.0.current_snapshot_id
    }

    /// The snapshots of the table. With `snapshots=refs`, only those that a reference points at.
    #[getter]
    fn snapshots(&self) -> ::std::vec::Vec<PyIcebergSnapshot> {
        self.0
            .snapshots
            .iter()
            .cloned()
            .map(PyIcebergSnapshot::from)
            .collect()
    }

    /// When each snapshot became current.
    #[getter]
    fn snapshot_log(&self) -> ::std::vec::Vec<PyIcebergSnapshotLogEntry> {
        self.0
            .snapshot_log
            .iter()
            .cloned()
            .map(PyIcebergSnapshotLogEntry::from)
            .collect()
    }

    /// The previous metadata files.
    #[getter]
    fn metadata_log(&self) -> ::std::vec::Vec<PyIcebergMetadataLogEntry> {
        self.0
            .metadata_log
            .iter()
            .cloned()
            .map(PyIcebergMetadataLogEntry::from)
            .collect()
    }

    /// The sort orders of the table.
    #[getter]
    fn sort_orders(&self) -> ::std::vec::Vec<PyIcebergSortOrder> {
        self.0
            .sort_orders
            .iter()
            .cloned()
            .map(PyIcebergSortOrder::from)
            .collect()
    }

    /// Id of the default sort order.
    #[getter]
    fn default_sort_order_id(&self) -> i64 {
        self.0.default_sort_order_id
    }

    /// The branches and tags of the table, by name. Absent in format version 1.
    #[getter]
    fn refs(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, PyIcebergSnapshotReference> {
        self.0
            .refs
            .iter()
            .map(|(k, v)| (k.clone(), PyIcebergSnapshotReference::from(v.clone())))
            .collect()
    }

    /// The table statistics files.
    #[getter]
    fn statistics(&self) -> ::std::vec::Vec<PyIcebergStatisticsFile> {
        self.0
            .statistics
            .iter()
            .cloned()
            .map(PyIcebergStatisticsFile::from)
            .collect()
    }

    /// The partition statistics files.
    #[getter]
    fn partition_statistics(&self) -> ::std::vec::Vec<PyIcebergPartitionStatisticsFile> {
        self.0
            .partition_statistics
            .iter()
            .cloned()
            .map(PyIcebergPartitionStatisticsFile::from)
            .collect()
    }

    /// The next row id to assign. Format version 3 only.
    #[getter]
    fn next_row_id(&self) -> ::core::option::Option<i64> {
        self.0.next_row_id
    }

    /// The encryption keys of the table. Format version 3 only.
    #[getter]
    fn encryption_keys(&self) -> ::std::vec::Vec<PyIcebergEncryptedKey> {
        self.0
            .encryption_keys
            .iter()
            .cloned()
            .map(PyIcebergEncryptedKey::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("format_version", false),
                ("table_uuid", false),
                ("location", false),
                ("last_sequence_number", false),
                ("last_updated_ms", false),
                ("last_column_id", false),
                ("schema", false),
                ("schemas", false),
                ("current_schema_id", false),
                ("partition_spec", false),
                ("partition_specs", false),
                ("default_spec_id", false),
                ("last_partition_id", false),
                ("properties", false),
                ("current_snapshot_id", false),
                ("snapshots", false),
                ("snapshot_log", false),
                ("metadata_log", false),
                ("sort_orders", false),
                ("default_sort_order_id", false),
                ("refs", false),
                ("statistics", false),
                ("partition_statistics", false),
                ("next_row_id", false),
                ("encryption_keys", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergTableMetadata> for PyIcebergTableMetadata {
    fn from(value: super::iceberg::v1::IcebergTableMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergTableMetadata> for super::iceberg::v1::IcebergTableMetadata {
    fn from(value: PyIcebergTableMetadata) -> Self {
        value.0
    }
}

/// A condition that a table must meet for a commit to apply.
///
/// `type` (`requirement_type`) picks the condition and its field:
/// - `assert-create`: the table does not exist (no field). The commit creates a staged table.
/// - `assert-table-uuid`: `uuid`.
/// - `assert-ref-snapshot-id`: `ref` (`ref_name`) and `snapshot-id`; without `snapshot-id` the
/// reference must not exist.
/// - `assert-last-assigned-field-id`: `last-assigned-field-id`.
/// - `assert-current-schema-id`: `current-schema-id`.
/// - `assert-last-assigned-partition-id`: `last-assigned-partition-id`.
/// - `assert-default-spec-id`: `default-spec-id`.
/// - `assert-default-sort-order-id`: `default-sort-order-id`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergTableRequirement",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergTableRequirement(pub super::iceberg::v1::IcebergTableRequirement);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergTableRequirement {
    #[new]
    #[pyo3(
        signature = (
            *,
            requirement_type = None,
            uuid = None,
            ref_name = None,
            snapshot_id = None,
            last_assigned_field_id = None,
            current_schema_id = None,
            last_assigned_partition_id = None,
            default_spec_id = None,
            default_sort_order_id = None
        )
    )]
    fn new(
        requirement_type: ::core::option::Option<::std::string::String>,
        uuid: ::core::option::Option<::std::string::String>,
        ref_name: ::core::option::Option<::std::string::String>,
        snapshot_id: ::core::option::Option<i64>,
        last_assigned_field_id: ::core::option::Option<i32>,
        current_schema_id: ::core::option::Option<i32>,
        last_assigned_partition_id: ::core::option::Option<i32>,
        default_spec_id: ::core::option::Option<i32>,
        default_sort_order_id: ::core::option::Option<i64>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergTableRequirement as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = requirement_type {
            inner.requirement_type = value;
        }
        {
            let value = uuid;
            inner.uuid = value;
        }
        {
            let value = ref_name;
            inner.ref_name = value;
        }
        {
            let value = snapshot_id;
            inner.snapshot_id = value;
        }
        {
            let value = last_assigned_field_id;
            inner.last_assigned_field_id = value;
        }
        {
            let value = current_schema_id;
            inner.current_schema_id = value;
        }
        {
            let value = last_assigned_partition_id;
            inner.last_assigned_partition_id = value;
        }
        {
            let value = default_spec_id;
            inner.default_spec_id = value;
        }
        {
            let value = default_sort_order_id;
            inner.default_sort_order_id = value;
        }
        Self(inner)
    }

    /// The kind of condition (JSON `type`), such as `assert-current-schema-id`.
    #[getter]
    fn requirement_type(&self) -> ::std::string::String {
        self.0.requirement_type.clone()
    }

    /// `assert-table-uuid`: the UUID of the table.
    #[getter]
    fn uuid(&self) -> ::core::option::Option<::std::string::String> {
        self.0.uuid.clone()
    }

    /// `assert-ref-snapshot-id`: name of the branch or tag (JSON `ref`).
    #[getter]
    fn ref_name(&self) -> ::core::option::Option<::std::string::String> {
        self.0.ref_name.clone()
    }

    /// `assert-ref-snapshot-id`: the snapshot that the reference points at.
    #[getter]
    fn snapshot_id(&self) -> ::core::option::Option<i64> {
        self.0.snapshot_id
    }

    /// `assert-last-assigned-field-id`: the highest field id of the table.
    #[getter]
    fn last_assigned_field_id(&self) -> ::core::option::Option<i32> {
        self.0.last_assigned_field_id
    }

    /// `assert-current-schema-id`: id of the current schema.
    #[getter]
    fn current_schema_id(&self) -> ::core::option::Option<i32> {
        self.0.current_schema_id
    }

    /// `assert-last-assigned-partition-id`: the highest partition field id of the table.
    #[getter]
    fn last_assigned_partition_id(&self) -> ::core::option::Option<i32> {
        self.0.last_assigned_partition_id
    }

    /// `assert-default-spec-id`: id of the default partition spec.
    #[getter]
    fn default_spec_id(&self) -> ::core::option::Option<i32> {
        self.0.default_spec_id
    }

    /// `assert-default-sort-order-id`: id of the default sort order.
    #[getter]
    fn default_sort_order_id(&self) -> ::core::option::Option<i64> {
        self.0.default_sort_order_id
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("requirement_type", false),
                ("uuid", false),
                ("ref_name", false),
                ("snapshot_id", false),
                ("last_assigned_field_id", false),
                ("current_schema_id", false),
                ("last_assigned_partition_id", false),
                ("default_spec_id", false),
                ("default_sort_order_id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergTableRequirement>
    for PyIcebergTableRequirement
{
    fn from(value: super::iceberg::v1::IcebergTableRequirement) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergTableRequirement>
    for super::iceberg::v1::IcebergTableRequirement
{
    fn from(value: PyIcebergTableRequirement) -> Self {
        value.0
    }
}

/// A change to the metadata of a table.
///
/// `action` picks the change and its fields:
/// - `upgrade-format-version`: `format-version`.
/// - `assign-uuid`: `uuid`.
/// - `add-schema`: `schema`.
/// - `set-current-schema`: `schema-id` (-1 for the schema added last).
/// - `add-spec`: `spec`.
/// - `set-default-spec`: `spec-id` (-1 for the spec added last).
/// - `add-sort-order`: `sort-order`.
/// - `set-default-sort-order`: `sort-order-id` (-1 for the sort order added last).
/// - `add-snapshot`: `snapshot`.
/// - `set-snapshot-ref`: `ref-name`, `snapshot-id`, `type` (`ref_type`) and the retention fields
/// of a `branch` (`min-snapshots-to-keep`, `max-snapshot-age-ms`, `max-ref-age-ms`) or a `tag`
/// (`max-ref-age-ms`).
/// - `remove-snapshots`: `snapshot-ids`.
/// - `remove-snapshot-ref`: `ref-name`.
/// - `set-location`: `location`.
/// - `set-properties`: `updates`.
/// - `remove-properties`: `removals`.
/// - `remove-partition-specs`: `spec-ids`.
/// - `set-statistics`: `statistics`, and optionally `snapshot-id` (that of the statistics).
/// - `remove-statistics`: `snapshot-id`.
/// - `set-partition-statistics`: `partition-statistics`.
/// - `remove-partition-statistics`: `snapshot-id`.
/// - `remove-schemas`: `schema-ids`.
/// - `add-encryption-key`: `encryption-key`.
/// - `remove-encryption-key`: `key-id`.
/// The lists and the map are always sent, empty for the other actions; the server ignores the
/// keys that an action does not read.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergTableUpdate",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergTableUpdate(pub super::iceberg::v1::IcebergTableUpdate);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergTableUpdate {
    #[new]
    #[pyo3(
        signature = (
            *,
            action = None,
            format_version = None,
            uuid = None,
            schema = None,
            schema_id = None,
            spec = None,
            spec_id = None,
            sort_order = None,
            sort_order_id = None,
            snapshot = None,
            ref_name = None,
            snapshot_id = None,
            ref_type = None,
            min_snapshots_to_keep = None,
            max_snapshot_age_ms = None,
            max_ref_age_ms = None,
            snapshot_ids = None,
            location = None,
            updates = None,
            removals = None,
            spec_ids = None,
            statistics = None,
            partition_statistics = None,
            schema_ids = None,
            encryption_key = None,
            key_id = None
        )
    )]
    fn new(
        action: ::core::option::Option<::std::string::String>,
        format_version: ::core::option::Option<i32>,
        uuid: ::core::option::Option<::std::string::String>,
        schema: ::core::option::Option<PyIcebergSchema>,
        schema_id: ::core::option::Option<i32>,
        spec: ::core::option::Option<PyIcebergPartitionSpec>,
        spec_id: ::core::option::Option<i32>,
        sort_order: ::core::option::Option<PyIcebergSortOrder>,
        sort_order_id: ::core::option::Option<i64>,
        snapshot: ::core::option::Option<PyIcebergSnapshot>,
        ref_name: ::core::option::Option<::std::string::String>,
        snapshot_id: ::core::option::Option<i64>,
        ref_type: ::core::option::Option<::std::string::String>,
        min_snapshots_to_keep: ::core::option::Option<i32>,
        max_snapshot_age_ms: ::core::option::Option<i64>,
        max_ref_age_ms: ::core::option::Option<i64>,
        snapshot_ids: ::core::option::Option<::std::vec::Vec<i64>>,
        location: ::core::option::Option<::std::string::String>,
        updates: ::core::option::Option<
            ::std::collections::HashMap<::std::string::String, ::std::string::String>,
        >,
        removals: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
        spec_ids: ::core::option::Option<::std::vec::Vec<i32>>,
        statistics: ::core::option::Option<PyIcebergStatisticsFile>,
        partition_statistics: ::core::option::Option<PyIcebergPartitionStatisticsFile>,
        schema_ids: ::core::option::Option<::std::vec::Vec<i32>>,
        encryption_key: ::core::option::Option<PyIcebergEncryptedKey>,
        key_id: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner =
            <super::iceberg::v1::IcebergTableUpdate as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = action {
            inner.action = value;
        }
        {
            let value = format_version;
            inner.format_version = value;
        }
        {
            let value = uuid;
            inner.uuid = value;
        }
        {
            let value = schema;
            inner.schema = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = schema_id;
            inner.schema_id = value;
        }
        {
            let value = spec;
            inner.spec = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = spec_id;
            inner.spec_id = value;
        }
        {
            let value = sort_order;
            inner.sort_order = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = sort_order_id;
            inner.sort_order_id = value;
        }
        {
            let value = snapshot;
            inner.snapshot = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = ref_name;
            inner.ref_name = value;
        }
        {
            let value = snapshot_id;
            inner.snapshot_id = value;
        }
        {
            let value = ref_type;
            inner.ref_type = value;
        }
        {
            let value = min_snapshots_to_keep;
            inner.min_snapshots_to_keep = value;
        }
        {
            let value = max_snapshot_age_ms;
            inner.max_snapshot_age_ms = value;
        }
        {
            let value = max_ref_age_ms;
            inner.max_ref_age_ms = value;
        }
        if let ::core::option::Option::Some(value) = snapshot_ids {
            inner.snapshot_ids = value;
        }
        {
            let value = location;
            inner.location = value;
        }
        if let ::core::option::Option::Some(value) = updates {
            inner.updates = value;
        }
        if let ::core::option::Option::Some(value) = removals {
            inner.removals = value;
        }
        if let ::core::option::Option::Some(value) = spec_ids {
            inner.spec_ids = value;
        }
        {
            let value = statistics;
            inner.statistics = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = partition_statistics;
            inner.partition_statistics = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        if let ::core::option::Option::Some(value) = schema_ids {
            inner.schema_ids = value;
        }
        {
            let value = encryption_key;
            inner.encryption_key = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = key_id;
            inner.key_id = value;
        }
        Self(inner)
    }

    /// The kind of change, such as `add-snapshot`.
    #[getter]
    fn action(&self) -> ::std::string::String {
        self.0.action.clone()
    }

    /// `upgrade-format-version`: the format version to upgrade to.
    #[getter]
    fn format_version(&self) -> ::core::option::Option<i32> {
        self.0.format_version
    }

    /// `assign-uuid`: the new UUID of the table.
    #[getter]
    fn uuid(&self) -> ::core::option::Option<::std::string::String> {
        self.0.uuid.clone()
    }

    /// `add-schema`: the schema to add.
    #[getter]
    fn schema(&self) -> ::core::option::Option<PyIcebergSchema> {
        self.0
            .schema
            .clone()
            .into_option()
            .map(PyIcebergSchema::from)
    }

    /// `set-current-schema`: id of the schema to make current, or -1 for the schema added last.
    #[getter]
    fn schema_id(&self) -> ::core::option::Option<i32> {
        self.0.schema_id
    }

    /// `add-spec`: the partition spec to add.
    #[getter]
    fn spec(&self) -> ::core::option::Option<PyIcebergPartitionSpec> {
        self.0
            .spec
            .clone()
            .into_option()
            .map(PyIcebergPartitionSpec::from)
    }

    /// `set-default-spec`: id of the spec to make the default, or -1 for the spec added last.
    #[getter]
    fn spec_id(&self) -> ::core::option::Option<i32> {
        self.0.spec_id
    }

    /// `add-sort-order`: the sort order to add.
    #[getter]
    fn sort_order(&self) -> ::core::option::Option<PyIcebergSortOrder> {
        self.0
            .sort_order
            .clone()
            .into_option()
            .map(PyIcebergSortOrder::from)
    }

    /// `set-default-sort-order`: id of the sort order to make the default, or -1 for the sort order
    /// added last.
    #[getter]
    fn sort_order_id(&self) -> ::core::option::Option<i64> {
        self.0.sort_order_id
    }

    /// `add-snapshot`: the snapshot to add.
    #[getter]
    fn snapshot(&self) -> ::core::option::Option<PyIcebergSnapshot> {
        self.0
            .snapshot
            .clone()
            .into_option()
            .map(PyIcebergSnapshot::from)
    }

    /// `set-snapshot-ref` and `remove-snapshot-ref`: name of the branch or tag.
    #[getter]
    fn ref_name(&self) -> ::core::option::Option<::std::string::String> {
        self.0.ref_name.clone()
    }

    /// `set-snapshot-ref`: the snapshot that the reference points at. `set-statistics`,
    /// `remove-statistics` and `remove-partition-statistics`: the snapshot of the statistics.
    #[getter]
    fn snapshot_id(&self) -> ::core::option::Option<i64> {
        self.0.snapshot_id
    }

    /// `set-snapshot-ref`: the kind of reference (JSON `type`), `branch` or `tag`.
    #[getter]
    fn ref_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.ref_type.clone()
    }

    /// `set-snapshot-ref`, branch only: the minimum number of snapshots to keep when snapshots
    /// expire.
    #[getter]
    fn min_snapshots_to_keep(&self) -> ::core::option::Option<i32> {
        self.0.min_snapshots_to_keep
    }

    /// `set-snapshot-ref`, branch only: the maximum age of the snapshots to keep, in milliseconds.
    #[getter]
    fn max_snapshot_age_ms(&self) -> ::core::option::Option<i64> {
        self.0.max_snapshot_age_ms
    }

    /// `set-snapshot-ref`: the maximum age of the reference, in milliseconds.
    #[getter]
    fn max_ref_age_ms(&self) -> ::core::option::Option<i64> {
        self.0.max_ref_age_ms
    }

    /// `remove-snapshots`: ids of the snapshots to remove.
    #[getter]
    fn snapshot_ids(&self) -> ::std::vec::Vec<i64> {
        self.0.snapshot_ids.clone()
    }

    /// `set-location`: the new base location of the table.
    #[getter]
    fn location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.location.clone()
    }

    /// `set-properties`: the properties to set.
    #[getter]
    fn updates(&self) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.updates.clone()
    }

    /// `remove-properties`: keys of the properties to remove.
    #[getter]
    fn removals(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.removals.clone()
    }

    /// `remove-partition-specs`: ids of the partition specs to remove.
    #[getter]
    fn spec_ids(&self) -> ::std::vec::Vec<i32> {
        self.0.spec_ids.clone()
    }

    /// `set-statistics`: the statistics file to set.
    #[getter]
    fn statistics(&self) -> ::core::option::Option<PyIcebergStatisticsFile> {
        self.0
            .statistics
            .clone()
            .into_option()
            .map(PyIcebergStatisticsFile::from)
    }

    /// `set-partition-statistics`: the partition statistics file to set.
    #[getter]
    fn partition_statistics(&self) -> ::core::option::Option<PyIcebergPartitionStatisticsFile> {
        self.0
            .partition_statistics
            .clone()
            .into_option()
            .map(PyIcebergPartitionStatisticsFile::from)
    }

    /// `remove-schemas`: ids of the schemas to remove.
    #[getter]
    fn schema_ids(&self) -> ::std::vec::Vec<i32> {
        self.0.schema_ids.clone()
    }

    /// `add-encryption-key`: the encryption key to add.
    #[getter]
    fn encryption_key(&self) -> ::core::option::Option<PyIcebergEncryptedKey> {
        self.0
            .encryption_key
            .clone()
            .into_option()
            .map(PyIcebergEncryptedKey::from)
    }

    /// `remove-encryption-key`: id of the encryption key to remove.
    #[getter]
    fn key_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.key_id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("action", false),
                ("format_version", false),
                ("uuid", false),
                ("schema", false),
                ("schema_id", false),
                ("spec", false),
                ("spec_id", false),
                ("sort_order", false),
                ("sort_order_id", false),
                ("snapshot", false),
                ("ref_name", false),
                ("snapshot_id", false),
                ("ref_type", false),
                ("min_snapshots_to_keep", false),
                ("max_snapshot_age_ms", false),
                ("max_ref_age_ms", false),
                ("snapshot_ids", false),
                ("location", false),
                ("updates", false),
                ("removals", false),
                ("spec_ids", false),
                ("statistics", false),
                ("partition_statistics", false),
                ("schema_ids", false),
                ("encryption_key", false),
                ("key_id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergTableUpdate> for PyIcebergTableUpdate {
    fn from(value: super::iceberg::v1::IcebergTableUpdate) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergTableUpdate> for super::iceberg::v1::IcebergTableUpdate {
    fn from(value: PyIcebergTableUpdate) -> Self {
        value.0
    }
}

/// The changes of a namespace properties update.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "IcebergUpdateNamespacePropertiesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.iceberg",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyIcebergUpdateNamespacePropertiesResponse(
    pub super::iceberg::v1::IcebergUpdateNamespacePropertiesResponse,
);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyIcebergUpdateNamespacePropertiesResponse {
    /// Keys that were set.
    #[getter]
    fn updated(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.updated.clone()
    }

    /// Keys that were removed.
    #[getter]
    fn removed(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.removed.clone()
    }

    /// Keys to remove that the namespace did not have.
    #[getter]
    fn missing(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.missing.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("updated", false), ("removed", false), ("missing", false)],
        )
    }
}

impl ::core::convert::From<super::iceberg::v1::IcebergUpdateNamespacePropertiesResponse>
    for PyIcebergUpdateNamespacePropertiesResponse
{
    fn from(value: super::iceberg::v1::IcebergUpdateNamespacePropertiesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyIcebergUpdateNamespacePropertiesResponse>
    for super::iceberg::v1::IcebergUpdateNamespacePropertiesResponse
{
    fn from(value: PyIcebergUpdateNamespacePropertiesResponse) -> Self {
        value.0
    }
}

/// A summary of the metastore of the server.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "GetMetastoreSummaryResponse",
    eq,
    frozen,
    module = "unitycatalog._core.metastore",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyGetMetastoreSummaryResponse(pub super::metastore::v1::GetMetastoreSummaryResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyGetMetastoreSummaryResponse {
    /// Unique identifier of the metastore.
    #[getter]
    fn metastore_id(&self) -> ::std::string::String {
        self.0.metastore_id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("metastore_id", false)])
    }
}

impl ::core::convert::From<super::metastore::v1::GetMetastoreSummaryResponse>
    for PyGetMetastoreSummaryResponse
{
    fn from(value: super::metastore::v1::GetMetastoreSummaryResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyGetMetastoreSummaryResponse>
    for super::metastore::v1::GetMetastoreSummaryResponse
{
    fn from(value: PyGetMetastoreSummaryResponse) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListModelVersionsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.models",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListModelVersionsResponse(pub super::models::v1::ListModelVersionsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListModelVersionsResponse {
    /// The model versions.
    #[getter]
    fn model_versions(&self) -> ::std::vec::Vec<PyModelVersionInfo> {
        self.0
            .model_versions
            .iter()
            .cloned()
            .map(PyModelVersionInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("model_versions", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::models::v1::ListModelVersionsResponse>
    for PyListModelVersionsResponse
{
    fn from(value: super::models::v1::ListModelVersionsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListModelVersionsResponse>
    for super::models::v1::ListModelVersionsResponse
{
    fn from(value: PyListModelVersionsResponse) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListRegisteredModelsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.models",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListRegisteredModelsResponse(pub super::models::v1::ListRegisteredModelsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListRegisteredModelsResponse {
    /// The registered models.
    #[getter]
    fn registered_models(&self) -> ::std::vec::Vec<PyRegisteredModelInfo> {
        self.0
            .registered_models
            .iter()
            .cloned()
            .map(PyRegisteredModelInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("registered_models", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::models::v1::ListRegisteredModelsResponse>
    for PyListRegisteredModelsResponse
{
    fn from(value: super::models::v1::ListRegisteredModelsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListRegisteredModelsResponse>
    for super::models::v1::ListRegisteredModelsResponse
{
    fn from(value: PyListRegisteredModelsResponse) -> Self {
        value.0
    }
}

/// A version of a registered model. The server gives each new version the next number.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ModelVersionInfo",
    eq,
    frozen,
    module = "unitycatalog._core.models",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyModelVersionInfo(pub super::models::v1::ModelVersionInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyModelVersionInfo {
    /// Name of the parent registered model, relative to its schema.
    #[getter]
    fn model_name(&self) -> ::std::string::String {
        self.0.model_name.clone()
    }

    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// Name of the parent schema.
    #[getter]
    fn schema_name(&self) -> ::std::string::String {
        self.0.schema_name.clone()
    }

    /// Number of the model version.
    #[getter]
    fn version(&self) -> i64 {
        self.0.version
    }

    /// URI of the source files the model version was created from, such as an MLflow run
    /// artifact path.
    #[getter]
    fn source(&self) -> ::std::string::String {
        self.0.source.clone()
    }

    /// ID of the run of the ML package that made the model.
    #[getter]
    fn run_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.run_id.clone()
    }

    /// Status of the model version.
    #[getter]
    fn status(&self) -> ::core::option::Option<PyModelVersionStatus> {
        self.0
            .status
            .as_known()
            .filter(|e| {
                <super::models::v1::ModelVersionStatus as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PyModelVersionStatus::from)
    }

    /// Storage location of the files of the model version.
    #[getter]
    fn storage_location(&self) -> ::std::string::String {
        self.0.storage_location.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Time at which the model version was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the model version.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the model version was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the model version.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the model version.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("model_name", false),
                ("catalog_name", false),
                ("schema_name", false),
                ("version", false),
                ("source", false),
                ("run_id", false),
                ("status", false),
                ("storage_location", false),
                ("comment", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::models::v1::ModelVersionInfo> for PyModelVersionInfo {
    fn from(value: super::models::v1::ModelVersionInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyModelVersionInfo> for super::models::v1::ModelVersionInfo {
    fn from(value: PyModelVersionInfo) -> Self {
        value.0
    }
}

/// A registered model: a securable in the three-level namespace (catalog.schema.model) that
/// holds model versions.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "RegisteredModelInfo",
    eq,
    frozen,
    module = "unitycatalog._core.models",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyRegisteredModelInfo(pub super::models::v1::RegisteredModelInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyRegisteredModelInfo {
    /// Name of the registered model, relative to its schema.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// Name of the parent schema.
    #[getter]
    fn schema_name(&self) -> ::std::string::String {
        self.0.schema_name.clone()
    }

    /// Storage location under which the model versions keep their files.
    #[getter]
    fn storage_location(&self) -> ::std::string::String {
        self.0.storage_location.clone()
    }

    /// Three-level name of the registered model: `catalog.schema.model`.
    #[getter]
    fn full_name(&self) -> ::std::string::String {
        self.0.full_name.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Username of the owner of the registered model.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the registered model was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the registered model.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the registered model was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the registered model.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the registered model.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("catalog_name", false),
                ("schema_name", false),
                ("storage_location", false),
                ("full_name", false),
                ("comment", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::models::v1::RegisteredModelInfo> for PyRegisteredModelInfo {
    fn from(value: super::models::v1::RegisteredModelInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyRegisteredModelInfo> for super::models::v1::RegisteredModelInfo {
    fn from(value: PyRegisteredModelInfo) -> Self {
        value.0
    }
}

/// A change to the privileges of one principal.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "PermissionsChange",
    eq,
    frozen,
    module = "unitycatalog._core.permissions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPermissionsChange(pub super::permissions::v1::PermissionsChange);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyPermissionsChange {
    #[new]
    #[pyo3(signature = (principal, *, add = None, remove = None))]
    fn new(
        principal: ::std::string::String,
        add: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
        remove: ::core::option::Option<::std::vec::Vec<::std::string::String>>,
    ) -> Self {
        let mut inner =
            <super::permissions::v1::PermissionsChange as ::core::default::Default>::default();
        {
            let value = principal;
            inner.principal = value;
        }
        if let ::core::option::Option::Some(value) = add {
            inner.add = value;
        }
        if let ::core::option::Option::Some(value) = remove {
            inner.remove = value;
        }
        Self(inner)
    }

    /// The principal whose privileges change: the email address of a user.
    #[getter]
    fn principal(&self) -> ::std::string::String {
        self.0.principal.clone()
    }

    /// The privileges to grant. The values are those of `PrivilegeAssignment.privileges`.
    #[getter]
    fn add(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.add.clone()
    }

    /// The privileges to revoke. The values are those of `PrivilegeAssignment.privileges`.
    #[getter]
    fn remove(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.remove.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("principal", false), ("add", false), ("remove", false)],
        )
    }
}

impl ::core::convert::From<super::permissions::v1::PermissionsChange> for PyPermissionsChange {
    fn from(value: super::permissions::v1::PermissionsChange) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyPermissionsChange> for super::permissions::v1::PermissionsChange {
    fn from(value: PyPermissionsChange) -> Self {
        value.0
    }
}

/// The privileges on a securable, per principal.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "PermissionsList",
    eq,
    frozen,
    module = "unitycatalog._core.permissions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPermissionsList(pub super::permissions::v1::PermissionsList);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyPermissionsList {
    /// The privileges of each principal.
    #[getter]
    fn privilege_assignments(&self) -> ::std::vec::Vec<PyPrivilegeAssignment> {
        self.0
            .privilege_assignments
            .iter()
            .cloned()
            .map(PyPrivilegeAssignment::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("privilege_assignments", false)])
    }
}

impl ::core::convert::From<super::permissions::v1::PermissionsList> for PyPermissionsList {
    fn from(value: super::permissions::v1::PermissionsList) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyPermissionsList> for super::permissions::v1::PermissionsList {
    fn from(value: PyPermissionsList) -> Self {
        value.0
    }
}

/// The privileges of one principal on a securable.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "PrivilegeAssignment",
    eq,
    frozen,
    module = "unitycatalog._core.permissions",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPrivilegeAssignment(pub super::permissions::v1::PrivilegeAssignment);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyPrivilegeAssignment {
    /// The principal: the email address of a user.
    #[getter]
    fn principal(&self) -> ::std::string::String {
        self.0.principal.clone()
    }

    /// The privileges of the principal. Values: `CREATE CATALOG`, `USE CATALOG`, `CREATE SCHEMA`,
    /// `USE SCHEMA`, `CREATE TABLE`, `SELECT`, `MODIFY`, `CREATE FUNCTION`, `EXECUTE`,
    /// `CREATE VOLUME`, `READ VOLUME`, `CREATE MODEL`, `CREATE EXTERNAL LOCATION`, `READ FILES`,
    /// `WRITE FILES`, `CREATE EXTERNAL TABLE`, `CREATE EXTERNAL VOLUME`, `CREATE MANAGED STORAGE`,
    /// `CREATE STORAGE CREDENTIAL`.
    #[getter]
    fn privileges(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.privileges.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("principal", false), ("privileges", false)])
    }
}

impl ::core::convert::From<super::permissions::v1::PrivilegeAssignment> for PyPrivilegeAssignment {
    fn from(value: super::permissions::v1::PrivilegeAssignment) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyPrivilegeAssignment> for super::permissions::v1::PrivilegeAssignment {
    fn from(value: PyPrivilegeAssignment) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListRecipientsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.recipients",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListRecipientsResponse(pub super::recipients::v1::ListRecipientsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListRecipientsResponse {
    /// The recipients.
    #[getter]
    fn items(&self) -> ::std::vec::Vec<PyRecipientInfo> {
        self.0
            .items
            .iter()
            .cloned()
            .map(PyRecipientInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("items", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::recipients::v1::ListRecipientsResponse>
    for PyListRecipientsResponse
{
    fn from(value: super::recipients::v1::ListRecipientsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListRecipientsResponse>
    for super::recipients::v1::ListRecipientsResponse
{
    fn from(value: PyListRecipientsResponse) -> Self {
        value.0
    }
}

/// A Delta Sharing recipient: an organization or user outside Unity Catalog that reads shares with
/// a bearer token. The server keeps a hash of the token only.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "RecipientInfo",
    eq,
    frozen,
    module = "unitycatalog._core.recipients",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyRecipientInfo(pub super::recipients::v1::RecipientInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyRecipientInfo {
    /// Name of the recipient.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Unique identifier of the recipient.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// How the recipient signs in.
    #[getter]
    fn authentication_type(&self) -> ::core::option::Option<PyAuthenticationType> {
        self.0
            .authentication_type
            .as_known()
            .filter(|e| {
                <super::recipients::v1::AuthenticationType as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PyAuthenticationType::from)
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Username of the owner of the recipient.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the recipient was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the recipient.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// The bearer token. Set only in the responses of `CreateRecipient` and `RotateRecipientToken`.
    #[getter]
    fn bearer_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.bearer_token.clone()
    }

    /// The first 8 characters of the bearer token, for display only.
    #[getter]
    fn token_prefix(&self) -> ::std::string::String {
        self.0.token_prefix.clone()
    }

    /// Names of the shares that the recipient can read.
    #[getter]
    fn shares(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.shares.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("id", false),
                ("authentication_type", false),
                ("comment", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("bearer_token", true),
                ("token_prefix", false),
                ("shares", false),
            ],
        )
    }
}

impl ::core::convert::From<super::recipients::v1::RecipientInfo> for PyRecipientInfo {
    fn from(value: super::recipients::v1::RecipientInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyRecipientInfo> for super::recipients::v1::RecipientInfo {
    fn from(value: PyRecipientInfo) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListSchemasResponse",
    eq,
    frozen,
    module = "unitycatalog._core.schemas",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListSchemasResponse(pub super::schemas::v1::ListSchemasResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListSchemasResponse {
    /// The schemas.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<PySchemaInfo> {
        self.0
            .schemas
            .iter()
            .cloned()
            .map(PySchemaInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("schemas", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::schemas::v1::ListSchemasResponse> for PyListSchemasResponse {
    fn from(value: super::schemas::v1::ListSchemasResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListSchemasResponse> for super::schemas::v1::ListSchemasResponse {
    fn from(value: PyListSchemasResponse) -> Self {
        value.0
    }
}

/// A schema: the second layer of the three-level namespace. It holds tables, volumes, functions
/// and models.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SchemaInfo",
    eq,
    frozen,
    module = "unitycatalog._core.schemas",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySchemaInfo(pub super::schemas::v1::SchemaInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySchemaInfo {
    /// Name of the schema, relative to the parent catalog.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Key-value properties of the schema.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    /// Full name of the schema, in the form `<catalog>.<schema>`.
    #[getter]
    fn full_name(&self) -> ::std::string::String {
        self.0.full_name.clone()
    }

    /// Username of the owner of the schema.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the schema was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the schema.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the schema was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the schema.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the schema.
    #[getter]
    fn schema_id(&self) -> ::std::string::String {
        self.0.schema_id.clone()
    }

    /// Storage root URL for the managed storage of the schema, such as `s3://bucket/ucroot`.
    /// Without it, managed tables and volumes of the schema go below the storage of the catalog.
    #[getter]
    fn storage_root(&self) -> ::core::option::Option<::std::string::String> {
        self.0.storage_root.clone()
    }

    /// Storage location URL (full path) of the managed storage of the schema.
    #[getter]
    fn storage_location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.storage_location.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("catalog_name", false),
                ("comment", false),
                ("properties", false),
                ("full_name", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("schema_id", false),
                ("storage_root", false),
                ("storage_location", false),
            ],
        )
    }
}

impl ::core::convert::From<super::schemas::v1::SchemaInfo> for PySchemaInfo {
    fn from(value: super::schemas::v1::SchemaInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySchemaInfo> for super::schemas::v1::SchemaInfo {
    fn from(value: PySchemaInfo) -> Self {
        value.0
    }
}

/// A page of SCIM groups.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListScimGroupsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListScimGroupsResponse(pub super::scim::v1::ListScimGroupsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListScimGroupsResponse {
    /// SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// Number of groups in the page.
    #[getter]
    fn total_results(&self) -> i32 {
        self.0.total_results
    }

    /// The groups.
    #[getter]
    fn resources(&self) -> ::std::vec::Vec<PyScimGroup> {
        self.0
            .resources
            .iter()
            .cloned()
            .map(PyScimGroup::from)
            .collect()
    }

    /// Index of the first group of the page, from 1.
    #[getter]
    fn start_index(&self) -> i32 {
        self.0.start_index
    }

    /// Number of groups in the page.
    #[getter]
    fn items_per_page(&self) -> i32 {
        self.0.items_per_page
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("total_results", false),
                ("resources", false),
                ("start_index", false),
                ("items_per_page", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ListScimGroupsResponse> for PyListScimGroupsResponse {
    fn from(value: super::scim::v1::ListScimGroupsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListScimGroupsResponse> for super::scim::v1::ListScimGroupsResponse {
    fn from(value: PyListScimGroupsResponse) -> Self {
        value.0
    }
}

/// The SCIM resource types of the server, User and Group, as a SCIM list (RFC 7644, section 4).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListScimResourceTypesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListScimResourceTypesResponse(pub super::scim::v1::ListScimResourceTypesResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListScimResourceTypesResponse {
    /// SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// Number of resource types.
    #[getter]
    fn total_results(&self) -> i32 {
        self.0.total_results
    }

    /// The resource types.
    #[getter]
    fn resources(&self) -> ::std::vec::Vec<PyScimResourceType> {
        self.0
            .resources
            .iter()
            .cloned()
            .map(PyScimResourceType::from)
            .collect()
    }

    /// Index of the first resource type: 1.
    #[getter]
    fn start_index(&self) -> i32 {
        self.0.start_index
    }

    /// Number of resource types.
    #[getter]
    fn items_per_page(&self) -> i32 {
        self.0.items_per_page
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("total_results", false),
                ("resources", false),
                ("start_index", false),
                ("items_per_page", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ListScimResourceTypesResponse>
    for PyListScimResourceTypesResponse
{
    fn from(value: super::scim::v1::ListScimResourceTypesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListScimResourceTypesResponse>
    for super::scim::v1::ListScimResourceTypesResponse
{
    fn from(value: PyListScimResourceTypesResponse) -> Self {
        value.0
    }
}

/// The SCIM schemas of the server, User and Group, as a SCIM list (RFC 7644, section 4).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListScimSchemasResponse",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListScimSchemasResponse(pub super::scim::v1::ListScimSchemasResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListScimSchemasResponse {
    /// SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// Number of schemas.
    #[getter]
    fn total_results(&self) -> i32 {
        self.0.total_results
    }

    /// The schemas.
    #[getter]
    fn resources(&self) -> ::std::vec::Vec<PyScimSchema> {
        self.0
            .resources
            .iter()
            .cloned()
            .map(PyScimSchema::from)
            .collect()
    }

    /// Index of the first schema: 1.
    #[getter]
    fn start_index(&self) -> i32 {
        self.0.start_index
    }

    /// Number of schemas.
    #[getter]
    fn items_per_page(&self) -> i32 {
        self.0.items_per_page
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("total_results", false),
                ("resources", false),
                ("start_index", false),
                ("items_per_page", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ListScimSchemasResponse> for PyListScimSchemasResponse {
    fn from(value: super::scim::v1::ListScimSchemasResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListScimSchemasResponse> for super::scim::v1::ListScimSchemasResponse {
    fn from(value: PyListScimSchemasResponse) -> Self {
        value.0
    }
}

/// A page of SCIM users.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListScimUsersResponse",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListScimUsersResponse(pub super::scim::v1::ListScimUsersResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListScimUsersResponse {
    /// SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:ListResponse`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// Number of users in the page.
    #[getter]
    fn total_results(&self) -> i32 {
        self.0.total_results
    }

    /// The users.
    #[getter]
    fn resources(&self) -> ::std::vec::Vec<PyScimUser> {
        self.0
            .resources
            .iter()
            .cloned()
            .map(PyScimUser::from)
            .collect()
    }

    /// Index of the first user of the page, from 1.
    #[getter]
    fn start_index(&self) -> i32 {
        self.0.start_index
    }

    /// Number of users in the page.
    #[getter]
    fn items_per_page(&self) -> i32 {
        self.0.items_per_page
    }

    /// Time of the list.
    #[getter]
    fn meta(&self) -> ::core::option::Option<PyScimMeta> {
        self.0.meta.clone().into_option().map(PyScimMeta::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("total_results", false),
                ("resources", false),
                ("start_index", false),
                ("items_per_page", false),
                ("meta", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ListScimUsersResponse> for PyListScimUsersResponse {
    fn from(value: super::scim::v1::ListScimUsersResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListScimUsersResponse> for super::scim::v1::ListScimUsersResponse {
    fn from(value: PyListScimUsersResponse) -> Self {
        value.0
    }
}

/// A way to authenticate to the SCIM endpoints.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimAuthenticationScheme",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimAuthenticationScheme(pub super::scim::v1::ScimAuthenticationScheme);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimAuthenticationScheme {
    /// Kind of the scheme: `oauthbearertoken`.
    #[getter]
    fn get_type(&self) -> ::std::string::String {
        self.0.r#type.clone()
    }

    /// Name of the scheme.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Description of the scheme.
    #[getter]
    fn description(&self) -> ::std::string::String {
        self.0.description.clone()
    }

    /// Whether this is the preferred scheme.
    #[getter]
    fn primary(&self) -> bool {
        self.0.primary
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("type", false),
                ("name", false),
                ("description", false),
                ("primary", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimAuthenticationScheme>
    for PyScimAuthenticationScheme
{
    fn from(value: super::scim::v1::ScimAuthenticationScheme) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimAuthenticationScheme>
    for super::scim::v1::ScimAuthenticationScheme
{
    fn from(value: PyScimAuthenticationScheme) -> Self {
        value.0
    }
}

/// How the server supports SCIM bulk requests.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimBulkConfig",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimBulkConfig(pub super::scim::v1::ScimBulkConfig);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimBulkConfig {
    /// Whether bulk requests are supported.
    #[getter]
    fn supported(&self) -> bool {
        self.0.supported
    }

    /// The maximum number of operations in a bulk request.
    #[getter]
    fn max_operations(&self) -> i32 {
        self.0.max_operations
    }

    /// The maximum size of a bulk request, in bytes.
    #[getter]
    fn max_payload_size(&self) -> i32 {
        self.0.max_payload_size
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("supported", false),
                ("max_operations", false),
                ("max_payload_size", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimBulkConfig> for PyScimBulkConfig {
    fn from(value: super::scim::v1::ScimBulkConfig) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimBulkConfig> for super::scim::v1::ScimBulkConfig {
    fn from(value: PyScimBulkConfig) -> Self {
        value.0
    }
}

/// One operation of a SCIM bulk request (RFC 7644, section 3.7).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimBulkOp",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimBulkOp(pub super::scim::v1::ScimBulkOp);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimBulkOp {
    #[new]
    #[pyo3(signature = (method, path, *, bulk_id = None, data = None))]
    fn new(
        method: ::std::string::String,
        path: ::std::string::String,
        bulk_id: ::core::option::Option<::std::string::String>,
        data: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
    ) -> ::pyo3::PyResult<Self> {
        let mut inner = <super::scim::v1::ScimBulkOp as ::core::default::Default>::default();
        {
            let value = method;
            inner.method = value;
        }
        {
            let value = path;
            inner.path = value;
        }
        {
            let value = bulk_id;
            inner.bulk_id = value;
        }
        if let ::core::option::Option::Some(value) = data {
            inner.data = ::buffa::MessageField::some(json_struct_from_py(&value)?);
        }
        ::core::result::Result::Ok(Self(inner))
    }

    /// The HTTP method of the operation: `POST`, `PATCH` or `DELETE`.
    #[getter]
    fn method(&self) -> ::std::string::String {
        self.0.method.clone()
    }

    /// The resource of the operation: `/Users` or `/Groups` for `POST`, `/Users/<id>` or
    /// `/Groups/<id>` for `PATCH` and `DELETE`.
    #[getter]
    fn path(&self) -> ::std::string::String {
        self.0.path.clone()
    }

    /// Identifier of the operation, which the result repeats.
    #[getter]
    fn bulk_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.bulk_id.clone()
    }

    /// The body of the operation: a `ScimUser` or `ScimGroup` for `POST`, a `ScimPatchOp` (without
    /// `id`) for `PATCH`.
    #[gen_stub(
        override_return_type(
            type_repr = "typing.Optional[dict[str, typing.Any]]",
            imports = ("typing")
        )
    )]
    #[getter]
    fn data(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .data
            .as_option()
            .map(|value| json_struct_to_py(py, value))
            .transpose()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("method", false),
                ("path", false),
                ("bulk_id", false),
                ("data", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimBulkOp> for PyScimBulkOp {
    fn from(value: super::scim::v1::ScimBulkOp) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimBulkOp> for super::scim::v1::ScimBulkOp {
    fn from(value: PyScimBulkOp) -> Self {
        value.0
    }
}

/// The result of one operation of a SCIM bulk request.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimBulkOpResult",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimBulkOpResult(pub super::scim::v1::ScimBulkOpResult);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimBulkOpResult {
    /// The HTTP method of the operation.
    #[getter]
    fn method(&self) -> ::std::string::String {
        self.0.method.clone()
    }

    /// The HTTP status of the operation, such as `201`.
    #[getter]
    fn status(&self) -> ::std::string::String {
        self.0.status.clone()
    }

    /// The `bulk_id` of the operation.
    #[getter]
    fn bulk_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.bulk_id.clone()
    }

    /// Path of the resource that the operation created or changed, such as `/scim2/Users/<id>`.
    #[getter]
    fn location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.location.clone()
    }

    /// The answer of the operation: the user or group, or an error with `status` and `detail`.
    #[gen_stub(
        override_return_type(
            type_repr = "typing.Optional[dict[str, typing.Any]]",
            imports = ("typing")
        )
    )]
    #[getter]
    fn response(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .response
            .as_option()
            .map(|value| json_struct_to_py(py, value))
            .transpose()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("method", false),
                ("status", false),
                ("bulk_id", false),
                ("location", false),
                ("response", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimBulkOpResult> for PyScimBulkOpResult {
    fn from(value: super::scim::v1::ScimBulkOpResult) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimBulkOpResult> for super::scim::v1::ScimBulkOpResult {
    fn from(value: PyScimBulkOpResult) -> Self {
        value.0
    }
}

/// The answer to a SCIM bulk request: one result per operation that ran, in order.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimBulkResponse",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimBulkResponse(pub super::scim::v1::ScimBulkResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimBulkResponse {
    /// SCIM schema URIs of the response: `urn:ietf:params:scim:api:messages:2.0:BulkResponse`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// The results of the operations.
    #[getter]
    fn operations(&self) -> ::std::vec::Vec<PyScimBulkOpResult> {
        self.0
            .operations
            .iter()
            .cloned()
            .map(PyScimBulkOpResult::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("schemas", false), ("operations", false)])
    }
}

impl ::core::convert::From<super::scim::v1::ScimBulkResponse> for PyScimBulkResponse {
    fn from(value: super::scim::v1::ScimBulkResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimBulkResponse> for super::scim::v1::ScimBulkResponse {
    fn from(value: PyScimBulkResponse) -> Self {
        value.0
    }
}

/// An email address of a user.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimEmail",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimEmail(pub super::scim::v1::ScimEmail);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimEmail {
    #[new]
    #[pyo3(signature = (value, *, primary = None, email_type = None))]
    fn new(
        value: ::std::string::String,
        primary: ::core::option::Option<bool>,
        email_type: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner = <super::scim::v1::ScimEmail as ::core::default::Default>::default();
        {
            inner.value = value;
        }
        {
            let value = primary;
            inner.primary = value;
        }
        {
            let value = email_type;
            inner.email_type = value;
        }
        Self(inner)
    }

    /// The email address.
    #[getter]
    fn value(&self) -> ::std::string::String {
        self.0.value.clone()
    }

    /// Whether this is the primary email address of the user.
    #[getter]
    fn primary(&self) -> ::core::option::Option<bool> {
        self.0.primary
    }

    /// Kind of the address, such as `work`. The server does not keep it.
    #[getter]
    fn email_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.email_type.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("value", false), ("primary", false), ("email_type", false)],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimEmail> for PyScimEmail {
    fn from(value: super::scim::v1::ScimEmail) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimEmail> for super::scim::v1::ScimEmail {
    fn from(value: PyScimEmail) -> Self {
        value.0
    }
}

/// How the server supports SCIM filters.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimFilterConfig",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimFilterConfig(pub super::scim::v1::ScimFilterConfig);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimFilterConfig {
    /// Whether filters are supported.
    #[getter]
    fn supported(&self) -> bool {
        self.0.supported
    }

    /// The maximum number of resources in an answer.
    #[getter]
    fn max_results(&self) -> i32 {
        self.0.max_results
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("supported", false), ("max_results", false)],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimFilterConfig> for PyScimFilterConfig {
    fn from(value: super::scim::v1::ScimFilterConfig) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimFilterConfig> for super::scim::v1::ScimFilterConfig {
    fn from(value: PyScimFilterConfig) -> Self {
        value.0
    }
}

/// A SCIM 2.0 group of users.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimGroup",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimGroup(pub super::scim::v1::ScimGroup);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimGroup {
    /// SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Group`. Requests
    /// send the key even when empty; the server fills it in.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// Unique identifier of the group. The server assigns it.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Name of the group.
    #[getter]
    fn display_name(&self) -> ::std::string::String {
        self.0.display_name.clone()
    }

    /// Members of the group.
    #[getter]
    fn members(&self) -> ::std::vec::Vec<PyScimGroupMember> {
        self.0
            .members
            .iter()
            .cloned()
            .map(PyScimGroupMember::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("id", false),
                ("display_name", false),
                ("members", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimGroup> for PyScimGroup {
    fn from(value: super::scim::v1::ScimGroup) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimGroup> for super::scim::v1::ScimGroup {
    fn from(value: PyScimGroup) -> Self {
        value.0
    }
}

/// A member of a SCIM group.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimGroupMember",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimGroupMember(pub super::scim::v1::ScimGroupMember);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimGroupMember {
    #[new]
    #[pyo3(signature = (value, *, display = None, member_type = None))]
    fn new(
        value: ::std::string::String,
        display: ::core::option::Option<::std::string::String>,
        member_type: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner = <super::scim::v1::ScimGroupMember as ::core::default::Default>::default();
        {
            inner.value = value;
        }
        {
            let value = display;
            inner.display = value;
        }
        {
            let value = member_type;
            inner.member_type = value;
        }
        Self(inner)
    }

    /// The `id` of the user.
    #[getter]
    fn value(&self) -> ::std::string::String {
        self.0.value.clone()
    }

    /// A display string, which clients set to the name of the user.
    #[getter]
    fn display(&self) -> ::core::option::Option<::std::string::String> {
        self.0.display.clone()
    }

    /// The SCIM resource type of the member: `User`.
    #[getter]
    fn member_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.member_type.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("value", false), ("display", false), ("member_type", false)],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimGroupMember> for PyScimGroupMember {
    fn from(value: super::scim::v1::ScimGroupMember) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimGroupMember> for super::scim::v1::ScimGroupMember {
    fn from(value: PyScimGroupMember) -> Self {
        value.0
    }
}

/// The SCIM `meta` of a resource: its type and when it was created and last changed.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimMeta",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimMeta(pub super::scim::v1::ScimMeta);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimMeta {
    #[new]
    #[pyo3(
        signature = (
            *,
            resource_type = None,
            created = None,
            last_modified = None,
            location = None
        )
    )]
    fn new(
        resource_type: ::core::option::Option<::std::string::String>,
        created: ::core::option::Option<::std::string::String>,
        last_modified: ::core::option::Option<::std::string::String>,
        location: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner = <super::scim::v1::ScimMeta as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = resource_type {
            inner.resource_type = value;
        }
        {
            let value = created;
            inner.created = value;
        }
        {
            let value = last_modified;
            inner.last_modified = value;
        }
        {
            let value = location;
            inner.location = value;
        }
        Self(inner)
    }

    /// The SCIM resource type, such as `User`.
    #[getter]
    fn resource_type(&self) -> ::std::string::String {
        self.0.resource_type.clone()
    }

    /// Time at which the resource was created, in RFC 3339.
    #[getter]
    fn created(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created.clone()
    }

    /// Time at which the resource was last changed, in RFC 3339.
    #[getter]
    fn last_modified(&self) -> ::core::option::Option<::std::string::String> {
        self.0.last_modified.clone()
    }

    /// Path of the resource, such as `/scim2/ResourceTypes/User`. Only the discovery resources have
    /// it.
    #[getter]
    fn location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.location.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("resource_type", false),
                ("created", false),
                ("last_modified", false),
                ("location", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimMeta> for PyScimMeta {
    fn from(value: super::scim::v1::ScimMeta) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimMeta> for super::scim::v1::ScimMeta {
    fn from(value: PyScimMeta) -> Self {
        value.0
    }
}

/// One operation of a SCIM PATCH request (RFC 7644, section 3.5.2).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimPatchOperation",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimPatchOperation(pub super::scim::v1::ScimPatchOperation);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimPatchOperation {
    #[new]
    #[pyo3(signature = (op, *, path = None, value = None))]
    fn new(
        op: ::std::string::String,
        path: ::core::option::Option<::std::string::String>,
        value: ::core::option::Option<::pyo3::Bound<'_, ::pyo3::PyAny>>,
    ) -> ::pyo3::PyResult<Self> {
        let mut inner =
            <super::scim::v1::ScimPatchOperation as ::core::default::Default>::default();
        {
            let value = op;
            inner.op = value;
        }
        {
            let value = path;
            inner.path = value;
        }
        if let ::core::option::Option::Some(value) = value {
            inner.value = ::buffa::MessageField::some(json_value_from_py(&value)?);
        }
        ::core::result::Result::Ok(Self(inner))
    }

    /// The operation: `add`, `remove` or `replace`, in any case.
    #[getter]
    fn op(&self) -> ::std::string::String {
        self.0.op.clone()
    }

    /// SCIM attribute path, such as `active`, `displayName`, `members` or
    /// `members[value eq "<id>"]`. Absent for a `replace` whose `value` holds the attributes.
    #[getter]
    fn path(&self) -> ::core::option::Option<::std::string::String> {
        self.0.path.clone()
    }

    /// The new value. Any JSON: a boolean or string for `active`, a string for `displayName`, an
    /// array of members for `members`, an object of attributes for a `replace` without `path`.
    #[gen_stub(override_return_type(type_repr = "typing.Any", imports = ("typing")))]
    #[getter]
    fn value(
        &self,
        py: ::pyo3::Python<'_>,
    ) -> ::pyo3::PyResult<::core::option::Option<::pyo3::Py<::pyo3::PyAny>>> {
        self.0
            .value
            .as_option()
            .map(|value| json_value_to_py(py, value))
            .transpose()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("op", false), ("path", false), ("value", false)],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimPatchOperation> for PyScimPatchOperation {
    fn from(value: super::scim::v1::ScimPatchOperation) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimPatchOperation> for super::scim::v1::ScimPatchOperation {
    fn from(value: PyScimPatchOperation) -> Self {
        value.0
    }
}

/// A picture of a user.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimPhoto",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimPhoto(pub super::scim::v1::ScimPhoto);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimPhoto {
    #[new]
    #[pyo3(signature = (value, *, photo_type = None))]
    fn new(
        value: ::std::string::String,
        photo_type: ::core::option::Option<::std::string::String>,
    ) -> Self {
        let mut inner = <super::scim::v1::ScimPhoto as ::core::default::Default>::default();
        {
            inner.value = value;
        }
        {
            let value = photo_type;
            inner.photo_type = value;
        }
        Self(inner)
    }

    /// URL of the picture. Empty when the user has no picture.
    #[getter]
    fn value(&self) -> ::std::string::String {
        self.0.value.clone()
    }

    /// Kind of the picture, such as `photo`. The server does not keep it.
    #[getter]
    fn photo_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.photo_type.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("value", false), ("photo_type", false)])
    }
}

impl ::core::convert::From<super::scim::v1::ScimPhoto> for PyScimPhoto {
    fn from(value: super::scim::v1::ScimPhoto) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimPhoto> for super::scim::v1::ScimPhoto {
    fn from(value: PyScimPhoto) -> Self {
        value.0
    }
}

/// A SCIM resource type (RFC 7643, section 6).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimResourceType",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimResourceType(pub super::scim::v1::ScimResourceType);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimResourceType {
    /// SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:ResourceType`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// Identifier of the resource type, such as `User`.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Name of the resource type, such as `User`.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Path of the resources, relative to `/scim2`, such as `/Users`.
    #[getter]
    fn endpoint(&self) -> ::std::string::String {
        self.0.endpoint.clone()
    }

    /// Description of the resource type.
    #[getter]
    fn description(&self) -> ::std::string::String {
        self.0.description.clone()
    }

    /// URI of the schema of the resources, such as `urn:ietf:params:scim:schemas:core:2.0:User`.
    #[getter]
    fn schema(&self) -> ::std::string::String {
        self.0.schema.clone()
    }

    /// The type and path of the resource type.
    #[getter]
    fn meta(&self) -> ::core::option::Option<PyScimMeta> {
        self.0.meta.clone().into_option().map(PyScimMeta::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("id", false),
                ("name", false),
                ("endpoint", false),
                ("description", false),
                ("schema", false),
                ("meta", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimResourceType> for PyScimResourceType {
    fn from(value: super::scim::v1::ScimResourceType) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimResourceType> for super::scim::v1::ScimResourceType {
    fn from(value: PyScimResourceType) -> Self {
        value.0
    }
}

/// A SCIM role of a user.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimRole",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimRole(pub super::scim::v1::ScimRole);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimRole {
    #[new]
    #[pyo3(signature = (*, value = None))]
    fn new(value: ::core::option::Option<::std::string::String>) -> Self {
        let mut inner = <super::scim::v1::ScimRole as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = value {
            inner.value = value;
        }
        Self(inner)
    }

    /// The role, such as `admin`.
    #[getter]
    fn value(&self) -> ::std::string::String {
        self.0.value.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("value", false)])
    }
}

impl ::core::convert::From<super::scim::v1::ScimRole> for PyScimRole {
    fn from(value: super::scim::v1::ScimRole) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimRole> for super::scim::v1::ScimRole {
    fn from(value: PyScimRole) -> Self {
        value.0
    }
}

/// A SCIM schema (RFC 7643, section 7).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimSchema",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimSchema(pub super::scim::v1::ScimSchema);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimSchema {
    /// SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:Schema`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// URI of the schema, such as `urn:ietf:params:scim:schemas:core:2.0:User`.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Name of the schema, such as `User`.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Description of the schema.
    #[getter]
    fn description(&self) -> ::std::string::String {
        self.0.description.clone()
    }

    /// The attributes of the schema.
    #[getter]
    fn attributes(&self) -> ::std::vec::Vec<PyScimSchemaAttribute> {
        self.0
            .attributes
            .iter()
            .cloned()
            .map(PyScimSchemaAttribute::from)
            .collect()
    }

    /// The type and path of the schema.
    #[getter]
    fn meta(&self) -> ::core::option::Option<PyScimMeta> {
        self.0.meta.clone().into_option().map(PyScimMeta::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("id", false),
                ("name", false),
                ("description", false),
                ("attributes", false),
                ("meta", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimSchema> for PyScimSchema {
    fn from(value: super::scim::v1::ScimSchema) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimSchema> for super::scim::v1::ScimSchema {
    fn from(value: PyScimSchema) -> Self {
        value.0
    }
}

/// An attribute of a SCIM schema (RFC 7643, section 7).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimSchemaAttribute",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimSchemaAttribute(pub super::scim::v1::ScimSchemaAttribute);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimSchemaAttribute {
    /// Name of the attribute, such as `userName`.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Data type of the attribute: `string`, `boolean` or `complex`.
    #[getter]
    fn get_type(&self) -> ::std::string::String {
        self.0.r#type.clone()
    }

    /// Whether a resource must have the attribute.
    #[getter]
    fn required(&self) -> bool {
        self.0.required
    }

    /// Whether the attribute holds a list of values. Absent: a single value.
    #[getter]
    fn multi_valued(&self) -> ::core::option::Option<bool> {
        self.0.multi_valued
    }

    /// How the server keeps values unique: `server`. Absent: not unique.
    #[getter]
    fn uniqueness(&self) -> ::core::option::Option<::std::string::String> {
        self.0.uniqueness.clone()
    }

    /// The attributes of a `complex` attribute.
    #[getter]
    fn sub_attributes(&self) -> ::std::vec::Vec<PyScimSchemaAttribute> {
        self.0
            .sub_attributes
            .iter()
            .cloned()
            .map(PyScimSchemaAttribute::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("type", false),
                ("required", false),
                ("multi_valued", false),
                ("uniqueness", false),
                ("sub_attributes", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimSchemaAttribute> for PyScimSchemaAttribute {
    fn from(value: super::scim::v1::ScimSchemaAttribute) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimSchemaAttribute> for super::scim::v1::ScimSchemaAttribute {
    fn from(value: PyScimSchemaAttribute) -> Self {
        value.0
    }
}

/// The SCIM features that the server supports (RFC 7643, section 5).
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimServiceProviderConfig",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimServiceProviderConfig(pub super::scim::v1::ScimServiceProviderConfig);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimServiceProviderConfig {
    /// SCIM schema URIs of the resource:
    /// `urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// PATCH requests.
    #[getter]
    fn patch(&self) -> ::core::option::Option<PyScimSupported> {
        self.0
            .patch
            .clone()
            .into_option()
            .map(PyScimSupported::from)
    }

    /// Bulk requests.
    #[getter]
    fn bulk(&self) -> ::core::option::Option<PyScimBulkConfig> {
        self.0
            .bulk
            .clone()
            .into_option()
            .map(PyScimBulkConfig::from)
    }

    /// Filters.
    #[getter]
    fn filter(&self) -> ::core::option::Option<PyScimFilterConfig> {
        self.0
            .filter
            .clone()
            .into_option()
            .map(PyScimFilterConfig::from)
    }

    /// Password changes.
    #[getter]
    fn change_password(&self) -> ::core::option::Option<PyScimSupported> {
        self.0
            .change_password
            .clone()
            .into_option()
            .map(PyScimSupported::from)
    }

    /// Sorting.
    #[getter]
    fn sort(&self) -> ::core::option::Option<PyScimSupported> {
        self.0.sort.clone().into_option().map(PyScimSupported::from)
    }

    /// ETags.
    #[getter]
    fn etag(&self) -> ::core::option::Option<PyScimSupported> {
        self.0.etag.clone().into_option().map(PyScimSupported::from)
    }

    /// The ways to authenticate.
    #[getter]
    fn authentication_schemes(&self) -> ::std::vec::Vec<PyScimAuthenticationScheme> {
        self.0
            .authentication_schemes
            .iter()
            .cloned()
            .map(PyScimAuthenticationScheme::from)
            .collect()
    }

    /// The type and path of the resource.
    #[getter]
    fn meta(&self) -> ::core::option::Option<PyScimMeta> {
        self.0.meta.clone().into_option().map(PyScimMeta::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("patch", false),
                ("bulk", false),
                ("filter", false),
                ("change_password", false),
                ("sort", false),
                ("etag", false),
                ("authentication_schemes", false),
                ("meta", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimServiceProviderConfig>
    for PyScimServiceProviderConfig
{
    fn from(value: super::scim::v1::ScimServiceProviderConfig) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimServiceProviderConfig>
    for super::scim::v1::ScimServiceProviderConfig
{
    fn from(value: PyScimServiceProviderConfig) -> Self {
        value.0
    }
}

/// Whether the server supports a SCIM feature.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimSupported",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimSupported(pub super::scim::v1::ScimSupported);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimSupported {
    /// Whether the feature is supported.
    #[getter]
    fn supported(&self) -> bool {
        self.0.supported
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("supported", false)])
    }
}

impl ::core::convert::From<super::scim::v1::ScimSupported> for PyScimSupported {
    fn from(value: super::scim::v1::ScimSupported) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimSupported> for super::scim::v1::ScimSupported {
    fn from(value: PyScimSupported) -> Self {
        value.0
    }
}

/// A SCIM 2.0 user. The email of the user is its user name and its one primary email.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ScimUser",
    eq,
    frozen,
    module = "unitycatalog._core.scim",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyScimUser(pub super::scim::v1::ScimUser);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyScimUser {
    /// SCIM schema URIs of the resource: `urn:ietf:params:scim:schemas:core:2.0:User`.
    #[getter]
    fn schemas(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.schemas.clone()
    }

    /// Unique identifier of the user.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// The identity the user has at the client's identity provider. The token exchange resolves an
    /// OAuth client (`azp`, `client_id`) to the user by it.
    #[getter]
    fn external_id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.external_id.clone()
    }

    /// When the user was created and last changed.
    #[getter]
    fn meta(&self) -> ::core::option::Option<PyScimMeta> {
        self.0.meta.clone().into_option().map(PyScimMeta::from)
    }

    /// The email of the user. Ignored in requests: the server takes the primary email.
    #[getter]
    fn user_name(&self) -> ::std::string::String {
        self.0.user_name.clone()
    }

    /// Name of the user.
    #[getter]
    fn display_name(&self) -> ::core::option::Option<::std::string::String> {
        self.0.display_name.clone()
    }

    /// Whether the user can sign in. A new user is always enabled; absent in a replace request:
    /// unchanged.
    #[getter]
    fn active(&self) -> ::core::option::Option<bool> {
        self.0.active
    }

    /// Email addresses of the user. A new user needs one with `primary` set; a replace request
    /// does not change the email.
    #[getter]
    fn emails(&self) -> ::std::vec::Vec<PyScimEmail> {
        self.0
            .emails
            .iter()
            .cloned()
            .map(PyScimEmail::from)
            .collect()
    }

    /// Pictures of the user. The server keeps the first; a replace request does not change it.
    #[getter]
    fn photos(&self) -> ::std::vec::Vec<PyScimPhoto> {
        self.0
            .photos
            .iter()
            .cloned()
            .map(PyScimPhoto::from)
            .collect()
    }

    /// Roles of the user. Only `/scim2/Me` sends them: the role `admin` for a caller that can do
    /// what the metastore admin does. The server ignores roles in requests.
    #[getter]
    fn roles(&self) -> ::std::vec::Vec<PyScimRole> {
        self.0.roles.iter().cloned().map(PyScimRole::from).collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("schemas", false),
                ("id", false),
                ("external_id", false),
                ("meta", false),
                ("user_name", false),
                ("display_name", false),
                ("active", false),
                ("emails", false),
                ("photos", false),
                ("roles", false),
            ],
        )
    }
}

impl ::core::convert::From<super::scim::v1::ScimUser> for PyScimUser {
    fn from(value: super::scim::v1::ScimUser) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyScimUser> for super::scim::v1::ScimUser {
    fn from(value: PyScimUser) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListSharesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.shares",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListSharesResponse(pub super::shares::v1::ListSharesResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListSharesResponse {
    /// The shares.
    #[getter]
    fn items(&self) -> ::std::vec::Vec<PyShareInfo> {
        self.0
            .items
            .iter()
            .cloned()
            .map(PyShareInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("items", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::shares::v1::ListSharesResponse> for PyListSharesResponse {
    fn from(value: super::shares::v1::ListSharesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListSharesResponse> for super::shares::v1::ListSharesResponse {
    fn from(value: PyListSharesResponse) -> Self {
        value.0
    }
}

/// A share: a named set of tables that Delta Sharing recipients can read.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ShareInfo",
    eq,
    frozen,
    module = "unitycatalog._core.shares",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyShareInfo(pub super::shares::v1::ShareInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyShareInfo {
    /// Name of the share.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Unique identifier of the share.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Username of the owner of the share.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the share was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the share.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the share was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the share.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// The tables of the share.
    #[getter]
    fn objects(&self) -> ::std::vec::Vec<PySharedDataObject> {
        self.0
            .objects
            .iter()
            .cloned()
            .map(PySharedDataObject::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("id", false),
                ("comment", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("objects", false),
            ],
        )
    }
}

impl ::core::convert::From<super::shares::v1::ShareInfo> for PyShareInfo {
    fn from(value: super::shares::v1::ShareInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyShareInfo> for super::shares::v1::ShareInfo {
    fn from(value: PyShareInfo) -> Self {
        value.0
    }
}

/// A table of a share: a Unity Catalog table and the name under which the recipients see it.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharedDataObject",
    eq,
    frozen,
    module = "unitycatalog._core.shares",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharedDataObject(pub super::shares::v1::SharedDataObject);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharedDataObject {
    #[new]
    #[pyo3(signature = (source_table_full_name, shared_as_schema, shared_as_name))]
    fn new(
        source_table_full_name: ::std::string::String,
        shared_as_schema: ::std::string::String,
        shared_as_name: ::std::string::String,
    ) -> Self {
        let mut inner =
            <super::shares::v1::SharedDataObject as ::core::default::Default>::default();
        {
            let value = source_table_full_name;
            inner.source_table_full_name = value;
        }
        {
            let value = shared_as_schema;
            inner.shared_as_schema = value;
        }
        {
            let value = shared_as_name;
            inner.shared_as_name = value;
        }
        Self(inner)
    }

    /// Three-level name of the Unity Catalog table: `catalog.schema.table`. A share holds a table
    /// once.
    #[getter]
    fn source_table_full_name(&self) -> ::std::string::String {
        self.0.source_table_full_name.clone()
    }

    /// Schema under which the recipients see the table, such as `default`.
    #[getter]
    fn shared_as_schema(&self) -> ::std::string::String {
        self.0.shared_as_schema.clone()
    }

    /// Name under which the recipients see the table.
    #[getter]
    fn shared_as_name(&self) -> ::std::string::String {
        self.0.shared_as_name.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("source_table_full_name", false),
                ("shared_as_schema", false),
                ("shared_as_name", false),
            ],
        )
    }
}

impl ::core::convert::From<super::shares::v1::SharedDataObject> for PySharedDataObject {
    fn from(value: super::shares::v1::SharedDataObject) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharedDataObject> for super::shares::v1::SharedDataObject {
    fn from(value: PySharedDataObject) -> Self {
        value.0
    }
}

/// A Delta Sharing profile file (`profile.json`): what a Delta Sharing client needs to connect to
/// the server as a recipient.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "RecipientProfile",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyRecipientProfile(pub super::sharing::v1::RecipientProfile);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyRecipientProfile {
    /// Version of the profile file format. Always 1.
    #[getter]
    fn share_credentials_version(&self) -> i32 {
        self.0.share_credentials_version
    }

    /// URL of the Delta Sharing server: the URL the caller used, ending in
    /// `/api/2.1/unity-catalog/delta-sharing`.
    #[getter]
    fn endpoint(&self) -> ::std::string::String {
        self.0.endpoint.clone()
    }

    /// The bearer token of the recipient.
    #[getter]
    fn bearer_token(&self) -> ::std::string::String {
        self.0.bearer_token.clone()
    }

    /// Time at which the token expires, as an ISO 8601 timestamp. The server sends `null`: the tokens
    /// do not expire.
    #[getter]
    fn expiration_time(&self) -> ::core::option::Option<::std::string::String> {
        self.0.expiration_time.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("share_credentials_version", false),
                ("endpoint", false),
                ("bearer_token", true),
                ("expiration_time", false),
            ],
        )
    }
}

impl ::core::convert::From<super::sharing::v1::RecipientProfile> for PyRecipientProfile {
    fn from(value: super::sharing::v1::RecipientProfile) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyRecipientProfile> for super::sharing::v1::RecipientProfile {
    fn from(value: PyRecipientProfile) -> Self {
        value.0
    }
}

/// The format of the data files of a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingFormat",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingFormat(pub super::sharing::v1::SharingFormat);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingFormat {
    /// Name of the file format. Always `parquet`.
    #[getter]
    fn provider(&self) -> ::std::string::String {
        self.0.provider.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("provider", false)])
    }
}

impl ::core::convert::From<super::sharing::v1::SharingFormat> for PySharingFormat {
    fn from(value: super::sharing::v1::SharingFormat) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingFormat> for super::sharing::v1::SharingFormat {
    fn from(value: PySharingFormat) -> Self {
        value.0
    }
}

/// The version of a table. The response has no body.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingGetTableVersionResponse",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingGetTableVersionResponse(pub super::sharing::v1::SharingGetTableVersionResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingGetTableVersionResponse {
    /// Version of the table, from the `Delta-Table-Version` header. Always 0: the server does not
    /// read the Delta log.
    #[getter]
    fn version(&self) -> i64 {
        self.0.version
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("version", false)])
    }
}

impl ::core::convert::From<super::sharing::v1::SharingGetTableVersionResponse>
    for PySharingGetTableVersionResponse
{
    fn from(value: super::sharing::v1::SharingGetTableVersionResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingGetTableVersionResponse>
    for super::sharing::v1::SharingGetTableVersionResponse
{
    fn from(value: PySharingGetTableVersionResponse) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingListSchemasResponse",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingListSchemasResponse(pub super::sharing::v1::SharingListSchemasResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingListSchemasResponse {
    /// The schemas, sorted by name.
    #[getter]
    fn items(&self) -> ::std::vec::Vec<PySharingSchema> {
        self.0
            .items
            .iter()
            .cloned()
            .map(PySharingSchema::from)
            .collect()
    }

    /// Opaque token of the next page. The server does not send it: the first page holds all
    /// schemas.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("items", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::sharing::v1::SharingListSchemasResponse>
    for PySharingListSchemasResponse
{
    fn from(value: super::sharing::v1::SharingListSchemasResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingListSchemasResponse>
    for super::sharing::v1::SharingListSchemasResponse
{
    fn from(value: PySharingListSchemasResponse) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingListSharesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingListSharesResponse(pub super::sharing::v1::SharingListSharesResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingListSharesResponse {
    /// The shares.
    #[getter]
    fn items(&self) -> ::std::vec::Vec<PySharingShare> {
        self.0
            .items
            .iter()
            .cloned()
            .map(PySharingShare::from)
            .collect()
    }

    /// Opaque token of the next page. The server does not send it: the first page holds all shares.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("items", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::sharing::v1::SharingListSharesResponse>
    for PySharingListSharesResponse
{
    fn from(value: super::sharing::v1::SharingListSharesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingListSharesResponse>
    for super::sharing::v1::SharingListSharesResponse
{
    fn from(value: PySharingListSharesResponse) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingListTablesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingListTablesResponse(pub super::sharing::v1::SharingListTablesResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingListTablesResponse {
    /// The tables.
    #[getter]
    fn items(&self) -> ::std::vec::Vec<PySharingTable> {
        self.0
            .items
            .iter()
            .cloned()
            .map(PySharingTable::from)
            .collect()
    }

    /// Opaque token of the next page. The server does not send it: the first page holds all tables.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("items", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::sharing::v1::SharingListTablesResponse>
    for PySharingListTablesResponse
{
    fn from(value: super::sharing::v1::SharingListTablesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingListTablesResponse>
    for super::sharing::v1::SharingListTablesResponse
{
    fn from(value: PySharingListTablesResponse) -> Self {
        value.0
    }
}

/// The metadata of a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingMetadata",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingMetadata(pub super::sharing::v1::SharingMetadata);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingMetadata {
    /// Unique identifier of the table. Empty when the table has none.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Name of the Unity Catalog table.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Format of the data files of the table.
    #[getter]
    fn format(&self) -> ::core::option::Option<PySharingFormat> {
        self.0
            .format
            .clone()
            .into_option()
            .map(PySharingFormat::from)
    }

    /// The columns of the Unity Catalog table: its `columns` as a JSON array in a string, not a Delta
    /// schema. Empty when the table has no columns.
    #[getter]
    fn schema_string(&self) -> ::std::string::String {
        self.0.schema_string.clone()
    }

    /// Names of the partition columns. Always empty.
    #[getter]
    fn partition_columns(&self) -> ::std::vec::Vec<::std::string::String> {
        self.0.partition_columns.clone()
    }

    /// Configuration of the table. Always empty.
    #[getter]
    fn configuration(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.configuration.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("id", false),
                ("name", false),
                ("format", false),
                ("schema_string", false),
                ("partition_columns", false),
                ("configuration", false),
            ],
        )
    }
}

impl ::core::convert::From<super::sharing::v1::SharingMetadata> for PySharingMetadata {
    fn from(value: super::sharing::v1::SharingMetadata) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingMetadata> for super::sharing::v1::SharingMetadata {
    fn from(value: PySharingMetadata) -> Self {
        value.0
    }
}

/// The protocol of a table: the minimum version of the Delta Sharing reader that can read it.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingProtocol",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingProtocol(pub super::sharing::v1::SharingProtocol);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingProtocol {
    /// Minimum reader version. Always 1.
    #[getter]
    fn min_reader_version(&self) -> i32 {
        self.0.min_reader_version
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("min_reader_version", false)])
    }
}

impl ::core::convert::From<super::sharing::v1::SharingProtocol> for PySharingProtocol {
    fn from(value: super::sharing::v1::SharingProtocol) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingProtocol> for super::sharing::v1::SharingProtocol {
    fn from(value: PySharingProtocol) -> Self {
        value.0
    }
}

/// A schema of a share: the `shared_as_schema` of one or more of its tables.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingSchema",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingSchema(pub super::sharing::v1::SharingSchema);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingSchema {
    /// Name of the schema.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the share.
    #[getter]
    fn share(&self) -> ::std::string::String {
        self.0.share.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("name", false), ("share", false)])
    }
}

impl ::core::convert::From<super::sharing::v1::SharingSchema> for PySharingSchema {
    fn from(value: super::sharing::v1::SharingSchema) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingSchema> for super::sharing::v1::SharingSchema {
    fn from(value: PySharingSchema) -> Self {
        value.0
    }
}

/// A share that the recipient can read.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingShare",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingShare(pub super::sharing::v1::SharingShare);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingShare {
    /// Name of the share.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Unique identifier of the share. The server does not send it.
    #[getter]
    fn id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("name", false), ("id", false)])
    }
}

impl ::core::convert::From<super::sharing::v1::SharingShare> for PySharingShare {
    fn from(value: super::sharing::v1::SharingShare) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingShare> for super::sharing::v1::SharingShare {
    fn from(value: PySharingShare) -> Self {
        value.0
    }
}

/// A table of a share, by the name under which the recipient sees it.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingTable",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingTable(pub super::sharing::v1::SharingTable);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingTable {
    /// Name of the table.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the schema of the table.
    #[getter]
    fn schema(&self) -> ::std::string::String {
        self.0.schema.clone()
    }

    /// Name of the share.
    #[getter]
    fn share(&self) -> ::std::string::String {
        self.0.share.clone()
    }

    /// Unique identifier of the table. The server does not send it.
    #[getter]
    fn id(&self) -> ::core::option::Option<::std::string::String> {
        self.0.id.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("schema", false),
                ("share", false),
                ("id", false),
            ],
        )
    }
}

impl ::core::convert::From<super::sharing::v1::SharingTable> for PySharingTable {
    fn from(value: super::sharing::v1::SharingTable) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingTable> for super::sharing::v1::SharingTable {
    fn from(value: PySharingTable) -> Self {
        value.0
    }
}

/// One line of the NDJSON answer of `SharingGetTableMetadata` and `SharingQueryTable`: an object
/// with one key. The first line holds `protocol`, the second `metaData`.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "SharingTableLine",
    eq,
    frozen,
    module = "unitycatalog._core.sharing",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PySharingTableLine(pub super::sharing::v1::SharingTableLine);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PySharingTableLine {
    /// The protocol of the table.
    #[getter]
    fn protocol(&self) -> ::core::option::Option<PySharingProtocol> {
        self.0
            .protocol
            .clone()
            .into_option()
            .map(PySharingProtocol::from)
    }

    /// The metadata of the table.
    #[getter]
    fn metadata(&self) -> ::core::option::Option<PySharingMetadata> {
        self.0
            .metadata
            .clone()
            .into_option()
            .map(PySharingMetadata::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("protocol", false), ("metadata", false)])
    }
}

impl ::core::convert::From<super::sharing::v1::SharingTableLine> for PySharingTableLine {
    fn from(value: super::sharing::v1::SharingTableLine) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PySharingTableLine> for super::sharing::v1::SharingTableLine {
    fn from(value: PySharingTableLine) -> Self {
        value.0
    }
}

/// A column of a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ColumnInfo",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyColumnInfo(pub super::tables::v1::ColumnInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyColumnInfo {
    #[new]
    #[pyo3(
        signature = (
            name,
            type_json,
            *,
            type_text = None,
            type_name = None,
            type_precision = None,
            type_scale = None,
            type_interval_type = None,
            position = None,
            comment = None,
            nullable = None,
            partition_index = None
        )
    )]
    fn new(
        name: ::std::string::String,
        type_json: ::std::string::String,
        type_text: ::core::option::Option<::std::string::String>,
        type_name: ::core::option::Option<PyColumnTypeName>,
        type_precision: ::core::option::Option<i32>,
        type_scale: ::core::option::Option<i32>,
        type_interval_type: ::core::option::Option<::std::string::String>,
        position: ::core::option::Option<i32>,
        comment: ::core::option::Option<::std::string::String>,
        nullable: ::core::option::Option<bool>,
        partition_index: ::core::option::Option<i32>,
    ) -> Self {
        let mut inner = <super::tables::v1::ColumnInfo as ::core::default::Default>::default();
        {
            let value = name;
            inner.name = value;
        }
        {
            let value = type_json;
            inner.type_json = value;
        }
        {
            let value = type_text;
            inner.type_text = value;
        }
        {
            let value = type_name;
            inner.type_name = value.map(|e| {
                ::buffa::EnumValue::Known(
                    <super::tables::v1::ColumnTypeName as ::core::convert::From<_>>::from(e),
                )
            });
        }
        {
            let value = type_precision;
            inner.type_precision = value;
        }
        {
            let value = type_scale;
            inner.type_scale = value;
        }
        {
            let value = type_interval_type;
            inner.type_interval_type = value;
        }
        {
            let value = position;
            inner.position = value;
        }
        {
            let value = comment;
            inner.comment = value;
        }
        {
            let value = nullable;
            inner.nullable = value;
        }
        {
            let value = partition_index;
            inner.partition_index = value;
        }
        Self(inner)
    }

    /// Name of the column.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Full data type of the column as SQL text, such as `decimal(10,2)`.
    #[getter]
    fn type_text(&self) -> ::core::option::Option<::std::string::String> {
        self.0.type_text.clone()
    }

    /// Full data type of the column as a JSON Spark `StructField`.
    #[getter]
    fn type_json(&self) -> ::std::string::String {
        self.0.type_json.clone()
    }

    /// Name of the data type of the column.
    #[getter]
    fn type_name(&self) -> ::core::option::Option<PyColumnTypeName> {
        self.0
            .type_name
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| <super::tables::v1::ColumnTypeName as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyColumnTypeName::from)
    }

    /// Digits of precision. Required for decimal types.
    #[getter]
    fn type_precision(&self) -> ::core::option::Option<i32> {
        self.0.type_precision
    }

    /// Digits to the right of the decimal point. Required for decimal types.
    #[getter]
    fn type_scale(&self) -> ::core::option::Option<i32> {
        self.0.type_scale
    }

    /// Format of an interval type.
    #[getter]
    fn type_interval_type(&self) -> ::core::option::Option<::std::string::String> {
        self.0.type_interval_type.clone()
    }

    /// Position of the column, starting at 0.
    #[getter]
    fn position(&self) -> ::core::option::Option<i32> {
        self.0.position
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Whether the column can hold null values. The server sets false when it is absent.
    #[getter]
    fn nullable(&self) -> ::core::option::Option<bool> {
        self.0.nullable
    }

    /// Position of the column in the partition key of the table.
    #[getter]
    fn partition_index(&self) -> ::core::option::Option<i32> {
        self.0.partition_index
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("type_text", false),
                ("type_json", false),
                ("type_name", false),
                ("type_precision", false),
                ("type_scale", false),
                ("type_interval_type", false),
                ("position", false),
                ("comment", false),
                ("nullable", false),
                ("partition_index", false),
            ],
        )
    }
}

impl ::core::convert::From<super::tables::v1::ColumnInfo> for PyColumnInfo {
    fn from(value: super::tables::v1::ColumnInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyColumnInfo> for super::tables::v1::ColumnInfo {
    fn from(value: PyColumnInfo) -> Self {
        value.0
    }
}

/// A dependency of a view or function. Exactly one of the fields is set.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "Dependency",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDependency(pub super::tables::v1::Dependency);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDependency {
    #[new]
    #[pyo3(signature = (*, table = None, function = None))]
    fn new(
        table: ::core::option::Option<PyTableDependency>,
        function: ::core::option::Option<PyFunctionDependency>,
    ) -> Self {
        let mut inner = <super::tables::v1::Dependency as ::core::default::Default>::default();
        {
            let value = table;
            inner.table = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        {
            let value = function;
            inner.function = value
                .map(|w| ::buffa::MessageField::some(w.into()))
                .unwrap_or_default();
        }
        Self(inner)
    }

    /// A table dependency.
    #[getter]
    fn table(&self) -> ::core::option::Option<PyTableDependency> {
        self.0
            .table
            .clone()
            .into_option()
            .map(PyTableDependency::from)
    }

    /// A function dependency.
    #[getter]
    fn function(&self) -> ::core::option::Option<PyFunctionDependency> {
        self.0
            .function
            .clone()
            .into_option()
            .map(PyFunctionDependency::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("table", false), ("function", false)])
    }
}

impl ::core::convert::From<super::tables::v1::Dependency> for PyDependency {
    fn from(value: super::tables::v1::Dependency) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDependency> for super::tables::v1::Dependency {
    fn from(value: PyDependency) -> Self {
        value.0
    }
}

/// The dependencies of a view or function.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "DependencyList",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyDependencyList(pub super::tables::v1::DependencyList);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyDependencyList {
    #[new]
    #[pyo3(signature = (*, dependencies = None))]
    fn new(dependencies: ::core::option::Option<::std::vec::Vec<PyDependency>>) -> Self {
        let mut inner = <super::tables::v1::DependencyList as ::core::default::Default>::default();
        if let ::core::option::Option::Some(value) = dependencies {
            inner.dependencies = value.into_iter().map(::core::convert::Into::into).collect();
        }
        Self(inner)
    }

    /// The dependencies.
    #[getter]
    fn dependencies(&self) -> ::std::vec::Vec<PyDependency> {
        self.0
            .dependencies
            .iter()
            .cloned()
            .map(PyDependency::from)
            .collect()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("dependencies", false)])
    }
}

impl ::core::convert::From<super::tables::v1::DependencyList> for PyDependencyList {
    fn from(value: super::tables::v1::DependencyList) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyDependencyList> for super::tables::v1::DependencyList {
    fn from(value: PyDependencyList) -> Self {
        value.0
    }
}

/// A function that a view or function depends on.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "FunctionDependency",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyFunctionDependency(pub super::tables::v1::FunctionDependency);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyFunctionDependency {
    #[new]
    #[pyo3(signature = (function_full_name))]
    fn new(function_full_name: ::std::string::String) -> Self {
        let mut inner =
            <super::tables::v1::FunctionDependency as ::core::default::Default>::default();
        {
            let value = function_full_name;
            inner.function_full_name = value;
        }
        Self(inner)
    }

    /// Full name of the function, in the form `<catalog>.<schema>.<function>`.
    #[getter]
    fn function_full_name(&self) -> ::std::string::String {
        self.0.function_full_name.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("function_full_name", false)])
    }
}

impl ::core::convert::From<super::tables::v1::FunctionDependency> for PyFunctionDependency {
    fn from(value: super::tables::v1::FunctionDependency) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyFunctionDependency> for super::tables::v1::FunctionDependency {
    fn from(value: PyFunctionDependency) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListTablesResponse",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListTablesResponse(pub super::tables::v1::ListTablesResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListTablesResponse {
    /// The tables.
    #[getter]
    fn tables(&self) -> ::std::vec::Vec<PyTableInfo> {
        self.0
            .tables
            .iter()
            .cloned()
            .map(PyTableInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("tables", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::tables::v1::ListTablesResponse> for PyListTablesResponse {
    fn from(value: super::tables::v1::ListTablesResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListTablesResponse> for super::tables::v1::ListTablesResponse {
    fn from(value: PyListTablesResponse) -> Self {
        value.0
    }
}

/// A staging table: a table id and managed storage location reserved for a MANAGED table that a
/// client is about to create.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "StagingTableInfo",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyStagingTableInfo(pub super::tables::v1::StagingTableInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyStagingTableInfo {
    /// Name of the table, relative to the parent schema.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// Name of the parent schema.
    #[getter]
    fn schema_name(&self) -> ::std::string::String {
        self.0.schema_name.clone()
    }

    /// Unique identifier of the staging table. The table gets it when it is created.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Storage location URL reserved for the table. Create the table with it as its
    /// `storage_location`.
    #[getter]
    fn staging_location(&self) -> ::std::string::String {
        self.0.staging_location.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("catalog_name", false),
                ("schema_name", false),
                ("id", false),
                ("staging_location", false),
            ],
        )
    }
}

impl ::core::convert::From<super::tables::v1::StagingTableInfo> for PyStagingTableInfo {
    fn from(value: super::tables::v1::StagingTableInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyStagingTableInfo> for super::tables::v1::StagingTableInfo {
    fn from(value: PyStagingTableInfo) -> Self {
        value.0
    }
}

/// A table that a view or function depends on.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "TableDependency",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTableDependency(pub super::tables::v1::TableDependency);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyTableDependency {
    #[new]
    #[pyo3(signature = (table_full_name))]
    fn new(table_full_name: ::std::string::String) -> Self {
        let mut inner = <super::tables::v1::TableDependency as ::core::default::Default>::default();
        {
            let value = table_full_name;
            inner.table_full_name = value;
        }
        Self(inner)
    }

    /// Full name of the table, in the form `<catalog>.<schema>.<table>`.
    #[getter]
    fn table_full_name(&self) -> ::std::string::String {
        self.0.table_full_name.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("table_full_name", false)])
    }
}

impl ::core::convert::From<super::tables::v1::TableDependency> for PyTableDependency {
    fn from(value: super::tables::v1::TableDependency) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyTableDependency> for super::tables::v1::TableDependency {
    fn from(value: PyTableDependency) -> Self {
        value.0
    }
}

/// A table: data in the third layer of the three-level namespace, below a schema.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "TableInfo",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTableInfo(pub super::tables::v1::TableInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyTableInfo {
    /// Name of the table, relative to the parent schema.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// Name of the parent schema.
    #[getter]
    fn schema_name(&self) -> ::std::string::String {
        self.0.schema_name.clone()
    }

    /// Type of the table.
    #[getter]
    fn table_type(&self) -> ::core::option::Option<PyTableType> {
        self.0
            .table_type
            .as_known()
            .filter(|e| <super::tables::v1::TableType as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyTableType::from)
    }

    /// File format of the data of the table.
    #[getter]
    fn data_source_format(&self) -> ::core::option::Option<PyDataSourceFormat> {
        self.0
            .data_source_format
            .as_ref()
            .and_then(|e| e.as_known())
            .filter(|e| {
                <super::tables::v1::DataSourceFormat as ::buffa::Enumeration>::to_i32(e) != 0
            })
            .map(PyDataSourceFormat::from)
    }

    /// The columns of the table.
    #[getter]
    fn columns(&self) -> ::std::vec::Vec<PyColumnInfo> {
        self.0
            .columns
            .iter()
            .cloned()
            .map(PyColumnInfo::from)
            .collect()
    }

    /// Storage location URL of the table. Absent for views.
    #[getter]
    fn storage_location(&self) -> ::core::option::Option<::std::string::String> {
        self.0.storage_location.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Key-value properties of the table.
    #[getter]
    fn properties(
        &self,
    ) -> ::std::collections::HashMap<::std::string::String, ::std::string::String> {
        self.0.properties.clone()
    }

    /// Username of the owner of the table.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the table was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the table.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the table was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the table.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the table.
    #[getter]
    fn table_id(&self) -> ::std::string::String {
        self.0.table_id.clone()
    }

    /// Definition of a view-like table: SQL for views, YAML for metric views.
    #[getter]
    fn view_definition(&self) -> ::core::option::Option<::std::string::String> {
        self.0.view_definition.clone()
    }

    /// The tables and functions that a view-like table reads.
    #[getter]
    fn view_dependencies(&self) -> ::core::option::Option<PyDependencyList> {
        self.0
            .view_dependencies
            .clone()
            .into_option()
            .map(PyDependencyList::from)
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("name", false),
                ("catalog_name", false),
                ("schema_name", false),
                ("table_type", false),
                ("data_source_format", false),
                ("columns", false),
                ("storage_location", false),
                ("comment", false),
                ("properties", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("table_id", false),
                ("view_definition", false),
                ("view_dependencies", false),
            ],
        )
    }
}

impl ::core::convert::From<super::tables::v1::TableInfo> for PyTableInfo {
    fn from(value: super::tables::v1::TableInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyTableInfo> for super::tables::v1::TableInfo {
    fn from(value: PyTableInfo) -> Self {
        value.0
    }
}

/// A new comment for a column of a table.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "UpdateColumn",
    eq,
    frozen,
    module = "unitycatalog._core.tables",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyUpdateColumn(pub super::tables::v1::UpdateColumn);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyUpdateColumn {
    #[new]
    #[pyo3(signature = (name, comment))]
    fn new(name: ::std::string::String, comment: ::std::string::String) -> Self {
        let mut inner = <super::tables::v1::UpdateColumn as ::core::default::Default>::default();
        {
            let value = name;
            inner.name = value;
        }
        {
            let value = comment;
            inner.comment = value;
        }
        Self(inner)
    }

    /// Name of the column.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// New comment of the column. Empty clears it.
    #[getter]
    fn comment(&self) -> ::std::string::String {
        self.0.comment.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(slf.as_any(), &[("name", false), ("comment", false)])
    }
}

impl ::core::convert::From<super::tables::v1::UpdateColumn> for PyUpdateColumn {
    fn from(value: super::tables::v1::UpdateColumn) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyUpdateColumn> for super::tables::v1::UpdateColumn {
    fn from(value: PyUpdateColumn) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListPatsResponse",
    eq,
    frozen,
    module = "unitycatalog._core.tokens",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListPatsResponse(pub super::tokens::v1::ListPatsResponse);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListPatsResponse {
    /// The tokens.
    #[getter]
    fn tokens(&self) -> ::std::vec::Vec<PyPatInfo> {
        self.0.tokens.iter().cloned().map(PyPatInfo::from).collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("tokens", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::tokens::v1::ListPatsResponse> for PyListPatsResponse {
    fn from(value: super::tokens::v1::ListPatsResponse) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListPatsResponse> for super::tokens::v1::ListPatsResponse {
    fn from(value: PyListPatsResponse) -> Self {
        value.0
    }
}

/// A personal access token of a user. The server keeps a hash of the token only: the token itself
/// is in the response of `CreateToken` and nowhere else.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "PatInfo",
    eq,
    frozen,
    module = "unitycatalog._core.tokens",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPatInfo(pub super::tokens::v1::PatInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyPatInfo {
    /// Unique identifier of the token.
    #[getter]
    fn id(&self) -> ::std::string::String {
        self.0.id.clone()
    }

    /// Name of the token.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// The first 8 characters of the token, for display only.
    #[getter]
    fn token_prefix(&self) -> ::std::string::String {
        self.0.token_prefix.clone()
    }

    /// Username of the owner of the token.
    #[getter]
    fn owner(&self) -> ::std::string::String {
        self.0.owner.clone()
    }

    /// Time at which the token was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Time at which the token expires, in epoch milliseconds. Absent or 0: the token does not
    /// expire.
    #[getter]
    fn expires_at(&self) -> ::core::option::Option<i64> {
        self.0.expires_at
    }

    /// The token. Set only in the response of `CreateToken`.
    #[getter]
    fn token_value(&self) -> ::core::option::Option<::std::string::String> {
        self.0.token_value.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("id", false),
                ("name", false),
                ("token_prefix", false),
                ("owner", false),
                ("created_at", false),
                ("expires_at", false),
                ("token_value", true),
            ],
        )
    }
}

impl ::core::convert::From<super::tokens::v1::PatInfo> for PyPatInfo {
    fn from(value: super::tokens::v1::PatInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyPatInfo> for super::tokens::v1::PatInfo {
    fn from(value: PyPatInfo) -> Self {
        value.0
    }
}

#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "ListVolumesResponseContent",
    eq,
    frozen,
    module = "unitycatalog._core.volumes",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyListVolumesResponseContent(pub super::volumes::v1::ListVolumesResponseContent);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyListVolumesResponseContent {
    /// The volumes.
    #[getter]
    fn volumes(&self) -> ::std::vec::Vec<PyVolumeInfo> {
        self.0
            .volumes
            .iter()
            .cloned()
            .map(PyVolumeInfo::from)
            .collect()
    }

    /// Opaque token of the next page. Absent on the last page.
    #[getter]
    fn next_page_token(&self) -> ::core::option::Option<::std::string::String> {
        self.0.next_page_token.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[("volumes", false), ("next_page_token", false)],
        )
    }
}

impl ::core::convert::From<super::volumes::v1::ListVolumesResponseContent>
    for PyListVolumesResponseContent
{
    fn from(value: super::volumes::v1::ListVolumesResponseContent) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyListVolumesResponseContent>
    for super::volumes::v1::ListVolumesResponseContent
{
    fn from(value: PyListVolumesResponseContent) -> Self {
        value.0
    }
}

/// A volume: files in the third layer of the three-level namespace, below a schema.
#[::pyo3_stub_gen::derive::gen_stub_pyclass]
#[::pyo3::pyclass(
    name = "VolumeInfo",
    eq,
    frozen,
    module = "unitycatalog._core.volumes",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyVolumeInfo(pub super::volumes::v1::VolumeInfo);
#[allow(clippy::too_many_arguments, clippy::useless_conversion)]
#[::pyo3_stub_gen::derive::gen_stub_pymethods]
#[::pyo3::pymethods]
impl PyVolumeInfo {
    /// Name of the parent catalog.
    #[getter]
    fn catalog_name(&self) -> ::std::string::String {
        self.0.catalog_name.clone()
    }

    /// Name of the parent schema.
    #[getter]
    fn schema_name(&self) -> ::std::string::String {
        self.0.schema_name.clone()
    }

    /// Name of the volume, relative to the parent schema.
    #[getter]
    fn name(&self) -> ::std::string::String {
        self.0.name.clone()
    }

    /// User-provided free-form text description.
    #[getter]
    fn comment(&self) -> ::core::option::Option<::std::string::String> {
        self.0.comment.clone()
    }

    /// Username of the owner of the volume.
    #[getter]
    fn owner(&self) -> ::core::option::Option<::std::string::String> {
        self.0.owner.clone()
    }

    /// Time at which the volume was created, in epoch milliseconds.
    #[getter]
    fn created_at(&self) -> i64 {
        self.0.created_at
    }

    /// Username of the creator of the volume.
    #[getter]
    fn created_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.created_by.clone()
    }

    /// Time at which the volume was last changed, in epoch milliseconds.
    #[getter]
    fn updated_at(&self) -> i64 {
        self.0.updated_at
    }

    /// Username of the user who last changed the volume.
    #[getter]
    fn updated_by(&self) -> ::core::option::Option<::std::string::String> {
        self.0.updated_by.clone()
    }

    /// Unique identifier of the volume.
    #[getter]
    fn volume_id(&self) -> ::std::string::String {
        self.0.volume_id.clone()
    }

    /// Type of the volume.
    #[getter]
    fn volume_type(&self) -> ::core::option::Option<PyVolumeType> {
        self.0
            .volume_type
            .as_known()
            .filter(|e| <super::volumes::v1::VolumeType as ::buffa::Enumeration>::to_i32(e) != 0)
            .map(PyVolumeType::from)
    }

    /// Storage location URL of the volume.
    #[getter]
    fn storage_location(&self) -> ::std::string::String {
        self.0.storage_location.clone()
    }

    /// Full name of the volume, in the form `<catalog>.<schema>.<volume>`.
    #[getter]
    fn full_name(&self) -> ::std::string::String {
        self.0.full_name.clone()
    }

    fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<::std::string::String> {
        __python_repr(
            slf.as_any(),
            &[
                ("catalog_name", false),
                ("schema_name", false),
                ("name", false),
                ("comment", false),
                ("owner", false),
                ("created_at", false),
                ("created_by", false),
                ("updated_at", false),
                ("updated_by", false),
                ("volume_id", false),
                ("volume_type", false),
                ("storage_location", false),
                ("full_name", false),
            ],
        )
    }
}

impl ::core::convert::From<super::volumes::v1::VolumeInfo> for PyVolumeInfo {
    fn from(value: super::volumes::v1::VolumeInfo) -> Self {
        Self(value)
    }
}

impl ::core::convert::From<PyVolumeInfo> for super::volumes::v1::VolumeInfo {
    fn from(value: PyVolumeInfo) -> Self {
        value.0
    }
}

/// `Name(field=value, …)` with the `repr` of each attribute in `fields`, without the
/// ones that are `None`. A field marked `true` (`debug_redact` in the proto) shows `'***'`.
fn __python_repr(
    obj: &::pyo3::Bound<'_, ::pyo3::PyAny>,
    fields: &[(&str, bool)],
) -> ::pyo3::PyResult<::std::string::String> {
    use ::pyo3::types::{PyAnyMethods, PyTypeMethods};
    let mut shown = ::std::vec::Vec::with_capacity(fields.len());
    for (field, redact) in fields {
        let value = obj.getattr(*field)?;
        if value.is_none() {
            continue;
        }
        if *redact {
            shown.push(::std::format!("{field}='***'"));
        } else {
            shown.push(::std::format!("{field}={}", value.repr()?));
        }
    }
    Ok(::std::format!(
        "{}({})",
        obj.get_type().name()?,
        shown.join(", ")
    ))
}

/// A JSON value as a Python object: `None`, `bool`, `int` for a whole number, `float`,
/// `str`, `list` or `dict`.
pub fn json_value_to_py(
    py: ::pyo3::Python<'_>,
    value: &::buffa_types::google::protobuf::Value,
) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
    use ::buffa_types::google::protobuf::value::Kind;
    use ::pyo3::IntoPyObjectExt;
    match &value.kind {
        ::core::option::Option::None | ::core::option::Option::Some(Kind::NullValue(_)) => {
            ::core::result::Result::Ok(py.None())
        }
        ::core::option::Option::Some(Kind::BoolValue(b)) => b.into_py_any(py),
        ::core::option::Option::Some(Kind::NumberValue(n))
            if n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_992.0 =>
        {
            (*n as i64).into_py_any(py)
        }
        ::core::option::Option::Some(Kind::NumberValue(n)) => n.into_py_any(py),
        ::core::option::Option::Some(Kind::StringValue(s)) => s.into_py_any(py),
        ::core::option::Option::Some(Kind::StructValue(s)) => json_struct_to_py(py, s),
        ::core::option::Option::Some(Kind::ListValue(l)) => json_list_to_py(py, l),
    }
}

/// A JSON object as a Python `dict`.
pub fn json_struct_to_py(
    py: ::pyo3::Python<'_>,
    value: &::buffa_types::google::protobuf::Struct,
) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
    let dict = ::pyo3::types::PyDict::new(py);
    for (key, item) in &value.fields {
        ::pyo3::types::PyDictMethods::set_item(&dict, key, json_value_to_py(py, item)?)?;
    }
    ::core::result::Result::Ok(dict.into_any().unbind())
}

/// A JSON array as a Python `list`.
pub fn json_list_to_py(
    py: ::pyo3::Python<'_>,
    value: &::buffa_types::google::protobuf::ListValue,
) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
    let items = value
        .values
        .iter()
        .map(|item| json_value_to_py(py, item))
        .collect::<::pyo3::PyResult<::std::vec::Vec<_>>>()?;
    ::core::result::Result::Ok(::pyo3::types::PyList::new(py, items)?.into_any().unbind())
}

/// A Python object as a JSON value. It takes `None`, `bool`, `int`, `float`, `str`, and
/// `list`, `tuple` and `dict` of those.
pub fn json_value_from_py(
    obj: &::pyo3::Bound<'_, ::pyo3::PyAny>,
) -> ::pyo3::PyResult<::buffa_types::google::protobuf::Value> {
    use ::buffa_types::google::protobuf::Value;
    use ::pyo3::types::{PyAnyMethods, PyTypeMethods};
    if obj.is_none() {
        return ::core::result::Result::Ok(Value::null());
    }
    if obj.is_instance_of::<::pyo3::types::PyBool>() {
        return ::core::result::Result::Ok(Value::from(obj.extract::<bool>()?));
    }
    if obj.is_instance_of::<::pyo3::types::PyInt>()
        || obj.is_instance_of::<::pyo3::types::PyFloat>()
    {
        let number = obj.extract::<f64>()?;
        if !number.is_finite() {
            return ::core::result::Result::Err(::pyo3::exceptions::PyValueError::new_err(
                ::std::format!("not a JSON number: {number}"),
            ));
        }
        return ::core::result::Result::Ok(Value::from(number));
    }
    if obj.is_instance_of::<::pyo3::types::PyString>() {
        return ::core::result::Result::Ok(Value::from(obj.extract::<::std::string::String>()?));
    }
    if obj.is_instance_of::<::pyo3::types::PyDict>() {
        return ::core::result::Result::Ok(Value::from(json_struct_from_py(obj)?));
    }
    if obj.is_instance_of::<::pyo3::types::PyList>()
        || obj.is_instance_of::<::pyo3::types::PyTuple>()
    {
        return ::core::result::Result::Ok(Value::from(json_list_from_py(obj)?));
    }
    ::core::result::Result::Err(::pyo3::exceptions::PyTypeError::new_err(::std::format!(
        "not a JSON value: {}",
        obj.get_type().name()?
    )))
}

/// A Python `dict` with `str` keys as a JSON object.
pub fn json_struct_from_py(
    obj: &::pyo3::Bound<'_, ::pyo3::PyAny>,
) -> ::pyo3::PyResult<::buffa_types::google::protobuf::Struct> {
    let items: ::std::collections::HashMap<
        ::std::string::String,
        ::pyo3::Bound<'_, ::pyo3::PyAny>,
    > = ::pyo3::types::PyAnyMethods::extract(obj)?;
    let mut value = ::buffa_types::google::protobuf::Struct::default();
    for (key, item) in &items {
        value.fields.insert(key.clone(), json_value_from_py(item)?);
    }
    ::core::result::Result::Ok(value)
}

/// A Python `list` or `tuple` as a JSON array.
pub fn json_list_from_py(
    obj: &::pyo3::Bound<'_, ::pyo3::PyAny>,
) -> ::pyo3::PyResult<::buffa_types::google::protobuf::ListValue> {
    let items: ::std::vec::Vec<::pyo3::Bound<'_, ::pyo3::PyAny>> =
        ::pyo3::types::PyAnyMethods::extract(obj)?;
    let mut value = ::buffa_types::google::protobuf::ListValue::default();
    for item in &items {
        value.values.push(json_value_from_py(item)?);
    }
    ::core::result::Result::Ok(value)
}
