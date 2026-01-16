//! Round result types for showdown.

extern crate alloc;

use alloc::vec::Vec;

/// Result of a single hand after showdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandOutcome {
    /// Player wins (dealer busts or player has higher value).
    Win,
    /// Player loses (player busts or dealer has higher value).
    Lose,
    /// Push (tie).
    Push,
    /// Player has blackjack.
    Blackjack,
    /// Player surrendered.
    Surrendered,
}

/// Result for a single hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandResult {
    /// The hand index (for split hands).
    pub hand_index: usize,
    /// The outcome of the hand.
    pub outcome: HandOutcome,
    /// The bet amount for this hand.
    pub bet: usize,
    /// The payout amount (winnings added to player money).
    pub payout: usize,
    /// The player's hand value.
    pub player_value: u8,
    /// The dealer's hand value.
    pub dealer_value: u8,
}

/// Result for a single player after showdown.
#[derive(Debug, Clone)]
pub struct PlayerResult {
    /// The player ID.
    pub player_id: u8,
    /// Results for each hand (multiple if split).
    pub hands: Vec<HandResult>,
    /// Total payout for all hands.
    pub total_payout: usize,
    /// Net result (positive = profit, negative = loss).
    pub net: isize,
    /// Insurance bet amount (0 if no insurance taken).
    pub insurance_bet: usize,
    /// Insurance payout (0 if dealer didn't have blackjack or no insurance taken).
    pub insurance_payout: usize,
}

/// Result of the entire round after showdown.
#[derive(Debug, Clone)]
pub struct RoundResult {
    /// Results for each player.
    pub players: Vec<PlayerResult>,
    /// The dealer's final hand value.
    pub dealer_value: u8,
    /// Whether the dealer busted.
    pub dealer_bust: bool,
    /// Whether the dealer had blackjack.
    pub dealer_blackjack: bool,
}
