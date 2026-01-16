//! Error types for game operations.

use thiserror::Error;

/// Errors that can occur during betting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum BetError {
    /// Player not found.
    #[error("player not found")]
    PlayerNotFound,
    /// Insufficient funds.
    #[error("insufficient funds")]
    InsufficientFunds,
    /// Invalid game state for betting.
    #[error("invalid game state for betting")]
    InvalidState,
    /// Bet amount is zero.
    #[error("bet amount is zero")]
    ZeroBet,
}

/// Errors that can occur during dealing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DealError {
    /// Invalid game state for dealing.
    #[error("invalid game state for dealing")]
    InvalidState,
    /// No players have placed bets.
    #[error("no players have placed bets")]
    NoBets,
    /// Not enough cards in the shoe.
    #[error("not enough cards in the shoe")]
    NotEnoughCards,
}

/// Errors that can occur during player actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ActionError {
    /// Invalid game state for this action.
    #[error("invalid game state for this action")]
    InvalidState,
    /// Not this player's turn.
    #[error("not this player's turn")]
    NotYourTurn,
    /// Player not found.
    #[error("player not found")]
    PlayerNotFound,
    /// Hand not found.
    #[error("hand not found")]
    HandNotFound,
    /// Hand is not active.
    #[error("hand is not active")]
    HandNotActive,
    /// Cannot double down on this hand.
    #[error("cannot double down on this hand")]
    CannotDouble,
    /// Cannot split this hand.
    #[error("cannot split this hand")]
    CannotSplit,
    /// Maximum splits reached.
    #[error("maximum splits reached")]
    MaxSplitsReached,
    /// Cannot surrender at this point.
    #[error("cannot surrender at this point")]
    CannotSurrender,
    /// Insufficient funds for this action.
    #[error("insufficient funds for this action")]
    InsufficientFunds,
    /// No cards left in the shoe.
    #[error("no cards left in the shoe")]
    NoCards,
}

/// Errors that can occur during insurance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum InsuranceError {
    /// Invalid game state for insurance.
    #[error("invalid game state for insurance")]
    InvalidState,
    /// Insurance is not offered at this table.
    #[error("insurance is not offered at this table")]
    NotOffered,
    /// Player not found.
    #[error("player not found")]
    PlayerNotFound,
    /// Insufficient funds for insurance.
    #[error("insufficient funds for insurance")]
    InsufficientFunds,
    /// Player already made insurance decision.
    #[error("player already made insurance decision")]
    AlreadyDecided,
    /// Player has not placed a bet.
    #[error("player has not placed a bet")]
    NoBet,
}

/// Errors that can occur during showdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ShowdownError {
    /// Invalid game state for showdown.
    #[error("invalid game state for showdown")]
    InvalidState,
    /// No cards left in the shoe.
    #[error("no cards left in the shoe")]
    NoCards,
}

/// Errors that can occur during reshuffling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReshuffleError {
    /// Invalid game state for reshuffling.
    #[error("invalid game state for reshuffling")]
    InvalidState,
}
