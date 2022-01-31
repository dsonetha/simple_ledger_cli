
use std::fmt;
use crate::Error;
use serde::{Serialize, Deserialize};
use crate::utils::round_serialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
// represents an operation type for a given transaction
pub enum Operation {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback
}

impl Operation {
    fn as_str(&self) -> &'static str {
        match self {
            Operation::Deposit => "deposit",
            Operation::Withdrawal => "withdrawal",
            Operation::Dispute => "dispute",
            Operation::Resolve => "resolve",
            Operation::Chargeback => "chargeback"
        }
    }
}

#[derive(Debug, Deserialize)]
// A CSV line representing a transaction
pub struct Record {
    pub r#type: Operation,
    pub client: u16,
    pub tx: u32,
    amount: Option<f64>,
}

impl Record {
    pub fn is_valid(&self) -> bool {
        if self.amount.is_none() && (self.r#type == Operation::Deposit || self.r#type == Operation::Withdrawal) {
            return false;
        } else if let Some(amount) = self.amount {
            return amount > 0.0;
        }
        true
    }
}

#[derive(Debug, Serialize)]
pub struct ClientDeposit {
    tx: u32,
    amount: f64,
    is_disputed: bool,
    is_dispute_handled: bool
}

impl ClientDeposit {
    pub fn from_record(record: &Record) -> ClientDeposit {
        ClientDeposit { tx: record.tx, amount: record.amount.expect("Expecting an amount from record"), is_disputed: false, is_dispute_handled: false }
    }
}

#[derive(Debug, Serialize)]
// Represents a client account, we store client deposits to handle disputes
// All the transactions are handled through a Client, you should avoid handling record without Client::handle_record
// Records must be valid and we assume that they have been checked before, see Record::is_valid
pub struct Client {
    #[serde(rename = "client")]
    id: u16,
    #[serde(skip_serializing)]
    deposits: Vec<ClientDeposit>,
    #[serde(serialize_with = "round_serialize")]
    available: f64,
    #[serde(serialize_with = "round_serialize")]
    held: f64,
    #[serde(serialize_with = "round_serialize")]
    total: f64,
    locked: bool
}

// As seen above for all the methods here we assume that the records are valid
impl Client {
    pub fn from_record(record: &Record) -> Result<Client, Box<dyn Error>> {
        match &record.r#type {
            Operation::Deposit => {
                let total = record.amount.expect("Expecting an amount from record");
                Ok(Client {
                    id: record.client,
                    available: total,
                    held: 0.0,
                    total,
                    locked: false,
                    deposits: vec![ClientDeposit::from_record(record)]
                })
            }
            op => Err(Box::new(BadRecordForClientCreation { operation: op.clone() }))
        }
    }

    fn deposit_amount(&mut self, amount: f64) {
        self.available += amount;
        self.total += amount;
    }
    fn withdraw_amount(&mut self, amount: f64) {
        if amount <= self.available {
            self.available -= amount;
            self.total -= amount;
        }
    }
    fn new_dispute(&mut self, tx: u32) {
        if let Some(deposit) = self.deposits.iter_mut().find(|d| d.tx == tx) {
            if deposit.is_disputed || deposit.is_dispute_handled {
                return
            }
            if self.available >= deposit.amount {
                deposit.is_disputed = true;
                self.available -= deposit.amount;
                self.held += deposit.amount;
            }
        }
    }
    // resolves and chargebacks are handled here
    fn resolve_dispute(&mut self, tx: u32, is_chargeback: bool) {
        if let Some(deposit) = self.deposits.iter_mut().find(|d| d.tx == tx) {
            if !deposit.is_disputed || deposit.is_dispute_handled {
                return
            }
            deposit.is_dispute_handled = true;
            self.held -= deposit.amount;

            if !is_chargeback {
                self.available += deposit.amount;
            } else {
                self.total -= deposit.amount;
                self.locked = true;
            }
        }
    }

    // handle record and process transaction based on the operation
    pub fn handle_record(&mut self, record: &Record, should_block_locked: bool) {
        if should_block_locked && self.locked {
            return
        }

        match &record.r#type {
            Operation::Deposit => {
                let amount = record.amount.expect("Expecting an amount from record");
                self.deposit_amount(amount);
                self.deposits.push(ClientDeposit::from_record(record));
            }
            Operation::Withdrawal => {
                let amount = record.amount.expect("Expecting an amount from record");
                self.withdraw_amount(amount);
            }
            Operation::Dispute => self.new_dispute(record.tx),
            Operation::Resolve => self.resolve_dispute(record.tx, false),
            Operation::Chargeback => self.resolve_dispute(record.tx, true),
        }
    }
}

