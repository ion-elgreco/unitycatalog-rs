pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::delta_api::{DeltaCreateTableRequest, DeltaLoadTableResponse, DeltaUpdateTableRequest};
use uc_api::{DeltaCommit, DeltaGetCommits, DeltaGetCommitsResponse};
use uc_common::Result;
use uc_delta::commits::CommitOutcome;

/// Tables through the Delta API, and the commit coordinator of MANAGED
/// Delta tables. The rules are in [`uc_delta`]; this trait applies them to
/// the stored tables and commits.
#[async_trait]
pub trait DeltaTableOps: Send + Sync {
    /// Upstream `DeltaCommitRepository.postCommit`: the commit coordinator
    /// for MANAGED Delta tables. See [`uc_delta::commits`]. `files_checked`
    /// says that the caller saw the published files that an earlier
    /// [`CommitOutcome::VerifyPublished`] asked for.
    async fn commit(
        &self,
        commit: &DeltaCommit,
        updated_by: Option<&str>,
        files_checked: bool,
    ) -> Result<CommitOutcome>;

    /// Upstream `DeltaCommitRepository.getCommits`.
    async fn get_commits(&self, req: DeltaGetCommits) -> Result<DeltaGetCommitsResponse>;

    /// Upstream `createTableForDelta`: map the Delta API request (with the
    /// managed-table settings), create, and load.
    async fn create_table(
        &self,
        catalog: &str,
        schema: &str,
        req: &DeltaCreateTableRequest,
        created_by: Option<&str>,
    ) -> Result<DeltaLoadTableResponse>;

    /// Upstream `deleteTableForDelta`: drop a table, refusing one that is
    /// not Delta.
    async fn delete_table(&self, full_name: &str) -> Result<()>;

    /// Upstream `renameTableForDelta`: rename a table, refusing one that is
    /// not Delta.
    async fn rename_table(
        &self,
        full_name: &str,
        new_name: &str,
        updated_by: Option<&str>,
    ) -> Result<()>;

    /// Upstream `loadTableForDelta`.
    async fn load_table(&self, full_name: &str) -> Result<DeltaLoadTableResponse>;

    /// Upstream `updateTableForDelta`: a Delta API table update. The caller
    /// loads the table for the answer. `files_checked` as for
    /// [`Self::commit`].
    async fn update_table(
        &self,
        full_name: &str,
        req: &DeltaUpdateTableRequest,
        updated_by: Option<&str>,
        files_checked: bool,
    ) -> Result<CommitOutcome>;
}
