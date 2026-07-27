use crate::card::Card;
use crate::error::ActionError;
use crate::hand::{Hand, HandStatus};
use crate::options::{DoubleOption, SurrenderOption};

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

    fn ensure_early_surrender_turn(
        &self,
        player_id: u8,
        hand_index: usize,
    ) -> Result<(), ActionError> {
        if *self.state.lock() != GameState::EarlySurrender {
            return Err(ActionError::InvalidState);
        }

        if !self.is_player_turn(player_id, hand_index) {
            return Err(ActionError::NotYourTurn);
        }

        if self.early_surrender_decided.lock().contains(&player_id) {
            return Err(ActionError::CannotSurrender);
        }

        Ok(())
    }

    fn advance_after_hand(&self) {
        self.advance_to_next_active_hand();
        if self.all_players_done() {
            *self.state.lock() = GameState::DealerTurn;
        }
    }

    fn start_player_or_dealer_turn(&self) {
        *self.current_turn.lock() = super::TurnPosition {
            player_index: 0,
            hand_index: 0,
        };
        self.advance_if_current_inactive();
        *self.state.lock() = if self.all_players_done() {
            GameState::DealerTurn
        } else {
            GameState::PlayerTurn
        };
    }

    fn dealer_up_card_is_ace(&self) -> bool {
        self.dealer_hand
            .lock()
            .up_card()
            .is_some_and(|card| card.rank == 1)
    }

    fn dealer_up_card_can_have_blackjack(&self) -> bool {
        self.dealer_hand
            .lock()
            .up_card()
            .is_some_and(|card| card.rank == 1 || card.rank >= 10)
    }

    fn should_offer_early_surrender(&self) -> bool {
        match self.options.surrender {
            SurrenderOption::Early => self.dealer_up_card_can_have_blackjack(),
            SurrenderOption::EarlyWithoutAce => {
                self.dealer_up_card_can_have_blackjack() && !self.dealer_up_card_is_ace()
            }
            SurrenderOption::None | SurrenderOption::Late => false,
        }
    }

    fn can_surrender_during_player_turn_without_prompt(&self) -> bool {
        matches!(
            self.options.surrender,
            SurrenderOption::Early | SurrenderOption::EarlyWithoutAce
        ) && !self.dealer_up_card_can_have_blackjack()
    }

    pub(super) fn peek_dealer_and_start_player_turn(&self) -> bool {
        let dealer_has_blackjack =
            self.dealer_up_card_can_have_blackjack() && self.dealer_hand.lock().is_blackjack();

        if dealer_has_blackjack {
            self.dealer_hand.lock().reveal_hole();
            *self.state.lock() = GameState::RoundOver;
            true
        } else {
            self.start_player_or_dealer_turn();
            false
        }
    }

    fn begin_insurance_or_peek(&self) {
        if self.dealer_up_card_is_ace()
            && self.options.insurance
            && self.has_players_eligible_for_insurance()
        {
            *self.state.lock() = GameState::Insurance;
        } else {
            self.peek_dealer_and_start_player_turn();
        }
    }

    fn can_surrender_hand(hand: &Hand) -> bool {
        hand.status() == HandStatus::Active && hand.len() == 2 && !hand.is_from_split()
    }

    fn has_pending_early_surrender_decision(&self) -> bool {
        let order = self.betting_order.lock();
        let hands = self.hands.lock();
        let decided = self.early_surrender_decided.lock();

        order.iter().any(|player_id| {
            !decided.contains(player_id)
                && hands
                    .get(player_id)
                    .and_then(|player_hands| player_hands.first())
                    .is_some_and(Self::can_surrender_hand)
        })
    }

    fn advance_to_next_early_surrender_turn(&self) {
        let mut turn = self.current_turn.lock();
        let order = self.betting_order.lock();
        let hands = self.hands.lock();
        let decided = self.early_surrender_decided.lock();

        while let Some(&player_id) = order.get(turn.player_index) {
            let can_decide = !decided.contains(&player_id)
                && hands
                    .get(&player_id)
                    .and_then(|player_hands| player_hands.first())
                    .is_some_and(Self::can_surrender_hand);

            if can_decide {
                turn.hand_index = 0;
                return;
            }

            turn.player_index += 1;
        }

        drop(decided);
        drop(hands);
        drop(order);
        turn.hand_index = 0;
    }

    pub(super) fn begin_initial_action_phase(&self) {
        if self.should_offer_early_surrender() {
            self.advance_to_next_early_surrender_turn();
            if self.has_pending_early_surrender_decision() {
                *self.state.lock() = GameState::EarlySurrender;
                return;
            }
        }

        self.begin_insurance_or_peek();
    }

    fn advance_after_early_surrender_decision(&self) {
        self.advance_to_next_early_surrender_turn();
        if !self.has_pending_early_surrender_decision() {
            self.begin_insurance_or_peek();
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

    /// Declines an early surrender offer and continues to the dealer check.
    ///
    /// # Errors
    ///
    /// Returns an error if early surrender is not enabled, it is not the
    /// player's early-surrender turn, or the initial hand is not eligible.
    pub fn decline_early_surrender(&self, player_id: u8) -> Result<(), ActionError> {
        if !matches!(
            self.options.surrender,
            SurrenderOption::Early | SurrenderOption::EarlyWithoutAce
        ) {
            return Err(ActionError::CannotSurrender);
        }

        self.ensure_early_surrender_turn(player_id, 0)?;

        let hands = self.hands.lock();
        let player_hands = hands.get(&player_id).ok_or(ActionError::PlayerNotFound)?;
        let hand = player_hands.first().ok_or(ActionError::HandNotFound)?;

        if hand.status() != HandStatus::Active {
            return Err(ActionError::HandNotActive);
        }
        if !Self::can_surrender_hand(hand) {
            return Err(ActionError::CannotSurrender);
        }
        drop(hands);

        self.early_surrender_decided.lock().push(player_id);
        self.advance_after_early_surrender_decision();

        Ok(())
    }

    /// Player action: Surrender (forfeit half the bet).
    ///
    /// # Errors
    ///
    /// Returns an error if the current surrender phase does not permit the
    /// action, it is not the player's turn, the player or hand cannot be
    /// found, or the hand is not eligible to surrender.
    pub fn surrender(&self, player_id: u8, hand_index: usize) -> Result<usize, ActionError> {
        let early_surrender = match self.options.surrender {
            SurrenderOption::None => return Err(ActionError::CannotSurrender),
            SurrenderOption::Early | SurrenderOption::EarlyWithoutAce => {
                if self.state() == GameState::EarlySurrender {
                    self.ensure_early_surrender_turn(player_id, hand_index)?;
                    true
                } else if self.can_surrender_during_player_turn_without_prompt() {
                    self.ensure_player_turn(player_id, hand_index)?;
                    false
                } else {
                    return Err(ActionError::CannotSurrender);
                }
            }
            SurrenderOption::Late => {
                self.ensure_player_turn(player_id, hand_index)?;
                false
            }
        };

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

        // Can only surrender on the initial two-card hand.
        if !Self::can_surrender_hand(hand) {
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

        if early_surrender {
            self.early_surrender_decided.lock().push(player_id);
            self.advance_after_early_surrender_decision();
        } else {
            self.advance_after_hand();
        }

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
            if let Some(&player_id) = order.get(turn.player_index)
                && let Some(player_hands) = hands.get(&player_id)
            {
                turn.hand_index += 1;
                if turn.hand_index < player_hands.len() {
                    if player_hands[turn.hand_index].status() == HandStatus::Active {
                        return;
                    }
                    continue;
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
            if let Some(&player_id) = order.get(turn.player_index)
                && let Some(player_hands) = hands.get(&player_id)
                && !player_hands.is_empty()
                && player_hands[0].status() == HandStatus::Active
            {
                return;
            }
        }
    }

    /// Checks if all players have finished their turns.
    pub(super) fn all_players_done(&self) -> bool {
        let turn = self.current_turn.lock();
        let order = self.betting_order.lock();
        turn.player_index >= order.len()
    }

    /// Checks if double down is allowed for the given hand value.
    fn can_double_value(&self, value: u8) -> bool {
        match self.options.double {
            DoubleOption::Any => true,
            DoubleOption::NineOrTen => value == 9 || value == 10,
            DoubleOption::TenOrEleven => value == 10 || value == 11,
            DoubleOption::NineThrough11 => (9..=11).contains(&value),
            DoubleOption::NineThrough15 => (9..=15).contains(&value),
            DoubleOption::None => false,
        }
    }
}
