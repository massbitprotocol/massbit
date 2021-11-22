use serde::{Deserialize, Serialize};

pub type SolanaBlock = ExtBlock;
pub type SolanaTransaction = ExtTransaction;
// The most similar Event concept in Solana is log_messages in UiTransactionStatusMeta in EncodedTransactionWithStatusMeta
pub type SolanaLogMessages = ExtLogMessages;

type Block = solana_transaction_status::ConfirmedBlock;
type Transaction = solana_transaction_status::TransactionWithStatusMeta;
type LogMessages = Option<Vec<String>>;

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtBlock {
    pub version: String,
    pub timestamp: i64,
    pub block_slot: u64,
    pub block: Block,
    pub list_log_messages: Vec<LogMessages>,
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtTransaction {
    pub block_slot: u64,
    pub transaction: Transaction,
    //pub block: Arc<ExtBlock>,
    pub log_messages: LogMessages,
    pub success: bool,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtLogMessages {
    pub block_slot: u64,
    pub log_messages: LogMessages,
    pub transaction: Transaction,
    //pub block: Arc<ExtBlock>,
}
