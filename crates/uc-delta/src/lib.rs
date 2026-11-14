//! The rules of catalog-managed Delta tables, without storage: the commit
//! coordinator's plan, the UC table contract, the Delta API's view of a
//! table, its table updates, and UniForm. The metastore applies them in
//! its transactions; the server uses them to check requests.

pub mod column_mapping;
pub mod commits;
pub mod contract;
pub mod schema_evolution;
pub mod table;
pub mod uniform;
mod unsupported_changes;
pub mod update;
