//! The rules of catalog-managed Delta tables, without storage: the commit
//! coordinator's plan, the UC table contract, the Delta API's view of a
//! table, its table updates, and UniForm. The metastore applies them in
//! its transactions; the server uses them to check requests.

pub mod commits;
pub mod contract;
pub mod table;
pub mod uniform;
pub mod update;
