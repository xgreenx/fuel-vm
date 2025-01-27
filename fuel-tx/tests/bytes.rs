use fuel_asm::op;
use fuel_tx::*;
use fuel_tx_test_helpers::{generate_bytes, generate_nonempty_padded_bytes};
use fuel_types::{bytes, Immediate24};
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

use fuel_tx::field::{Inputs, Script, ScriptData};
use std::fmt;
use std::io::{self, Read, Write};

pub fn assert_encoding_correct<'a, T>(data: &[T])
where
    T: Read
        + Write
        + fmt::Debug
        + Clone
        + PartialEq
        + bytes::SizedBytes
        + bytes::SerializableVec
        + bytes::Deserializable
        + serde::Serialize
        + serde::Deserialize<'a>,
{
    let mut buffer;

    for data in data.iter() {
        let d_s = bincode::serialize(&data).expect("Failed to serialize data");
        // Safety: bincode/serde fails to understand the elision so this is a cheap way to convince it
        let d_s: T =
            bincode::deserialize(unsafe { std::mem::transmute(d_s.as_slice()) }).expect("Failed to deserialize data");

        assert_eq!(&d_s, data);

        let mut d = data.clone();

        let d_bytes = data.clone().to_bytes();
        let d_p = T::from_bytes(d_bytes.as_slice()).expect("Failed to deserialize T");

        assert_eq!(d, d_p);

        let mut d_p = data.clone();

        buffer = vec![0u8; 2048];
        let read_size = d.read(buffer.as_mut_slice()).expect("Failed to read");
        let write_size = d_p.write(buffer.as_slice()).expect("Failed to write");

        // Simple RW assertion
        assert_eq!(d, d_p);
        assert_eq!(read_size, write_size);

        buffer = vec![0u8; read_size];

        // Minimum size buffer assertion
        let _ = d.read(buffer.as_mut_slice()).expect("Failed to read");
        let _ = d_p.write(buffer.as_slice()).expect("Failed to write");
        assert_eq!(d, d_p);
        assert_eq!(d_bytes.as_slice(), buffer.as_slice());

        // No panic assertion
        loop {
            buffer.pop();

            let err = d
                .read(buffer.as_mut_slice())
                .expect_err("Insufficient buffer should fail!");
            assert_eq!(io::ErrorKind::UnexpectedEof, err.kind());

            let err = d_p
                .write(buffer.as_slice())
                .expect_err("Insufficient buffer should fail!");
            assert_eq!(io::ErrorKind::UnexpectedEof, err.kind());

            if buffer.is_empty() {
                break;
            }
        }
    }
}

#[test]
fn witness() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let w = generate_bytes(rng).into();

    assert_encoding_correct(&[w, Witness::default()]);
}

#[test]
fn input() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Input::coin_signed(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
        ),
        Input::coin_predicate(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
        Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Input::message_signed(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            generate_bytes(rng),
        ),
        Input::message_predicate(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            generate_bytes(rng),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
    ]);
}

#[test]
fn output() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Output::coin(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract(rng.gen(), rng.gen(), rng.gen()),
        Output::message(rng.gen(), rng.next_u64()),
        Output::change(rng.gen(), rng.next_u64(), rng.gen()),
        Output::variable(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract_created(rng.gen(), rng.gen()),
    ]);
}

