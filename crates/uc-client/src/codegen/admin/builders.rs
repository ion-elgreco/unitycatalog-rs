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
use uc_client::models::admin::v1::*;

/// Builder for backup
pub struct GetBackupBuilder {
    client: AdminClient,
    request: GetBackupRequest,
}

impl GetBackupBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `AdminClient`.
    pub(crate) fn new(client: AdminClient) -> Self {
        let request = GetBackupRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetBackupBuilder {
    type Output = Result<BackupResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_backup(&request).await })
    }
}

/// Builder for backup
pub struct RestoreBackupBuilder {
    client: AdminClient,
    request: BackupResponse,
}

impl RestoreBackupBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `AdminClient`.
    pub(crate) fn new(
        client: AdminClient,
        version: i64,
        tables: impl IntoIterator<
            Item = (
                impl Into<String>,
                ::buffa_types::google::protobuf::ListValue,
            ),
        >,
    ) -> Self {
        let request = BackupResponse {
            version,
            tables: tables.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for RestoreBackupBuilder {
    type Output = Result<RestoreResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.restore_backup(&request).await })
    }
}
