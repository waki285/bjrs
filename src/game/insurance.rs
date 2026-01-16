use crate::error::InsuranceError;

use super::{Game, GameState};

impl Game {
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

        // Record decision (no insurance bet)
        self.insurance_decided.lock().push(player_id);

        Ok(())
    }

    /// Checks if all players have made their insurance decision.
    pub fn all_insurance_decided(&self) -> bool {
        let order = self.betting_order.lock();
        let decided = self.insurance_decided.lock();
        order.iter().all(|id| decided.contains(id))
    }

    /// Finishes the insurance phase and moves to player turns.
    ///
    /// This should be called after all players have made their insurance decision.
    /// If the dealer has blackjack, the round ends immediately.
    ///
    /// Returns `true` if the dealer has blackjack (round ends), `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in insurance state.
    pub fn finish_insurance(&self) -> Result<bool, InsuranceError> {
        if *self.state.lock() != GameState::Insurance {
            return Err(InsuranceError::InvalidState);
        }

        let dealer_has_blackjack = self.dealer_hand.lock().is_blackjack();

        if dealer_has_blackjack {
            // Reveal dealer's hole card
            self.dealer_hand.lock().reveal_hole();
            *self.state.lock() = GameState::RoundOver;
            Ok(true)
        } else {
            // Continue to player turns
            self.advance_if_current_inactive();
            *self.state.lock() = GameState::PlayerTurn;
            Ok(false)
        }
    }

    /// Returns the insurance bet for the specified player.
    pub fn get_insurance_bet(&self, player_id: u8) -> Option<usize> {
        self.insurance_bets.lock().get(&player_id).copied()
    }
}