#[test]
fn receipt() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Receipt::call(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::ret(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::return_data(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![rng.gen(), rng.gen()],
            rng.gen(),
            rng.gen(),
        ),
        Receipt::revert(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::log(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::log_data(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![rng.gen(), rng.gen()],
            rng.gen(),
            rng.gen(),
        ),
        Receipt::transfer(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::transfer_out(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(PanicReason::Success, op::noop().into()),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(PanicReason::Revert, op::ji(rng.gen::<Immediate24>() & 0xffffff).into()),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::OutOfGas,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::TransactionValidity,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MemoryOverflow,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ArithmeticOverflow,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ContractNotFound,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MemoryOwnership,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::NotEnoughBalance,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedInternalContext,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::AssetIdNotFound,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InputNotFound,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::OutputNotFound,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::WitnessNotFound,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::TransactionMaturity,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InvalidMetadataIdentifier,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MalformedCallStructure,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ReservedRegisterNotWritable,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ErrorFlag,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InvalidImmediateValue,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedCoinInput,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MaxMemoryAccess,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MemoryWriteOverlap,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ContractNotInInputs,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InternalBalanceOverflow,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ContractMaxSize,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedUnallocatedStack,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::TransferAmountCannotBeZero,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedOutputVariable,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedParentInternalContext,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ContractNotInInputs,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        )
        .with_panic_contract_id(Some(rng.gen())),
        Receipt::script_result(ScriptExecutionResult::Success, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::Panic, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::Revert, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::GenericFailure(rng.gen()), rng.gen()),
        Receipt::message_out(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![rng.gen()],
        ),
    ]);
}

#[test]
fn transaction() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let i = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen());
    let o = Output::coin(rng.gen(), rng.next_u64(), rng.gen());
    let w = rng.gen::<Witness>();
    let s = rng.gen::<StorageSlot>();

    assert_encoding_correct(&[
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            generate_bytes(rng),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            vec![],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![s.clone()],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![s],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![i],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![w],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[
        Transaction::mint(rng.gen(), vec![o]),
        Transaction::mint(rng.gen(), vec![o, o]),
        Transaction::mint(rng.gen(), vec![o, o, o]),
        Transaction::mint(rng.gen(), vec![o, o, o, o]),
        Transaction::mint(rng.gen(), vec![o, o, o, o, o]),
        Transaction::mint(rng.gen(), vec![o, o, o, o, o, o]),
    ]);
}

#[test]
fn create_input_data_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10;
    let bytecode_witness_index = 0x00;
    let salt = rng.gen();

    let storage_slots: Vec<Vec<StorageSlot>> = vec![vec![], vec![rng.gen()], vec![rng.gen(), rng.gen()]];
    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen())],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()); 2],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![
            Output::contract(rng.gen(), rng.gen(), rng.gen()),
            Output::message(rng.gen(), rng.next_u64()),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![generate_bytes(rng).into()],
        vec![generate_bytes(rng).into(), generate_bytes(rng).into()],
    ];

    let predicate = generate_nonempty_padded_bytes(rng);
    let predicate_data = generate_bytes(rng);

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let data = generate_bytes(rng);
    let input_message = Input::message_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        data,
        predicate.clone(),
        predicate_data,
    );

    let mut buffer = vec![0u8; 4096];
    for storage_slot in storage_slots.iter() {
        for inputs in inputs.iter() {
            for outputs in outputs.iter() {
                for witnesses in witnesses.iter() {
                    let mut inputs = inputs.clone();

                    let input_coin_idx = inputs.len();
                    inputs.push(input_coin.clone());

                    let input_message_idx = inputs.len();
                    inputs.push(input_message.clone());

                    let mut tx = Transaction::create(
                        gas_price,
                        gas_limit,
                        maturity,
                        bytecode_witness_index,
                        salt,
                        storage_slot.clone(),
                        inputs,
                        outputs.clone(),
                        witnesses.clone(),
                    );

                    let mut tx_p = tx.clone();
                    tx_p.precompute();

                    buffer.iter_mut().for_each(|b| *b = 0x00);
                    let _ = tx.read(buffer.as_mut_slice()).expect("Failed to serialize input");

                    let (offset, len) = tx
                        .inputs_predicate_offset_at(input_coin_idx)
                        .expect("Failed to fetch offset");

                    let (offset_p, _) = tx_p
                        .inputs_predicate_offset_at(input_coin_idx)
                        .expect("Failed to fetch offset from tx with precomputed metadata!");

                    assert_eq!(offset, offset_p);
                    assert_eq!(predicate.as_slice(), &buffer[offset..offset + len][..predicate.len()]);

                    let (offset, len) = tx
                        .inputs_predicate_offset_at(input_message_idx)
                        .expect("Failed to fetch offset");

                    let (offset_p, _) = tx_p
                        .inputs_predicate_offset_at(input_message_idx)
                        .expect("Failed to fetch offset from tx with precomputed metadata!");

                    assert_eq!(offset, offset_p);
                    assert_eq!(predicate.as_slice(), &buffer[offset..offset + len][..predicate.len()]);
                }
            }
        }
    }
}

#[test]
fn script_input_coin_data_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10;

    let script: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];
    let script_data: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen())],
        vec![
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![
            Output::contract(rng.gen(), rng.gen(), rng.gen()),
            Output::message(rng.gen(), rng.next_u64()),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![generate_bytes(rng).into()],
        vec![generate_bytes(rng).into(), generate_bytes(rng).into()],
    ];

    let mut predicate = generate_nonempty_padded_bytes(rng);

    // force word-unaligned predicate
    if predicate.len() % 2 == 0 {
        predicate.push(0xff);
    }

    let predicate_data = generate_bytes(rng);

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate.clone(),
        predicate_data,
    );

    let mut buffer = vec![0u8; 4096];
    for script in script.iter() {
        for script_data in script_data.iter() {
            for inputs in inputs.iter() {
                for outputs in outputs.iter() {
                    for witnesses in witnesses.iter() {
                        let mut inputs = inputs.clone();
                        let offset = inputs.len();
                        inputs.push(input_coin.clone());

                        let mut tx = Transaction::script(
                            gas_price,
                            gas_limit,
                            maturity,
                            script.clone(),
                            script_data.clone(),
                            inputs,
                            outputs.clone(),
                            witnesses.clone(),
                        );

                        let mut tx_p = tx.clone();
                        tx_p.precompute();

                        buffer.iter_mut().for_each(|b| *b = 0x00);

                        let _ = tx.read(buffer.as_mut_slice()).expect("Failed to serialize input");

                        let script_offset = tx.script_offset();
                        assert_eq!(script.as_slice(), &buffer[script_offset..script_offset + script.len()]);

                        let script_data_offset = tx.script_data_offset();

                        let script_data_offset_p = tx_p.script_data_offset();

                        assert_eq!(script_data_offset, script_data_offset_p);
                        assert_eq!(
                            script_data.as_slice(),
                            &buffer[script_data_offset..script_data_offset + script_data.len()]
                        );

                        let (offset, len) = tx.inputs_predicate_offset_at(offset).expect("Failed to fetch offset");

                        assert_ne!(bytes::padded_len(&predicate), predicate.len());
                        assert_eq!(bytes::padded_len(&predicate), len);

                        assert_eq!(predicate.as_slice(), &buffer[offset..offset + predicate.len()]);
                    }
                }
            }
        }
    }
}
