//! Model Switchboard: catalog + routing + gateway transform.
//! Split: catalog.rs gateway.rs budget.rs ledger.rs adapters/ transform/.
pub mod adapters;
pub mod budget;
pub mod catalog;
pub mod gateway;
pub mod ledger;
pub mod port;
