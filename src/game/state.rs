//! Game state types.

/// Game state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// Waiting for players to join.
    WaitingForPlayers,
    /// Accepting bets for the next round.
    Betting,
    /// Dealing initial cards.
    Dealing,
    /// Offering insurance decisions.
    Insurance,
    /// Waiting for player actions.
    PlayerTurn,
    /// Dealer plays out their hand.
    DealerTurn,
    /// Round has ended and results can be settled.
    RoundOver,
}

/// Represents the current turn position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnPosition {
    /// Index into the betting players list.
    pub player_index: usize,
    /// Index into the player's hands (for splits).
    pub hand_index: usize,
}
