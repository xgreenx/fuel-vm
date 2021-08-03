use crate::consts::*;
use crate::debug::Debugger;

use fuel_asm::{RegisterId, Word};
use fuel_tx::consts::*;
use fuel_tx::{Bytes32, Color, ContractId, Transaction};

use std::convert::TryFrom;
use std::mem;

mod alu;
mod blockchain;
mod contract;
mod crypto;
mod error;
mod executors;
mod flow;
mod frame;
mod gas;
mod log;
mod memory;

#[cfg(feature = "debug")]
mod debug;

pub use contract::Contract;
pub use error::ExecuteError;
pub use executors::{ProgramState, StateTransition, StateTransitionRef};
pub use frame::{Call, CallFrame};
pub use gas::GasUnit;
pub use log::LogEvent;
pub use memory::MemoryRange;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub enum Context {
    Predicate,
    Script,
    Call,
    NotInitialized,
}

impl Default for Context {
    fn default() -> Self {
        Self::NotInitialized
    }
}

impl Context {
    pub const fn is_external(&self) -> bool {
        match self {
            Self::Predicate | Self::Script => true,
            _ => false,
        }
    }
}

impl From<&Transaction> for Context {
    fn from(tx: &Transaction) -> Self {
        if tx.is_script() {
            Self::Script
        } else {
            Self::Predicate
        }
    }
}

#[derive(Debug, Clone)]
pub struct Interpreter<S> {
    registers: [Word; VM_REGISTER_COUNT],
    memory: Vec<u8>,
    frames: Vec<CallFrame>,
    log: Vec<LogEvent>,
    // TODO review all opcodes that mutates the tx in the stack and keep this one sync
    tx: Transaction,
    storage: S,
    debugger: Debugger,
    context: Context,
}

impl<S> Interpreter<S> {
    pub fn with_storage(storage: S) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory: vec![0; VM_MAX_RAM as usize],
            frames: vec![],
            log: vec![],
            tx: Transaction::default(),
            storage,
            debugger: Debugger::default(),
            context: Context::default(),
        }
    }

    pub(crate) fn push_stack(&mut self, data: &[u8]) -> Result<(), ExecuteError> {
        let (ssp, overflow) = self.registers[REG_SSP].overflowing_add(data.len() as Word);

        if overflow || !self.is_external_context() && ssp > self.registers[REG_FP] {
            Err(ExecuteError::StackOverflow)
        } else {
            self.memory[self.registers[REG_SSP] as usize..ssp as usize].copy_from_slice(data);
            self.registers[REG_SSP] = ssp;

            Ok(())
        }
    }

    pub const fn tx_mem_address() -> usize {
        Bytes32::size_of() // Tx ID
            + WORD_SIZE // Tx size
            + MAX_INPUTS as usize * (Color::size_of() + WORD_SIZE) // Color/Balance
                                                                   // coin input
                                                                   // pairs
    }

    pub(crate) const fn block_height(&self) -> u32 {
        // TODO fetch block height
        u32::MAX >> 1
    }

    pub(crate) fn set_flag(&mut self, a: Word) {
        self.registers[REG_FLAG] = a;
    }

    pub(crate) fn clear_err(&mut self) {
        self.registers[REG_ERR] = 0;
    }

    pub(crate) fn set_err(&mut self) {
        self.registers[REG_ERR] = 1;
    }

    pub(crate) fn inc_pc(&mut self) -> bool {
        let (result, overflow) = self.registers[REG_PC].overflowing_add(4);

        self.registers[REG_PC] = result;

        !overflow
    }

    pub fn memory(&self) -> &[u8] {
        self.memory.as_slice()
    }

    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    pub(crate) const fn context(&self) -> Context {
        if self.registers[REG_FP] == 0 {
            self.context
        } else {
            Context::Call
        }
    }

    pub(crate) const fn is_external_context(&self) -> bool {
        self.context().is_external()
    }

    pub(crate) const fn is_predicate(&self) -> bool {
        matches!(self.context, Context::Predicate)
    }

    // TODO convert to private scope after using internally
    pub const fn is_unsafe_math(&self) -> bool {
        self.registers[REG_FLAG] & 0x01 == 0x01
    }

    // TODO convert to private scope after using internally
    pub const fn is_wrapping(&self) -> bool {
        self.registers[REG_FLAG] & 0x02 == 0x02
    }

    pub(crate) const fn is_valid_register_alu(ra: RegisterId) -> bool {
        ra > REG_FLAG && ra < VM_REGISTER_COUNT
    }

    pub(crate) const fn is_valid_register_couple_alu(ra: RegisterId, rb: RegisterId) -> bool {
        ra > REG_FLAG && ra < VM_REGISTER_COUNT && rb < VM_REGISTER_COUNT
    }

    pub(crate) const fn is_valid_register_triple_alu(ra: RegisterId, rb: RegisterId, rc: RegisterId) -> bool {
        ra > REG_FLAG && ra < VM_REGISTER_COUNT && rb < VM_REGISTER_COUNT && rc < VM_REGISTER_COUNT
    }

    pub(crate) const fn is_valid_register_quadruple_alu(
        ra: RegisterId,
        rb: RegisterId,
        rc: RegisterId,
        rd: RegisterId,
    ) -> bool {
        ra > REG_FLAG
            && ra < VM_REGISTER_COUNT
            && rb < VM_REGISTER_COUNT
            && rc < VM_REGISTER_COUNT
            && rd < VM_REGISTER_COUNT
    }

    pub(crate) const fn is_valid_register_quadruple(
        ra: RegisterId,
        rb: RegisterId,
        rc: RegisterId,
        rd: RegisterId,
    ) -> bool {
        ra < VM_REGISTER_COUNT && rb < VM_REGISTER_COUNT && rc < VM_REGISTER_COUNT && rd < VM_REGISTER_COUNT
    }

    pub(crate) const fn is_valid_register_triple(ra: RegisterId, rb: RegisterId, rc: RegisterId) -> bool {
        ra < VM_REGISTER_COUNT && rb < VM_REGISTER_COUNT && rc < VM_REGISTER_COUNT
    }

    pub(crate) const fn is_valid_register_couple(ra: RegisterId, rb: RegisterId) -> bool {
        ra < VM_REGISTER_COUNT && rb < VM_REGISTER_COUNT
    }

    pub(crate) const fn is_valid_register(ra: RegisterId) -> bool {
        ra < VM_REGISTER_COUNT
    }

    pub(crate) const fn transaction(&self) -> &Transaction {
        &self.tx
    }

    pub(crate) fn internal_contract(&self) -> Result<ContractId, ExecuteError> {
        if self.is_external_context() {
            return Err(ExecuteError::ExpectedInternalContext);
        }

        let c = self.registers[REG_FP] as usize;
        let cx = c + ContractId::size_of();
        let contract = ContractId::try_from(&self.memory[c..cx]).expect("Memory bounds logically verified");

        Ok(contract)
    }

    pub fn log(&self) -> &[LogEvent] {
        self.log.as_slice()
    }
}

impl<S> From<Interpreter<S>> for Transaction {
    fn from(vm: Interpreter<S>) -> Self {
        vm.tx
    }
}
