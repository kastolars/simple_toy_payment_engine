mod client_account_state;
mod precision;
mod io_data;
mod transaction_type;

use std::error::Error;
use std::{env, io, process};
use std::collections::HashMap;
use csv::{ReaderBuilder, Trim};
use io_data::{Output, Transaction};
use precision::convert_precision;
use crate::client_account_state::ClientAccountState;
use transaction_type::TransactionType::{CHARGEBACK, DEPOSIT, DISPUTE, RESOLVE, WITHDRAW};


extern crate csv;
#[macro_use]
extern crate serde_derive;


fn run() -> Result<(), Box<dyn Error>> {
    // Read the input filepath
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Must provide input file.");
        process::exit(1);
    }
    let input_filepath = &args[1]; // input file, ie. transactions.csv

    // Initialize the reader and record shape
    let mut rdr = ReaderBuilder::new().trim(Trim::All).from_path(input_filepath)?;
    let mut raw_record = csv::ByteRecord::new();
    let headers = rdr.byte_headers()?.clone();

    // Initialize our in-memory account states
    let mut client_account_states = HashMap::<u16, ClientAccountState>::new();

    // Iterate over each row one by one
    while rdr.read_byte_record(&mut raw_record)? { // Performance adjustments made following this: https://docs.rs/csv/latest/csv/tutorial/index.html#performance

        // Attempt a deserialization
        let record: Transaction = raw_record.deserialize(Some(&headers))?;

        // Fetch the client by id, or create the account state if it doesn't exist
        let state = client_account_states.entry(record.client).or_default();

        // In the case of locked accounts, we will skip the transaction
        if state.locked { continue; }

        // Match on each transaction type and pass on relevant data the corresponding state handler
        // Errors are ignored; failed transactions should not crash the system
        match record.r#type {
            DEPOSIT => { if let Some(amount) = record.amount { let _ = state.deposit(record.tx, amount); } }
            WITHDRAW => { if let Some(amount) = record.amount { let _ = state.withdraw(record.tx, amount); } }
            DISPUTE => { let _ = state.dispute(record.tx); }
            RESOLVE => { let _ = state.resolve(record.tx); }
            CHARGEBACK => { let _ = state.chargeback(record.tx); }
        }
    }

    // Initialize the output writer to std output
    // If a > $file argument is provided it should pipe std output to that file.
    let mut wtr = csv::Writer::from_writer(io::stdout());

    // Iterate over all accounts store in the state and write it to the file
    for (client_id, client_state) in &client_account_states {
        let available = convert_precision(client_state.available);
        let held = convert_precision(client_state.held);
        wtr.serialize(Output {
            client: *client_id,
            available,
            held,
            total: available + held,
            locked: client_state.locked,
        })?;
    }

    wtr.flush()?;
    Ok(())
}

#[allow(unused_must_use)]
fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        process::exit(1)
    }
}