#[derive(Debug)]
struct BadRecordForClientCreation {
    operation: Operation
}

impl Error for BadRecordForClientCreation {}
impl fmt::Display for BadRecordForClientCreation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid record for client creation: {}", self.operation.as_str())
    }
}

#[test]
fn record_validity() {
    // a simple deposit record
    let mut r = Record {
        r#type: Operation::Deposit,
        client: 1,
        amount: Some(1.0),
        tx: 1
    };
    assert!(r.is_valid());

    // it should be valid for a c creation
    let c = Client::from_record(&r);
    assert!(c.is_ok());

    // amount must be strictly greater than 0
    r.amount = Some(0.0);
    assert!(!r.is_valid());

    // the amount should be present as well...
    r.amount = None;
    assert!(!r.is_valid());

    r.r#type = Operation::Withdrawal;
    assert!(!r.is_valid());

    // ...but only for some operations
    r.r#type = Operation::Dispute;
    assert!(r.is_valid());
}

#[test]
fn check_operations() {
    let mut r = Record {
        r#type: Operation::Deposit,
        client: 1,
        amount: Some(3.0),
        tx: 1
    };
    // valid init
    let mut c = Client::from_record(&r).unwrap();
    assert_eq!(c.total, r.amount.unwrap());
    assert_eq!(c.total, c.available);
    assert_eq!(c.held, 0.0);
    assert!(!c.locked);

    // deposit
    c.handle_record(&r, false);
    assert_eq!(c.total, r.amount.unwrap() * 2.0);

    // withdrawal
    r.amount = Some(0.5);
    r.r#type = Operation::Withdrawal;
    c.handle_record(&r, false);
    assert_eq!(c.total, 5.5);
    assert_eq!(c.available, 5.5);

    // ignore withdrawal if not enough funds
    r.amount = Some(10.0);
    r.r#type = Operation::Withdrawal;
    c.handle_record(&r, false);
    assert_eq!(c.total, 5.5);
    assert_eq!(c.available, 5.5);

    // validity check
    assert_eq!(c.total, c.available + c.held);
}

#[test]
fn check_disputes() {
    let mut r = Record {
        r#type: Operation::Deposit,
        client: 1,
        amount: Some(3.0),
        tx: 1
    };
    let mut c = Client::from_record(&r).unwrap();
    r.tx = 2;
    c.handle_record(&r, false);
    r.tx = 3;
    c.handle_record(&r, false);

    // ignore chargeback if no dispute
    r.tx = 1;
    r.r#type = Operation::Chargeback;
    c.handle_record(&r, false);
    assert_eq!(c.held, 0.0);

    r.r#type = Operation::Dispute;
    c.handle_record(&r, false);
    assert_eq!(c.held, 3.0);
    assert_eq!(c.total, 9.0);
    assert_eq!(c.total, c.available + c.held);

    // valid chargeback
    r.r#type = Operation::Chargeback;
    c.handle_record(&r, false);
    assert_eq!(c.held, 0.0);
    assert_eq!(c.total, 6.0);
    assert_eq!(c.total, c.available + c.held);
    assert!(c.locked);
    c.locked = false;

    // dispute already handled for the given transaction
    r.r#type = Operation::Dispute;
    c.handle_record(&r, false);
    assert_eq!(c.held, 0.0);
    assert_eq!(c.total, c.available + c.held);

    r.tx = 2;
    r.r#type = Operation::Dispute;
    c.handle_record(&r, false);
    assert_eq!(c.held, 3.0);
    assert_eq!(c.total, 6.0);

    // resolve the dispute
    r.r#type = Operation::Resolve;
    c.handle_record(&r, false);
    assert_eq!(c.held, 0.0);
    assert_eq!(c.total, 6.0);
    assert_eq!(c.total, c.available + c.held);

    r.tx = 4;
    r.r#type = Operation::Deposit;
    c.handle_record(&r, false);

    r.r#type = Operation::Withdrawal;
    r.amount = Some(9.0);
    c.handle_record(&r, false);
    assert_eq!(c.total, 0.0);

    // no available amount for a dispute
    r.r#type = Operation::Dispute;
    c.handle_record(&r, false);
    assert_eq!(c.held, 0.0);
    assert_eq!(c.total, c.available + c.held);

    // ignore operation if account is locked
    r.tx = 5;
    r.r#type = Operation::Deposit;
    c.locked = true;
    c.handle_record(&r, true);
    assert_eq!(c.total, 0.0);
}
