//! [`PyFuture`]: what the `name_async` methods return.

use std::future::Future;
use std::marker::PhantomData;

use pyo3::prelude::*;
use pyo3_stub_gen::{PyStubType, TypeInfo};

/// The `asyncio.Future` of a `T`, from `future_into_py`. `T` is the type of
/// its result; the stubs show it as `asyncio.Future[T]`.
pub struct PyFuture<'py, T> {
    future: Bound<'py, PyAny>,
    result: PhantomData<T>,
}

impl<'py, T> PyFuture<'py, T>
where
    T: for<'a> IntoPyObject<'a> + Send + 'static,
{
    /// Run `future` on the Tokio runtime; its result completes the
    /// `asyncio.Future`. It needs a running event loop.
    pub fn new(
        py: Python<'py>,
        future: impl Future<Output = PyResult<T>> + Send + 'static,
    ) -> PyResult<Self> {
        Ok(Self {
            future: pyo3_async_runtimes::tokio::future_into_py(py, future)?,
            result: PhantomData,
        })
    }
}

impl<'py, T> IntoPyObject<'py> for PyFuture<'py, T> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, _py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.future)
    }
}

/// The result of a request that returns nothing: Python's `None`. A `()`
/// result would give an empty tuple.
pub struct NoResult;

impl<'py> IntoPyObject<'py> for NoResult {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(py.None().into_bound(py))
    }
}

impl PyStubType for NoResult {
    fn type_output() -> TypeInfo {
        TypeInfo::none()
    }
}

impl<T: PyStubType> PyStubType for PyFuture<'_, T> {
    fn type_output() -> TypeInfo {
        let result = T::type_output();
        let mut import = result.import;
        import.insert("asyncio".into());
        TypeInfo {
            name: format!("asyncio.Future[{}]", result.name),
            source_module: None,
            import,
            type_refs: result.type_refs,
        }
    }
}
