//! Card types and deck utilities.

/// Card suit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    /// Hearts.
    Hearts,
    /// Diamonds.
    Diamonds,
    /// Clubs.
    Clubs,
    /// Spades.
    Spades,
}

/// A playing card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    /// The suit of the card.
    pub suit: Suit,
    /// The rank of the card (1 = Ace, 11 = Jack, 12 = Queen, 13 = King).
    pub rank: u8,
}

impl Card {
    /// Creates a new card.
    ///
    /// Note: This function does not validate the rank. Values outside 1..=13
    /// are accepted but may yield non-standard results when evaluating a hand.
    #[must_use]
    pub const fn new(suit: Suit, rank: u8) -> Self {
        Self { suit, rank }
    }
}

/// Number of cards per deck.
pub const DECK_SIZE: usize = 52;
