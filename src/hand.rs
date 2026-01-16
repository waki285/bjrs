//! Player and dealer hand representations.

extern crate alloc;

use alloc::vec::Vec;

use crate::card::Card;

const fn card_value(rank: u8) -> u8 {
    match rank {
        1 => 11,
        2..=10 => rank,
        11..=13 => 10,
        _ => 0,
    }
}

fn evaluate_cards(cards: &[Card]) -> (u8, bool) {
    let mut value: u8 = 0;
    let mut aces: u8 = 0;

    for card in cards {
        if card.rank == 1 {
            aces += 1;
        }
        value = value.saturating_add(card_value(card.rank));
    }

    while value > 21 && aces > 0 {
        value -= 10;
        aces -= 1;
    }

    let is_soft = aces > 0 && value <= 21;
    (value, is_soft)
}

/// Hand status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandStatus {
    /// Hand is active and can take actions.
    Active,
    /// Player has stood.
    Stand,
    /// Hand has busted (over 21).
    Bust,
    /// Hand is a blackjack (natural 21).
    Blackjack,
    /// Player has surrendered.
    Surrendered,
}

/// A player's hand.
#[derive(Debug, Clone)]
pub struct Hand {
    /// Cards in the hand.
    cards: Vec<Card>,
    /// Current status of the hand.
    status: HandStatus,
    /// Bet amount for this hand.
    bet: usize,
    /// Whether this hand is from a split.
    from_split: bool,
}

impl Hand {
    /// Creates a new empty hand with the given bet.
    #[must_use]
    pub const fn new(bet: usize) -> Self {
        Self {
            cards: Vec::new(),
            status: HandStatus::Active,
            bet,
            from_split: false,
        }
    }

    /// Creates a new hand from a split with a single card.
    #[must_use]
    pub fn from_split(card: Card, bet: usize) -> Self {
        Self {
            cards: alloc::vec![card],
            status: HandStatus::Active,
            bet,
            from_split: true,
        }
    }

    /// Adds a card to the hand.
    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);

        let (value, _) = evaluate_cards(&self.cards);

        // Check for bust
        if value > 21 {
            self.status = HandStatus::Bust;
        }
        // Check for blackjack (only on initial deal, not from split)
        else if self.cards.len() == 2 && value == 21 && !self.from_split {
            self.status = HandStatus::Blackjack;
        }
    }

    /// Returns the cards in the hand.
    #[must_use]
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// Returns the current status of the hand.
    #[must_use]
    pub const fn status(&self) -> HandStatus {
        self.status
    }

    /// Sets the hand status.
    pub const fn set_status(&mut self, status: HandStatus) {
        self.status = status;
    }

    /// Returns the bet amount for this hand.
    #[must_use]
    pub const fn bet(&self) -> usize {
        self.bet
    }

    /// Doubles the bet amount.
    pub const fn double_bet(&mut self) {
        self.bet *= 2;
    }

    /// Returns whether this hand is from a split.
    #[must_use]
    pub const fn is_from_split(&self) -> bool {
        self.from_split
    }

    /// Calculates the value of the hand.
    ///
    /// Aces are counted as 11 if possible without busting, otherwise as 1.
    #[must_use]
    pub fn value(&self) -> u8 {
        evaluate_cards(&self.cards).0
    }

    /// Returns whether the hand is soft (contains an ace counted as 11).
    #[must_use]
    pub fn is_soft(&self) -> bool {
        evaluate_cards(&self.cards).1
    }

    /// Returns whether the hand can be split.
    #[must_use]
    pub fn can_split(&self) -> bool {
        self.cards.len() == 2 && self.cards[0].rank == self.cards[1].rank
    }

    /// Returns the number of cards in the hand.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Returns whether the hand is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Removes and returns the second card (for splitting).
    pub fn take_split_card(&mut self) -> Option<Card> {
        if self.cards.len() == 2 {
            self.cards.pop()
        } else {
            None
        }
    }
}

/// The dealer's hand.
#[derive(Debug, Clone)]
pub struct DealerHand {
    /// Cards in the hand.
    cards: Vec<Card>,
    /// Whether the hole card is revealed.
    hole_revealed: bool,
}

impl DealerHand {
    /// Creates a new empty dealer hand.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cards: Vec::new(),
            hole_revealed: false,
        }
    }

    /// Adds a card to the hand.
    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    /// Returns all cards in the hand.
    #[must_use]
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// Returns the visible card (first card).
    #[must_use]
    pub fn up_card(&self) -> Option<&Card> {
        self.cards.first()
    }

    /// Returns whether the hole card is revealed.
    #[must_use]
    pub const fn is_hole_revealed(&self) -> bool {
        self.hole_revealed
    }

    /// Reveals the hole card.
    pub const fn reveal_hole(&mut self) {
        self.hole_revealed = true;
    }

    /// Calculates the visible value (only up card if hole not revealed).
    #[must_use]
    pub fn visible_value(&self) -> u8 {
        if self.hole_revealed {
            self.value()
        } else {
            self.cards.first().map_or(0, |c| card_value(c.rank))
        }
    }

    /// Calculates the full value of the hand.
    #[must_use]
    pub fn value(&self) -> u8 {
        evaluate_cards(&self.cards).0
    }

    /// Returns whether the hand is a blackjack.
    #[must_use]
    pub fn is_blackjack(&self) -> bool {
        self.cards.len() == 2 && self.value() == 21
    }

    /// Returns whether the hand is bust.
    #[must_use]
    pub fn is_bust(&self) -> bool {
        self.value() > 21
    }

    /// Returns whether the hand is soft (contains an ace counted as 11).
    #[must_use]
    pub fn is_soft(&self) -> bool {
        evaluate_cards(&self.cards).1
    }

    /// Returns the number of cards.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Returns whether the hand is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Clears the hand for a new round.
    pub fn clear(&mut self) {
        self.cards.clear();
        self.hole_revealed = false;
    }
}

impl Default for DealerHand {
    fn default() -> Self {
        Self::new()
    }
}
