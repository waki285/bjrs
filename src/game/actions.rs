use crate::card::Card;
use crate::error::ActionError;
use crate::hand::{Hand, HandStatus};
use crate::options::DoubleOption;

use super::{Game, GameState};

impl Game {
    fn ensure_player_turn(&self, player_id: u8, hand_index: usize) -> Result<(), ActionError> {
        if *self.state.lock() != GameState::PlayerTurn {
            return Err(ActionError::InvalidState);
        }

        if !self.is_player_turn(player_id, hand_index) {
            return Err(ActionError::NotYourTurn);
        }

        Ok(())
    }

    fn advance_after_hand(&self) {
        self.advance_to_next_active_hand();
        if self.all_players_done() {
            *self.state.lock() = GameState::DealerTurn;
        }
    }

    /// Player action: Hit (draw a card).
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in player turn state, it is not the
    /// player's turn, the player or hand cannot be found, the hand is not
    /// active, or the shoe is empty.
    #[expect(
        clippy::missing_panics_doc,
        reason = "internal expects are guaranteed to succeed"
    )]
    pub fn hit(&self, player_id: u8, hand_index: usize) -> Result<Card, ActionError> {
        self.ensure_player_turn(player_id, hand_index)?;

        // Get the hand
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .ok_or(ActionError::PlayerNotFound)?;
        let hand = player_hands
            .get_mut(hand_index)
            .ok_or(ActionError::HandNotFound)?;

        if hand.status() != HandStatus::Active {
            return Err(ActionError::HandNotActive);
        }

        // Draw a card
        drop(hands);
        let card = self.draw().ok_or(ActionError::NoCards)?;

        // Add card to hand
        // SAFETY: player_id and hand_index were validated above via ok_or checks.
        // The lock was temporarily dropped to call draw(), but no other code path
        // removes players or hands during a player's turn.
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .expect("player_id was validated above and cannot be removed mid-turn");
        let hand = player_hands
            .get_mut(hand_index)
            .expect("hand_index was validated above and cannot be removed mid-turn");
        hand.add_card(card);

        let status = hand.status();
        drop(hands);

        // If bust or 21, advance to next hand
        if status != HandStatus::Active {
            self.advance_after_hand();
        }

        Ok(card)
    }

    /// Player action: Stand (keep current hand).
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in player turn state, it is not the
    /// player's turn, the player or hand cannot be found, or the hand is not
    /// active.
    pub fn stand(&self, player_id: u8, hand_index: usize) -> Result<(), ActionError> {
        self.ensure_player_turn(player_id, hand_index)?;

        // Get the hand and set status
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .ok_or(ActionError::PlayerNotFound)?;
        let hand = player_hands
            .get_mut(hand_index)
            .ok_or(ActionError::HandNotFound)?;

        if hand.status() != HandStatus::Active {
            return Err(ActionError::HandNotActive);
        }

        hand.set_status(HandStatus::Stand);
        drop(hands);

        // Advance to next hand
        self.advance_after_hand();

        Ok(())
    }

    /// Player action: Double down (double bet, receive one card, then stand).
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in player turn state, it is not the
    /// player's turn, the player or hand cannot be found, the hand is not
    /// eligible to double down, the player lacks funds, or the shoe is empty.
    #[expect(
        clippy::missing_panics_doc,
        reason = "internal expects are guaranteed to succeed"
    )]
    pub fn double_down(&self, player_id: u8, hand_index: usize) -> Result<Card, ActionError> {
        self.ensure_player_turn(player_id, hand_index)?;

        // Get the hand
        let hands = self.hands.lock();
        let player_hands = hands.get(&player_id).ok_or(ActionError::PlayerNotFound)?;
        let hand = player_hands
            .get(hand_index)
            .ok_or(ActionError::HandNotFound)?;

        if hand.status() != HandStatus::Active {
            return Err(ActionError::HandNotActive);
        }

        // Can only double on first two cards
        if hand.len() != 2 {
            return Err(ActionError::CannotDouble);
        }

        // Check if from split and double after split is allowed
        if hand.is_from_split() && !self.options.double_after_split {
            return Err(ActionError::CannotDouble);
        }

        // Check if value allows doubling
        if !self.can_double_value(hand.value()) {
            return Err(ActionError::CannotDouble);
        }

        let bet = hand.bet();
        drop(hands);

        // Check if player has enough money
        let mut money = self.money.lock();
        let player_money = money
            .get_mut(&player_id)
            .ok_or(ActionError::PlayerNotFound)?;

        if *player_money < bet {
            return Err(ActionError::InsufficientFunds);
        }

        *player_money -= bet;
        drop(money);

        // Draw a card
        let card = self.draw().ok_or(ActionError::NoCards)?;

        // Add card and double bet
        // SAFETY: player_id and hand_index were validated above via ok_or checks.
        // The lock was temporarily dropped to call draw(), but no other code path
        // removes players or hands during a player's turn.
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .expect("player_id was validated above and cannot be removed mid-turn");
        let hand = player_hands
            .get_mut(hand_index)
            .expect("hand_index was validated above and cannot be removed mid-turn");
        hand.double_bet();
        hand.add_card(card);

        // If not bust, set to stand
        if hand.status() == HandStatus::Active {
            hand.set_status(HandStatus::Stand);
        }
        drop(hands);

        // Advance to next hand
        self.advance_after_hand();

        Ok(card)
    }

    /// Player action: Split (split a pair into two hands).
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in player turn state, it is not the
    /// player's turn, the player or hand cannot be found, the hand cannot be
    /// split, the maximum splits are reached, the player lacks funds, or the
    /// shoe is empty.
    #[expect(
        clippy::missing_panics_doc,
        reason = "internal expects are guaranteed to succeed"
    )]
    pub fn split(&self, player_id: u8, hand_index: usize) -> Result<(), ActionError> {
        self.ensure_player_turn(player_id, hand_index)?;

        // Get the hand
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .ok_or(ActionError::PlayerNotFound)?;

        // Check max splits
        if player_hands.len() > self.options.split as usize {
            return Err(ActionError::MaxSplitsReached);
        }

        let hand = player_hands
            .get_mut(hand_index)
            .ok_or(ActionError::HandNotFound)?;

        if hand.status() != HandStatus::Active {
            return Err(ActionError::HandNotActive);
        }

        // Check if can split
        if !hand.can_split() {
            return Err(ActionError::CannotSplit);
        }

        // Check ace split restrictions
        let is_ace = hand.cards().first().is_some_and(|c| c.rank == 1);
        if is_ace && hand.is_from_split() && self.options.split_aces_only_once {
            return Err(ActionError::CannotSplit);
        }

        let bet = hand.bet();
        drop(hands);

        // Check if player has enough money
        let mut money = self.money.lock();
        let player_money = money
            .get_mut(&player_id)
            .ok_or(ActionError::PlayerNotFound)?;

        if *player_money < bet {
            return Err(ActionError::InsufficientFunds);
        }

        *player_money -= bet;
        drop(money);

        // Perform the split
        // SAFETY: player_id and hand_index were validated above via ok_or checks.
        // can_split() was also verified, so take_split_card() will succeed.
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .expect("player_id was validated above and cannot be removed mid-turn");
        let hand = player_hands
            .get_mut(hand_index)
            .expect("hand_index was validated above and cannot be removed mid-turn");

        let split_card = hand
            .take_split_card()
            .expect("can_split() was verified above");
        let new_hand = Hand::from_split(split_card, bet);

        // Draw a card for each hand
        drop(hands);
        let card1 = self.draw().ok_or(ActionError::NoCards)?;
        let card2 = self.draw().ok_or(ActionError::NoCards)?;

        // SAFETY: player_id and hand_index were validated above.
        // The lock was temporarily dropped to call draw(), but no other code path
        // removes players or hands during a player's turn.
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .expect("player_id was validated above and cannot be removed mid-turn");

        // Add card to original hand
        let hand = player_hands
            .get_mut(hand_index)
            .expect("hand_index was validated above and cannot be removed mid-turn");
        hand.add_card(card1);

        // If split aces receive only one card, stand immediately
        if is_ace && self.options.split_aces_receive_one_card && hand.status() == HandStatus::Active
        {
            hand.set_status(HandStatus::Stand);
        }

        // Insert new hand after current one
        let mut new_hand = new_hand;
        new_hand.add_card(card2);

        if is_ace
            && self.options.split_aces_receive_one_card
            && new_hand.status() == HandStatus::Active
        {
            new_hand.set_status(HandStatus::Stand);
        }

        player_hands.insert(hand_index + 1, new_hand);
        drop(hands);

        // If aces that auto-stand, advance
        if is_ace && self.options.split_aces_receive_one_card {
            self.advance_after_hand();
        }

        Ok(())
    }

    /// Player action: Surrender (forfeit half the bet).
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in player turn state, surrender is
    /// disabled, it is not the player's turn, the player or hand cannot be
    /// found, or the hand is not eligible to surrender.
    pub fn surrender(&self, player_id: u8, hand_index: usize) -> Result<usize, ActionError> {
        if *self.state.lock() != GameState::PlayerTurn {
            return Err(ActionError::InvalidState);
        }

        // Check if surrender is allowed
        if !self.options.surrender {
            return Err(ActionError::CannotSurrender);
        }

        // Check if it's this player's turn
        if !self.is_player_turn(player_id, hand_index) {
            return Err(ActionError::NotYourTurn);
        }

        // Get the hand
        let mut hands = self.hands.lock();
        let player_hands = hands
            .get_mut(&player_id)
            .ok_or(ActionError::PlayerNotFound)?;
        let hand = player_hands
            .get_mut(hand_index)
            .ok_or(ActionError::HandNotFound)?;

        if hand.status() != HandStatus::Active {
            return Err(ActionError::HandNotActive);
        }

        // Can only surrender on first two cards and not from split
        if hand.len() != 2 || hand.is_from_split() {
            return Err(ActionError::CannotSurrender);
        }

        let bet = hand.bet();
        hand.set_status(HandStatus::Surrendered);
        drop(hands);

        // Return half the bet
        #[expect(
            clippy::cast_precision_loss,
            reason = "f64 has sufficient precision for monetary values"
        )]
        let refund = self.round_payout((bet as f64) * 0.5, self.options.rounding_surrender);
        let mut money = self.money.lock();
        if let Some(player_money) = money.get_mut(&player_id) {
            *player_money += refund;
        }
        drop(money);

        // Advance to next hand
        self.advance_after_hand();

        Ok(refund)
    }

    /// Checks if it's the specified player's turn on the specified hand.
    fn is_player_turn(&self, player_id: u8, hand_index: usize) -> bool {
        let turn = self.current_turn.lock();
        let order = self.betting_order.lock();

        if let Some(&current_player) = order.get(turn.player_index) {
            current_player == player_id && turn.hand_index == hand_index
        } else {
            false
        }
    }

    /// Advances to the next active hand (skipping blackjacks, busts, stands).
    pub(super) fn advance_to_next_active_hand(&self) {
        let mut turn = self.current_turn.lock();
        let order = self.betting_order.lock();
        let hands = self.hands.lock();

        loop {
            // Try next hand for current player
            if let Some(&player_id) = order.get(turn.player_index) {
                if let Some(player_hands) = hands.get(&player_id) {
                    turn.hand_index += 1;
                    if turn.hand_index < player_hands.len() {
                        if player_hands[turn.hand_index].status() == HandStatus::Active {
                            return;
                        }
                        continue;
                    }
                }
            }

            // Move to next player
            turn.player_index += 1;
            turn.hand_index = 0;

            if turn.player_index >= order.len() {
                // No more players, transition will happen
                return;
            }

            // Check if this player's first hand is active
            if let Some(&player_id) = order.get(turn.player_index) {
                if let Some(player_hands) = hands.get(&player_id) {
                    if !player_hands.is_empty() && player_hands[0].status() == HandStatus::Active {
                        return;
                    }
                }
            }
        }
    }

    /// Checks if all players have finished their turns.
    fn all_players_done(&self) -> bool {
        let turn = self.current_turn.lock();
        let order = self.betting_order.lock();
        turn.player_index >= order.len()
    }

    /// Checks if double down is allowed for the given hand value.
    fn can_double_value(&self, value: u8) -> bool {
        match self.options.double {
            DoubleOption::Any => true,
            DoubleOption::NineOrTen => value == 9 || value == 10,
            DoubleOption::NineThrough11 => (9..=11).contains(&value),
            DoubleOption::NineThrough15 => (9..=15).contains(&value),
            DoubleOption::None => false,
        }
    }
}
