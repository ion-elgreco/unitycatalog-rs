//! `unitycatalog._core.identity`: the token sources of cloud identities, made in Rust
//! (`uc_client::identity`), for `Client(url, token_provider=...)`. `Client(url)` without a
//! token calls `detect` itself. The generated extension module exports [`identity`] as its
//! submodule (`submodules` of the generation config, in `crates/uc-codegen`).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use uc_client::identity as sources;
use uc_client::transport::TokenError;

use crate::codegen::RustTokenSource;
use crate::codegen::error::PyUnityCatalogError;

/// The token sources of cloud identities, for `Client(url, token_provider=...)`.
#[pymodule]
pub mod identity {
    #[pymodule_export]
    use super::{aws, azure, client_credentials, detect, google, token_env, token_file};
}

fn error(e: TokenError) -> PyErr {
    PyUnityCatalogError::from(uc_client::error::Error::Token(e)).into()
}

fn source(
    source: sources::Result<std::sync::Arc<dyn uc_client::transport::TokenSource>>,
) -> PyResult<RustTokenSource> {
    Ok(RustTokenSource {
        source: source.map_err(error)?,
    })
}

/// An Azure identity of the environment, for the scope `scope`, such as `api://uc/.default`: a
/// service principal (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`), a workload
/// identity (`AZURE_FEDERATED_TOKEN_FILE`), the sign-in of the Azure CLI when `az` is on the
/// path, else the managed identity of the machine or pod.
#[pyfunction]
fn azure(scope: &str) -> PyResult<RustTokenSource> {
    source(sources::azure(scope))
}

/// A Google identity of the environment, as an ID token for `audience`: the service-account
/// key file of `GOOGLE_APPLICATION_CREDENTIALS`, the sign-in of the Google Cloud CLI when
/// `gcloud` is on the path, else the metadata server of the machine or pod.
#[pyfunction]
fn google(audience: &str) -> PyResult<RustTokenSource> {
    source(sources::google(audience))
}

/// An AWS identity of the environment, for the server whose AWS issuer block has the audience
/// `server_id`, such as `https://uc.example.com`. Each token is a signed `GetCallerIdentity`
/// request, which the server sends to STS: the credentials stay here. They are the access keys
/// of `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` (with `AWS_SESSION_TOKEN`), a web identity
/// (`AWS_ROLE_ARN`, `AWS_WEB_IDENTITY_TOKEN_FILE`), the container's credentials, else the
/// instance role of the machine.
#[pyfunction]
fn aws(server_id: &str) -> PyResult<RustTokenSource> {
    source(sources::aws(server_id))
}

/// An OAuth client of any provider, with the client-credentials grant at `token_url`.
#[pyfunction]
#[pyo3(signature = (token_url, client_id, client_secret, *, scope = None, audience = None))]
fn client_credentials(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scope: Option<&str>,
    audience: Option<&str>,
) -> PyResult<RustTokenSource> {
    source(sources::client_credentials(
        token_url,
        client_id,
        client_secret,
        scope,
        audience,
    ))
}

/// The token in the file `path`, read again at each refresh: a projected Kubernetes
/// service-account token, for one.
#[pyfunction]
fn token_file(path: &str) -> RustTokenSource {
    RustTokenSource {
        source: sources::token_file(path),
    }
}

/// The token in the environment variable `var`, read again at each refresh.
#[pyfunction]
fn token_env(var: &str) -> RustTokenSource {
    RustTokenSource {
        source: sources::token_env(var),
    }
}

/// The token source of the environment for the server at `base_url`, if it has one: `UC_TOKEN`
/// or `UC_TOKEN_FILE`, an OAuth client in `UC_OAUTH_*`, else an Azure, Google or AWS identity at
/// one of the server's issuers. `Client(base_url)` without a token does this itself.
#[pyfunction]
fn detect(py: Python<'_>, base_url: &str) -> PyResult<Option<RustTokenSource>> {
    let url: url::Url = base_url
        .parse()
        .map_err(|e: url::ParseError| PyValueError::new_err(e.to_string()))?;
    let source = py
        .detach(|| crate::codegen::runtime::rt().block_on(sources::detect(&url)))
        .map_err(error)?;
    Ok(source.map(|source| RustTokenSource { source }))
}
