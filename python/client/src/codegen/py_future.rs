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
    /// the `asyncio.Future`. Needs a running event loop.
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

impl<'py, T: IntoPyObject<'py>> IntoPyObject<'py> for PyFuture<'py, T> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = ::std::convert::Infallible;
    const OUTPUT_TYPE: ::pyo3::inspect::PyStaticExpr = ::pyo3::type_hint_subscript!(
        ::pyo3::type_hint_identifier!("asyncio", "Future"),
        T::OUTPUT_TYPE
    );
    fn into_pyobject(self, _py: Python<'py>) -> ::core::result::Result<Self::Output, Self::Error> {
        Ok(self.future)
    }
}

/// Python's `None`, for a request that returns nothing (`()` would be an empty tuple).
pub struct NoResult;
impl<'py> IntoPyObject<'py> for NoResult {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = ::std::convert::Infallible;
    const OUTPUT_TYPE: ::pyo3::inspect::PyStaticExpr =
        <::pyo3::types::PyNone as ::pyo3::PyTypeInfo>::TYPE_HINT;
    fn into_pyobject(self, py: Python<'py>) -> ::core::result::Result<Self::Output, Self::Error> {
        Ok(py.None().into_bound(py))
    }
}
