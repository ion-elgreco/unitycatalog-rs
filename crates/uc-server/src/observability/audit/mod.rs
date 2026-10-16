//! The audit log: a layer records one event per mutating request, and a
//! sink writes the events to the metastore.

pub mod layer;
pub mod sink;
