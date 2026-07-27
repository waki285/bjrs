use crate::error::InsuranceError;
use crate::hand::HandStatus;

use super::{Game, GameState};

impl Game {
    fn player_is_eligible_for_insurance(&self, player_id: u8) -> bool {
        self.hands
            .lock()
            .get(&player_id)
            .and_then(|player_hands| player_hands.first())
            .is_some_and(|hand| hand.status() != HandStatus::Surrendered)
    }

    pub(super) fn has_players_eligible_for_insurance(&self) -> bool {
        let order = self.betting_order.lock();
        let hands = self.hands.lock();

        order.iter().any(|player_id| {
            hands
                .get(player_id)
                .and_then(|player_hands| player_hands.first())
                .is_some_and(|hand| hand.status() != HandStatus::Surrendered)
        })
    }

    /// Returns whether insurance is currently being offered.
    pub fn is_insurance_offered(&self) -> bool {
        *self.state.lock() == GameState::Insurance
    }

    /// Takes insurance for the specified player.
    ///
    /// The insurance bet is half of the original bet.
    /// If the dealer has blackjack, pays 2:1.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The game is not in the insurance state
    /// - Insurance is not offered at this table
    /// - The player is not found or has not bet
    /// - The player already surrendered
    /// - The player has insufficient funds
    /// - The player has already made an insurance decision
    pub fn take_insurance(&self, player_id: u8) -> Result<usize, InsuranceError> {
        if *self.state.lock() != GameState::Insurance {
            return Err(InsuranceError::InvalidState);
        }

        if !self.options.insurance {
            return Err(InsuranceError::NotOffered);
        }

        // Check if player already decided
        if self.insurance_decided.lock().contains(&player_id) {
            return Err(InsuranceError::AlreadyDecided);
        }

        // Get original bet
        let original_bet = self
            .bets
            .lock()
            .get(&player_id)
            .copied()
            .ok_or(InsuranceError::NoBet)?;

        if !self.player_is_eligible_for_insurance(player_id) {
            return Err(InsuranceError::NotEligible);
        }

        let insurance_bet = original_bet / 2;

        // Check if player has enough money
        let mut money = self.money.lock();
        let player_money = money
            .get_mut(&player_id)
            .ok_or(InsuranceError::PlayerNotFound)?;

        if *player_money < insurance_bet {
            return Err(InsuranceError::InsufficientFunds);
        }

        *player_money -= insurance_bet;
        drop(money);

        // Record insurance bet
        self.insurance_bets.lock().insert(player_id, insurance_bet);
        self.insurance_decided.lock().push(player_id);

        Ok(insurance_bet)
    }

    /// Declines insurance for the specified player.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The game is not in the insurance state
    /// - The player has not bet
    /// - The player already surrendered
    /// - The player has already made an insurance decision
    pub fn decline_insurance(&self, player_id: u8) -> Result<(), InsuranceError> {
        if *self.state.lock() != GameState::Insurance {
            return Err(InsuranceError::InvalidState);
        }

        // Check if player already decided
        if self.insurance_decided.lock().contains(&player_id) {
            return Err(InsuranceError::AlreadyDecided);
        }

        // Check if player has bet
        if !self.bets.lock().contains_key(&player_id) {
            return Err(InsuranceError::NoBet);
        }

        if !self.player_is_eligible_for_insurance(player_id) {
            return Err(InsuranceError::NotEligible);
        }

        // Record decision (no insurance bet)
        self.insurance_decided.lock().push(player_id);

        Ok(())
    }

    /// Checks if all players have made their insurance decision.
    pub fn all_insurance_decided(&self) -> bool {
        let order = self.betting_order.lock();
        let hands = self.hands.lock();
        let decided = self.insurance_decided.lock();
        order.iter().all(|player_id| {
            let eligible = hands
                .get(player_id)
                .and_then(|player_hands| player_hands.first())
                .is_some_and(|hand| hand.status() != HandStatus::Surrendered);

            !eligible || decided.contains(player_id)
        })
    }

    /// Finishes the insurance phase and advances to the next required turn.
    ///
    /// This should be called after all players have made their insurance decision.
    /// The dealer checks for blackjack after all early-surrender and insurance
    /// decisions have been made. If the dealer has blackjack, the round ends
    /// immediately.
    ///
    /// Returns `true` if the dealer has blackjack (round ends), `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in insurance state or eligible
    /// players still have an insurance decision pending.
    pub fn finish_insurance(&self) -> Result<bool, InsuranceError> {
        if *self.state.lock() != GameState::Insurance {
            return Err(InsuranceError::InvalidState);
        }

        if !self.all_insurance_decided() {
            return Err(InsuranceError::DecisionsPending);
        }

        Ok(self.peek_dealer_and_start_player_turn())
    }

    /// Returns the insurance bet for the specified player.
    pub fn get_insurance_bet(&self, player_id: u8) -> Option<usize> {
        self.insurance_bets.lock().get(&player_id).copied()
    }
}
