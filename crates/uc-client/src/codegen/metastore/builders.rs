// @generated — do not edit by hand.
#![allow(unused_mut)]
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
type BoxFut<'a, T> = ::futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxFut<'a, T> = ::futures::future::LocalBoxFuture<'a, T>;
use super::client::*;
use crate::codegen::error::Result;
use std::future::IntoFuture;
use uc_client::models::metastore::v1::*;

/// Builder for metastore summary
pub struct GetMetastoreSummaryBuilder {
    client: MetastoreClient,
    request: GetMetastoreSummaryRequest,
}

impl GetMetastoreSummaryBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `MetastoreClient`.
    pub(crate) fn new(client: MetastoreClient) -> Self {
        let request = GetMetastoreSummaryRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetMetastoreSummaryBuilder {
    type Output = Result<GetMetastoreSummaryResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_metastore_summary(&request).await })
    }
}
