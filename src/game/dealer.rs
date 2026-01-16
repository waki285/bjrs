use alloc::vec::Vec;

use crate::card::Card;
use crate::error::ShowdownError;
use crate::hand::HandStatus;
use crate::options::RoundingMode;
use crate::result::{HandOutcome, HandResult, PlayerResult, RoundResult};

use super::{Game, GameState};

#[cfg(feature = "std")]
fn round_amount(amount: f64, mode: RoundingMode) -> usize {
    match mode {
        RoundingMode::Up => amount.ceil() as usize,
        RoundingMode::Down => amount.floor() as usize,
        RoundingMode::Nearest => amount.round() as usize,
    }
}

#[cfg(all(not(feature = "std"), feature = "alloc"))]
fn round_amount(amount: f64, mode: RoundingMode) -> usize {
    match mode {
        RoundingMode::Up => libm::ceil(amount) as usize,
        RoundingMode::Down => libm::floor(amount) as usize,
        RoundingMode::Nearest => libm::round(amount) as usize,
    }
}

impl Game {
    /// Checks if any player has a non-busted, non-surrendered hand.
    fn any_active_hands(&self) -> bool {
        for player_hands in self.hands.lock().values() {
            for hand in player_hands {
                match hand.status() {
                    HandStatus::Stand | HandStatus::Blackjack => return true,
                    _ => {}
                }
            }
        }
        false
    }

    /// Dealer plays their hand according to the rules.
    ///
    /// The dealer reveals their hole card and draws until reaching 17 or higher.
    /// If `stand_on_soft_17` is true, dealer stands on soft 17.
    /// Otherwise, dealer hits on soft 17.
    ///
    /// Returns the cards drawn by the dealer.
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in dealer turn state or the shoe is
    /// empty while the dealer must draw.
    pub fn dealer_play(&self) -> Result<Vec<Card>, ShowdownError> {
        if *self.state.lock() != GameState::DealerTurn {
            return Err(ShowdownError::InvalidState);
        }

        let mut dealer = self.dealer_hand.lock();
        dealer.reveal_hole();

        let mut drawn_cards = Vec::new();

        // If no active hands, dealer doesn't need to draw
        drop(dealer);
        if !self.any_active_hands() {
            *self.state.lock() = GameState::RoundOver;
            return Ok(drawn_cards);
        }

        // Dealer draws according to rules
        loop {
            let dealer = self.dealer_hand.lock();
            let value = dealer.value();
            let is_soft = dealer.is_soft();
            drop(dealer);

            // Stand on 17 or higher (considering soft 17 rule)
            if value > 17 {
                break;
            }
            if value == 17 && (!is_soft || self.options.stand_on_soft_17) {
                break;
            }

            // Draw a card
            let card = self.draw().ok_or(ShowdownError::NoCards)?;
            self.dealer_hand.lock().add_card(card);
            drawn_cards.push(card);
        }

        *self.state.lock() = GameState::RoundOver;

        Ok(drawn_cards)
    }

    /// Rounds a payout according to the rounding mode.
    pub(super) fn round_payout(&self, amount: f64, mode: RoundingMode) -> usize {
        round_amount(amount, mode)
    }

    /// Performs the showdown and calculates payouts.
    ///
    /// This function:
    /// 1. Compares each player's hand to the dealer's hand
    /// 2. Calculates winnings based on the outcome
    /// 3. Updates player money
    /// 4. Returns detailed results for each player
    ///
    /// # Errors
    ///
    /// Returns an error if the game is not in round-over state.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "locks are held for entire operation"
    )]
    pub fn showdown(&self) -> Result<RoundResult, ShowdownError> {
        let state = *self.state.lock();
        if state != GameState::RoundOver {
            return Err(ShowdownError::InvalidState);
        }

        let dealer = self.dealer_hand.lock();
        let dealer_value = dealer.value();
        let dealer_bust = dealer.is_bust();
        let dealer_blackjack = dealer.is_blackjack();
        drop(dealer);

        let order = self.betting_order.lock();
        let hands = self.hands.lock();
        let mut money = self.money.lock();

        let mut player_results = Vec::new();

        for &player_id in order.iter() {
            let Some(player_hands) = hands.get(&player_id) else {
                continue;
            };

            let mut hand_results = Vec::new();
            let mut total_payout: usize = 0;
            let mut total_bet: usize = 0;
            let mut surrender_refund_total: usize = 0;

            for (hand_index, hand) in player_hands.iter().enumerate() {
                let bet = hand.bet();
                total_bet += bet;
                let player_value = hand.value();

                let (outcome, payout) = match hand.status() {
                    HandStatus::Surrendered => {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "f64 has sufficient precision for monetary values"
                        )]
                        let refund =
                            self.round_payout((bet as f64) * 0.5, self.options.rounding_surrender);
                        surrender_refund_total += refund;
                        // Already refunded half during surrender
                        (HandOutcome::Surrendered, 0)
                    }
                    HandStatus::Bust => {
                        // Player busted, loses bet
                        (HandOutcome::Lose, 0)
                    }
                    HandStatus::Blackjack => {
                        if dealer_blackjack {
                            // Push - return original bet
                            (HandOutcome::Push, bet)
                        } else {
                            // Blackjack pays extra
                            #[expect(
                                clippy::cast_precision_loss,
                                reason = "f64 has sufficient precision for monetary values"
                            )]
                            let winnings = (bet as f64) * self.options.blackjack_pays;
                            let rounded =
                                self.round_payout(winnings, self.options.rounding_blackjack);
                            (HandOutcome::Blackjack, bet + rounded)
                        }
                    }
                    HandStatus::Stand | HandStatus::Active => {
                        if dealer_bust {
                            // Dealer busts, player wins
                            (HandOutcome::Win, bet * 2)
                        } else if dealer_blackjack && !hand.is_from_split() && hand.len() == 2 {
                            // Dealer has blackjack, player loses (unless they also have blackjack)
                            (HandOutcome::Lose, 0)
                        } else if player_value > dealer_value {
                            // Player wins
                            (HandOutcome::Win, bet * 2)
                        } else if player_value < dealer_value {
                            // Dealer wins
                            (HandOutcome::Lose, 0)
                        } else {
                            // Push
                            (HandOutcome::Push, bet)
                        }
                    }
                };

                total_payout += payout;

                hand_results.push(HandResult {
                    hand_index,
                    outcome,
                    bet,
                    payout,
                    player_value,
                    dealer_value,
                });
            }

            // Handle insurance payout
            let insurance_bet = self
                .insurance_bets
                .lock()
                .get(&player_id)
                .copied()
                .unwrap_or(0);

            let insurance_payout = if dealer_blackjack && insurance_bet > 0 {
                // Insurance pays 2:1
                insurance_bet * 3 // Original bet + 2x winnings
            } else {
                0
            };

            total_payout += insurance_payout;
            total_bet += insurance_bet;

            // Update player money
            if let Some(player_money) = money.get_mut(&player_id) {
                *player_money += total_payout;
            }

            #[expect(clippy::cast_possible_wrap, reason = "payout values fit in isize")]
            let net =
                (total_payout as isize + surrender_refund_total as isize) - (total_bet as isize);

            player_results.push(PlayerResult {
                player_id,
                hands: hand_results,
                total_payout,
                net,
                insurance_bet,
                insurance_payout,
            });
        }

        Ok(RoundResult {
            players: player_results,
            dealer_value,
            dealer_bust,
            dealer_blackjack,
        })
    }
}
