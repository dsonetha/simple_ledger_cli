use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::env;
use std::fs::File;
use std::process;
use std::ffi::OsString;
use std::io;

mod structs;
use crate::structs::*;

mod utils;

// Read the CSV and loop through each line to process transactions
fn handle_csv() -> Result<(), Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut raw_record = csv::ByteRecord::new();
    let mut headers = rdr.byte_headers()?.clone();
    headers.trim();
    // mapping client with their ids to facilitate account and operation management
    let mut clients_map: HashMap<u16, Client> = HashMap::new();
    // store tx ids to avoid duplicates
    let mut tx_set: HashSet<u32> = HashSet::new();
    // locked accounts behaviour
    let should_block_locked = match env::var("BLOCK_LOCKED_ACCOUNTS") {
        Ok(val) => val == "true",
        _ => false
    };

    while rdr.read_byte_record(&mut raw_record)? {
        raw_record.trim();
        let record: Record = raw_record.deserialize(Some(&headers))?;

        // ignore invalid records or transaction already handled
        if !record.is_valid() {
            continue;
        }
        if record.r#type == Operation::Deposit || record.r#type == Operation::Withdrawal {
            if tx_set.contains(&record.tx) {
                continue;
            } else {
                tx_set.insert(record.tx);
            }
        }

        // handle record and associated client/operation
        // if no client found for the record and if the operation is a new deposit, create a client
        if let Some(client) = clients_map.get_mut(&record.client) {
            client.handle_record(&record, should_block_locked);
        } else if record.r#type == Operation::Deposit {
            let client = Client::from_record(&record)?;
            clients_map.insert(record.client, client);
        }
    }
    output_clients(clients_map.into_values().collect())
}

// Output result, client accounts, as CSV
fn output_clients(clients: Vec<Client>) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_writer(io::stdout());

    // When writing records with Serde using structs, the header row is written automatically.
    for c in clients {
        wtr.serialize(c)?;
    }

    wtr.flush()?;
    Ok(())
}

// Returns the first positional argument sent to this process. If there are no positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    match handle_csv() {
        Ok(_) => {},
        Err(err) => {
            println!("{}", err);
            process::exit(1);
        }
    }
}
