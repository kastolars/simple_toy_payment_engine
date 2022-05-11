#[derive(Debug, PartialEq, Deserialize)]
pub enum TransactionType {
    #[serde(alias = "deposit")]
    DEPOSIT,
    #[serde(alias = "withdrawal")]
    WITHDRAW,
    #[serde(alias = "dispute")]
    DISPUTE,
    #[serde(alias = "resolve")]
    RESOLVE,
    #[serde(alias = "chargeback")]
    CHARGEBACK,
}
