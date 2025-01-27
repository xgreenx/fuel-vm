#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::try_err)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

// TODO Add docs

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod consts;
mod tx_pointer;

pub use fuel_asm::{InstructionResult, PanicReason};
pub use fuel_types::{Address, AssetId, Bytes32, Bytes4, Bytes64, Bytes8, ContractId, MessageId, Salt, Word};
pub use tx_pointer::TxPointer;

#[cfg(feature = "builder")]
mod builder;

#[cfg(feature = "alloc")]
mod contract;

#[cfg(feature = "alloc")]
mod receipt;

#[cfg(feature = "alloc")]
mod transaction;

#[cfg(feature = "builder")]
pub use builder::{Buildable, Finalizable, TransactionBuilder};

#[cfg(feature = "alloc")]
pub use receipt::{Receipt, ScriptExecutionResult};

#[cfg(feature = "alloc")]
pub use transaction::{
    field, Cacheable, Chargeable, CheckError, ConsensusParameters, Create, Executable, FormatValidityChecks, Input,
    InputRepr, Mint, Output, OutputRepr, Script, StorageSlot, Transaction, TransactionFee, TransactionRepr, TxId,
    UtxoId, Witness,
};

#[cfg(feature = "std")]
pub use transaction::{Signable, UniqueIdentifier};

#[cfg(feature = "alloc")]
#[allow(deprecated)]
pub use transaction::consensus_parameters::default_parameters;

#[cfg(feature = "alloc")]
pub use contract::Contract;
