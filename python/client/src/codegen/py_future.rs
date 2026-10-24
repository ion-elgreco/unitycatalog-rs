// @generated — do not edit by hand.
//! What the `_async` methods return.

use pyo3::prelude::*;
use std::future::Future;
use std::marker::PhantomData;

/// The `asyncio.Future` of a `T`, from `future_into_py`.
pub struct PyFuture<'py, T> {
    future: Bound<'py, PyAny>,
    result: PhantomData<T>,
}

impl<'py, T> PyFuture<'py, T>
where
    T: for<'a> IntoPyObject<'a> + Send + 'static,
{
    /// Run `future` on the Tokio runtime of `pyo3-async-runtimes`; its result completes
    /// the `asyncio.Future`. It needs a running event loop.
    ///
    /// It first calls `crate::codegen::runtime::rt`, so that a call in a forked child fails
    /// as a sync call does: `future_into_py` spawns on the installed runtime without that
    /// check.
    pub fn new(
        py: Python<'py>,
        future: impl Future<Output = PyResult<T>> + Send + 'static,
    ) -> PyResult<Self> {
        crate::codegen::runtime::rt();
        Ok(Self {
            future: ::pyo3_async_runtimes::tokio::future_into_py(py, future)?,
            result: PhantomData,
        })
    }
}

impl<'py, T> IntoPyObject<'py> for PyFuture<'py, T> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = ::std::convert::Infallible;
    fn into_pyobject(self, _py: Python<'py>) -> ::core::result::Result<Self::Output, Self::Error> {
        Ok(self.future)
    }
}

/// The result of a request that returns nothing: Python's `None`. A `()` result would
/// give an empty tuple.
pub struct NoResult;
impl<'py> IntoPyObject<'py> for NoResult {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = ::std::convert::Infallible;
    fn into_pyobject(self, py: Python<'py>) -> ::core::result::Result<Self::Output, Self::Error> {
        Ok(py.None().into_bound(py))
    }
}

impl ::pyo3_stub_gen::PyStubType for NoResult {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        ::pyo3_stub_gen::TypeInfo::none()
    }
}

impl<T: ::pyo3_stub_gen::PyStubType> ::pyo3_stub_gen::PyStubType for PyFuture<'_, T> {
    fn type_output() -> ::pyo3_stub_gen::TypeInfo {
        let result = T::type_output();
        let mut import = result.import;
        import.insert("asyncio".into());
        ::pyo3_stub_gen::TypeInfo {
            name: ::std::format!("asyncio.Future[{}]", result.name),
            source_module: None,
            import,
            type_refs: result.type_refs,
        }
    }
}
