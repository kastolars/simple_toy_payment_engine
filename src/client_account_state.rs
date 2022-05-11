use std::collections::HashMap;
use crate::precision;


#[derive(Debug)]
pub struct TxMeta {
    amount: f32,
    under_dispute: bool,
}

#[derive(Debug)]
pub struct ClientAccountState {
    pub available: f32,
    pub held: f32,
    pub locked: bool,
    pub txs: HashMap<u32, TxMeta>,
}

impl ClientAccountState {
    /// A deposit is a credit to the client's asset account,
    /// meaning it should increase the available and total funds of the client account
    pub fn deposit(&mut self, tx: u32, amount: f32) -> Result<(), String> {
        let rounded_amount = precision::convert_precision(amount);
        if rounded_amount > 0.0000 {
            self.available += rounded_amount;
            self.txs.insert(tx, TxMeta {
                amount: rounded_amount,
                under_dispute: false,
            });
            return Ok(());
        }
        Err(String::from(format!("Transaction {} failed: must be positive non-zero amount", tx)))
    }

    /// A withdraw is a debit to the client's asset account,
    /// meaning it should decrease the available and total funds of the client account
    /// If a client does not have sufficient available funds the withdrawal should fail and the total amount of funds should not change
    pub fn withdraw(&mut self, tx: u32, amount: f32) -> Result<(), String> {
        let rounded_amount = precision::convert_precision(amount);
        if rounded_amount <= 0.0000 {
            return Err(String::from(format!("Transaction {} failed: must be positive non-zero amount", tx)));
        }
        if amount <= self.available {
            self.available -= rounded_amount;
            self.txs.insert(tx, TxMeta {
                amount: rounded_amount,
                under_dispute: false,
            });
            return Ok(());
        }
        Err(String::from(format!("Transaction {} failed: insufficient funds", tx)))
    }

    /// A dispute represents a client's claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn't be reversed yet but the associated funds should be held.
    /// This means that the clients available funds should decrease by the amount disputed,
    /// their held funds should increase by the amount disputed,
    /// while their total funds should remain the same.
    /// If the tx specified by the dispute doesn't exist,
    /// you can ignore it and assume this is an error on our partners side.
    pub fn dispute(&mut self, tx: u32) -> Result<(), String> {
        if let Some(tx_meta) = self.txs.get_mut(&tx) {
            tx_meta.under_dispute = true;
            let disputed_funds = tx_meta.amount;
            self.available -= disputed_funds;
            self.held += disputed_funds;
            return Ok(());
        }
        Err(String::from(format!("Dispute failed: transaction {} not found", tx)))
    }

    /// A resolve represents a resolution to a dispute,
    /// releasing the associated held funds.
    /// Funds that were previously disputed are no longer disputed.
    /// This means that the clients held funds should decrease by the amount no longer disputed,
    /// their available funds should increase by the amount no longer disputed,
    /// and their total funds should remain the same.
    /// If the tx specified doesn't exist,
    /// or the tx isn't under dispute,
    /// you can ignore the resolve and assume this is an error on our partner's side.
    pub fn resolve(&mut self, tx: u32) -> Result<(), String> {
        if let Some(tx_meta) = self.txs.get(&tx) {
            if !tx_meta.under_dispute {
                return Err(String::from("Invalid resolution: target transaction is not a dispute."));
            }
            let disputed_funds = tx_meta.amount;
            self.held -= disputed_funds;
            self.available += disputed_funds;
            return Ok(());
        }
        Err(String::from("No such transaction found"))
    }

    /// A chargeback is the final state of a dispute and represents the client reversing a transaction.
    /// Funds that were held have now been withdrawn.
    /// This means that the clients held funds and total funds should decrease by the amount previously disputed.
    /// If a chargeback occurs the client's account should be immediately frozen.
    /// If the tx specified doesn't exist,
    /// or the tx isn't under dispute,
    /// you can ignore the resolve and assume this is an error on our partner's side.
    pub fn chargeback(&mut self, tx: u32) -> Result<(), String> {
        if let Some(tx_meta) = self.txs.get_mut(&tx) {
            if !tx_meta.under_dispute {
                return Err(String::from("Invalid resolution: target transaction is not a dispute."));
            }
            tx_meta.under_dispute = false;
            let disputed_funds = tx_meta.amount;
            self.held -= disputed_funds;
            self.locked = true;
            return Ok(());
        }
        Err(String::from("No such transaction found"))
    }
}

