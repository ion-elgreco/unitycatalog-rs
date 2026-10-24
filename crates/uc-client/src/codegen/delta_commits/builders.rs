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
use uc_client::models::delta::v1::*;

/// Builder for delta
pub struct CommitDeltaBuilder {
    client: DeltaCommitClient,
    request: DeltaCommit,
}

impl CommitDeltaBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaCommitClient`.
    pub(crate) fn new(
        client: DeltaCommitClient,
        table_id: impl Into<String>,
        table_uri: impl Into<String>,
    ) -> Self {
        let request = DeltaCommit {
            table_id: table_id.into(),
            table_uri: table_uri.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The commit file to ratify.
    pub fn with_commit_info(mut self, commit_info: impl Into<Option<DeltaCommitInfo>>) -> Self {
        self.request.commit_info = {
            let commit_info: ::core::option::Option<_> = commit_info.into();
            buffa::MessageField::from(commit_info)
        };
        self
    }

    /// The commits up to this version are backfilled into `_delta_log`, so the server stops tracking
    /// them.
    pub fn with_latest_backfilled_version(
        mut self,
        latest_backfilled_version: impl Into<Option<i64>>,
    ) -> Self {
        self.request.latest_backfilled_version = latest_backfilled_version.into();
        self
    }

    /// A metadata change to apply with the commit. Not with a backfill report alone.
    pub fn with_metadata(mut self, metadata: impl Into<Option<DeltaMetadata>>) -> Self {
        self.request.metadata = {
            let metadata: ::core::option::Option<_> = metadata.into();
            buffa::MessageField::from(metadata)
        };
        self
    }

    /// UniForm metadata to commit with the commit.
    pub fn with_uniform(mut self, uniform: impl Into<Option<DeltaUniform>>) -> Self {
        self.request.uniform = {
            let uniform: ::core::option::Option<_> = uniform.into();
            buffa::MessageField::from(uniform)
        };
        self
    }
}

impl IntoFuture for CommitDeltaBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.commit_delta(&request).await })
    }
}

/// Builder for delta commits
pub struct GetDeltaCommitsBuilder {
    client: DeltaCommitClient,
    request: DeltaGetCommits,
}

impl GetDeltaCommitsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaCommitClient`.
    pub(crate) fn new(
        client: DeltaCommitClient,
        table_id: impl Into<String>,
        start_version: i64,
    ) -> Self {
        let request = DeltaGetCommits {
            table_id: table_id.into(),
            start_version,
            ..Default::default()
        };
        Self { client, request }
    }

    /// Storage location of the table.
    pub fn with_table_uri(mut self, table_uri: impl Into<Option<String>>) -> Self {
        self.request.table_uri = table_uri.into();
        self
    }

    /// Last version to return (inclusive). It does not change `latest_table_version`.
    pub fn with_end_version(mut self, end_version: impl Into<Option<i64>>) -> Self {
        self.request.end_version = end_version.into();
        self
    }
}

impl IntoFuture for GetDeltaCommitsBuilder {
    type Output = Result<DeltaGetCommitsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_delta_commits(&request).await })
    }
}
