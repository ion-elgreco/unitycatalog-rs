pub(crate) mod sql;

use async_trait::async_trait;

use uc_api::delta_api::{DeltaCreateTableRequest, DeltaLoadTableResponse, DeltaUpdateTableRequest};
use uc_api::{DeltaCommit, DeltaGetCommits, DeltaGetCommitsResponse};
use uc_common::Result;
use uc_delta::commits::{CommitOutcome, StoredCommit};

/// Tables through the Delta API, and the commit coordinator of MANAGED
/// Delta tables. The rules are in [`uc_delta`]; this trait applies them to
/// the stored tables and commits.
#[async_trait]
pub trait DeltaTableOps: Send + Sync {
    /// Upstream `DeltaCommitRepository.postCommit`: the commit coordinator
    /// for MANAGED Delta tables. See [`uc_delta::commits`].
    async fn commit(&self, commit: DeltaCommit, updated_by: Option<&str>) -> Result<CommitOutcome>;

    /// Upstream `DeltaCommitRepository.getCommits`.
    async fn get_commits(&self, req: DeltaGetCommits) -> Result<DeltaGetCommitsResponse>;

    /// Upstream `getUnbackfilledCommits`: the unbackfilled commits of a
    /// table, newest first, and its latest version (0 before the first
    /// commit).
    async fn unbackfilled_commits(&self, table_id: &str) -> Result<(Vec<StoredCommit>, i64)>;

    /// Upstream `createTableForDelta`: map the Delta API request (with the
    /// managed-table settings), create, and load.
    async fn create_table(
        &self,
        catalog: &str,
        schema: &str,
        req: &DeltaCreateTableRequest,
        created_by: Option<&str>,
    ) -> Result<DeltaLoadTableResponse>;

    /// Upstream `loadTableForDelta`.
    async fn load_table(&self, full_name: &str) -> Result<DeltaLoadTableResponse>;

    /// Upstream `updateTableForDelta`: a Delta API table update. The caller
    /// loads the table for the answer.
    async fn update_table(
        &self,
        full_name: &str,
        req: DeltaUpdateTableRequest,
        updated_by: Option<&str>,
    ) -> Result<CommitOutcome>;
}