impl Default for ClientAccountState {
    fn default() -> Self {
        return ClientAccountState {
            available: 0.0000,
            held: 0.0000,
            locked: false,
            txs: Default::default(),
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::client_account_state::ClientAccountState;

    #[test]
    fn test_deposit() {
        let mut state = ClientAccountState::default();

        assert_eq!(state.available, 0.0000);

        // Deposit zero amount should fail
        let result = state.deposit(1, 0.000000000);
        assert!(result.is_err());

        // Deposit negative amount should fail
        let result = state.deposit(2, -0.000000001);
        assert!(result.is_err());

        // Deposit positive, non-zero amount with large precision should fail
        let result = state.deposit(3, 0.0000000000001);
        assert!(result.is_err());

        // Deposit positive non-zero amount should succeed
        let result = state.deposit(4, 0.0001);
        assert!(result.is_ok());
        assert_eq!(state.available, 0.0001);
        assert!(state.txs.get(&4).is_some());
    }

    #[test]
    fn test_withdraw() {
        let mut state = ClientAccountState::default();

        assert_eq!(state.available, 0.0000);

        // Should fail due to insufficient funds
        let result = state.withdraw(1, 0.00000000001);
        assert!(result.is_err());
        assert_eq!(state.available, 0.0000);
        assert!(state.txs.get(&1).is_none());

        // Deposit funds so we can attempt to withdraw
        let _ = state.deposit(2, 60.0);

        // Should fail with negative amount
        let result = state.withdraw(3, -30.0);
        assert!(result.is_err());

        // Should fail with zero amount
        let result = state.withdraw(4, 0.00);
        assert!(result.is_err());

        // Should succeed after sufficient funds are available
        let result = state.withdraw(5, 30.0);
        assert!(result.is_ok());
        assert_eq!(state.available, 30.0);
        assert!(state.txs.get(&2).is_some());
        assert!(state.txs.get(&5).is_some());
    }

    #[test]
    fn test_dispute() {
        let mut state = ClientAccountState::default();

        assert_eq!(state.available, 0.0);
        assert_eq!(state.held, 0.0);

        // Disputing non-existent transaction should fail
        let result = state.dispute(1);
        assert!(result.is_err());
        assert_eq!(state.available, 0.0);
        assert_eq!(state.held, 0.0);

        // Dispute deposit
        let _ = state.deposit(2, 60.0);
        let result = state.dispute(2);
        assert!(result.is_ok());
        assert_eq!(state.available, 0.0);
        assert_eq!(state.held, 60.0);
        assert!(state.txs.get(&2).is_some());
        assert!(state.txs.get(&2).unwrap().under_dispute);

        // Dispute withdrawal
        let _ = state.deposit(3, 50.0);
        let _ = state.withdraw(4, 50.0);
        let result = state.dispute(4);
        assert!(result.is_ok());
        assert_eq!(state.available, -50.0);
        assert_eq!(state.held, 110.0);
        assert!(state.txs.get(&2).is_some());
        assert!(state.txs.get(&2).unwrap().under_dispute);
    }

    #[test]
    fn test_resolve() {
        let mut state = ClientAccountState::default();

        // Resolving non-existent transaction should fail
        let result = state.resolve(1);
        assert!(result.is_err());

        // Resolving undisputed transaction should fail
        let _ = state.deposit(2, 50.0);
        let result = state.resolve(2);
        assert!(result.is_err());

        // Resolve deposit
        let _ = state.dispute(2);
        let result = state.resolve(2);
        assert!(result.is_ok());
        assert_eq!(state.available, 50.0);
        assert_eq!(state.held, 0.0);

        // Resolve withdrawal
        let _ = state.withdraw(3, 50.0);
        let _ = state.dispute(3);
        let result = state.resolve(3);
        assert!(result.is_ok());
        assert_eq!(state.available, 0.0);
        assert_eq!(state.held, 0.0);
    }

    #[test]
    fn test_chargeback() {
        let mut state = ClientAccountState::default();

        // Charge back non-existent transaction should fail
        let result = state.chargeback(1);
        assert!(result.is_err());

        // Charging back undisputed transaction should fail
        let _ = state.deposit(2, 50.0);
        let result = state.chargeback(2);
        assert!(result.is_err());

        // Chargeback deposit
        let _ = state.dispute(2);
        let result = state.chargeback(2);
        assert!(result.is_ok());
        assert_eq!(state.available, 0.0);
        assert_eq!(state.held, 0.0);
        assert!(state.locked);

        // Chargeback withdrawal
        state.locked = false;
        let _ = state.deposit(3, 50.0);
        let _ = state.withdraw(4, 50.0);
        let _ = state.dispute(4);
        let result = state.chargeback(4);
        assert!(result.is_ok());
        assert_eq!(state.available, -50.0);
        assert_eq!(state.held, 0.0);
        assert!(state.locked);
    }
}