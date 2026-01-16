use alloc::vec::Vec;

use crate::error::{BetError, DealError};
use crate::hand::Hand;

use super::{Game, GameState, TurnPosition};

impl Game {
    fn deal_one_card_to_players(&self, players: &[u8]) {
        for &player_id in players {
            if let Some(card) = self.draw() {
                let mut hands = self.hands.lock();
                if let Some(player_hands) = hands.get_mut(&player_id) {
                    if let Some(hand) = player_hands.first_mut() {
                        hand.add_card(card);
                    }
                }
            }
        }
    }

    /// Places a bet for the specified player.
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in betting state, the player cannot
    /// be found, the bet is zero, or the player lacks funds.
    pub fn bet(&self, player_id: u8, amount: usize) -> Result<(), BetError> {
        if amount == 0 {
            return Err(BetError::ZeroBet);
        }

        let state = self.state.lock();
        if *state != GameState::Betting {
            return Err(BetError::InvalidState);
        }
        drop(state);

        let mut money = self.money.lock();
        let player_money = money.get_mut(&player_id).ok_or(BetError::PlayerNotFound)?;

        if *player_money < amount {
            return Err(BetError::InsufficientFunds);
        }

        *player_money -= amount;
        drop(money);

        self.bets.lock().insert(player_id, amount);

        Ok(())
    }

    /// Deals initial cards to all players and the dealer.
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in betting state, no bets have been
    /// placed, or there are not enough cards in the shoe.
    pub fn deal(&self) -> Result<(), DealError> {
        let mut state = self.state.lock();
        if *state != GameState::Betting {
            return Err(DealError::InvalidState);
        }

        let bets = self.bets.lock();
        if bets.is_empty() {
            return Err(DealError::NoBets);
        }

        let player_count = bets.len();
        let cards_needed = (player_count + 1) * 2;

        if self.cards_remaining() < cards_needed {
            return Err(DealError::NotEnoughCards);
        }

        // Get player IDs who have bet (in order)
        let players = self.players.lock();
        let betting_players: Vec<u8> = players
            .iter()
            .filter(|id| bets.contains_key(*id))
            .copied()
            .collect();
        drop(bets);
        drop(players);

        // Store betting order
        (*self.betting_order.lock()).clone_from(&betting_players);

        // Initialize hands for each betting player
        let mut hands = self.hands.lock();
        hands.clear();

        for &player_id in &betting_players {
            let bet = self.bets.lock().get(&player_id).copied().unwrap_or(0);
            hands.insert(player_id, alloc::vec![Hand::new(bet)]);
        }
        drop(hands);

        // Clear dealer's hand
        self.dealer_hand.lock().clear();

        // Deal first card to each player
        self.deal_one_card_to_players(&betting_players);

        // Dealer's first card (up card)
        if let Some(card) = self.draw() {
            self.dealer_hand.lock().add_card(card);
        }

        // Second card to each player
        self.deal_one_card_to_players(&betting_players);

        // Dealer's second card (hole card)
        if let Some(card) = self.draw() {
            self.dealer_hand.lock().add_card(card);
        }

        // Initialize turn to first player, first hand
        *self.current_turn.lock() = TurnPosition {
            player_index: 0,
            hand_index: 0,
        };

        // Clear insurance state
        self.insurance_bets.lock().clear();
        self.insurance_decided.lock().clear();

        // Check if dealer's up card is an Ace and insurance is offered
        let dealer_up_card_is_ace = self
            .dealer_hand
            .lock()
            .up_card()
            .is_some_and(|c| c.rank == 1);

        if dealer_up_card_is_ace && self.options.insurance {
            *state = GameState::Insurance;
        } else {
            // Skip players with blackjack
            self.advance_if_current_inactive();
            *state = GameState::PlayerTurn;
            drop(state);
        }

        Ok(())
    }
}
