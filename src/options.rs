//! Game configuration options.

/// Conditions under which doubling down is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DoubleOption {
    /// Double down allowed on any hand.
    #[default]
    Any,
    /// Double down allowed only on 9 or 10.
    NineOrTen,
    /// Double down allowed only on 9 through 11.
    NineThrough11,
    /// Double down allowed only on 9 through 15.
    NineThrough15,
    /// Double down not allowed.
    None,
}

/// Rounding mode for payouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoundingMode {
    /// Round up.
    Up,
    /// Round down.
    Down,
    /// Round to nearest.
    Nearest,
}

/// Configuration options for a blackjack game.
///
/// Use the builder pattern to customize options:
///
/// ```
/// use bjrs::GameOptions;
///
/// let options = GameOptions::default()
///     .with_decks(6)
///     .with_blackjack_pays(1.5)
///     .with_stand_on_soft_17(true);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GameOptions {
    /// Number of decks.
    pub decks: u8,
    /// Blackjack payout ratio (typically 1.5).
    pub blackjack_pays: f64,
    /// Whether dealer stands on soft 17.
    pub stand_on_soft_17: bool,
    /// Double down conditions.
    pub double: DoubleOption,
    /// Maximum number of splits allowed.
    pub split: u8,
    /// Whether double down is allowed after split.
    pub double_after_split: bool,
    /// Whether aces can only be split once.
    pub split_aces_only_once: bool,
    /// Whether split aces receive only one card.
    pub split_aces_receive_one_card: bool,
    /// Whether surrender is allowed.
    pub surrender: bool,
    /// Whether insurance is offered.
    pub insurance: bool,
    /// Rounding mode for blackjack payouts.
    pub rounding_blackjack: RoundingMode,
    /// Rounding mode for surrender payouts.
    pub rounding_surrender: RoundingMode,
    /// Deck penetration (fraction of deck played before reshuffle).
    /// 0 to disable reshuffling.
    pub penetration: f64,
}

impl Default for GameOptions {
    fn default() -> Self {
        Self {
            decks: 2,
            blackjack_pays: 1.5,
            stand_on_soft_17: true,
            double: DoubleOption::Any,
            split: 3,
            double_after_split: true,
            split_aces_only_once: true,
            split_aces_receive_one_card: true,
            surrender: true,
            insurance: true,
            rounding_blackjack: RoundingMode::Down,
            rounding_surrender: RoundingMode::Nearest,
            penetration: 0.75,
        }
    }
}

impl GameOptions {
    /// Sets the number of decks.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_decks(6);
    /// assert_eq!(options.decks, 6);
    /// ```
    #[must_use]
    pub const fn with_decks(mut self, decks: u8) -> Self {
        self.decks = decks;
        self
    }

    /// Sets the blackjack payout ratio.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_blackjack_pays(1.2);
    /// assert_eq!(options.blackjack_pays, 1.2);
    /// ```
    #[must_use]
    pub const fn with_blackjack_pays(mut self, ratio: f64) -> Self {
        self.blackjack_pays = ratio;
        self
    }

    /// Sets whether dealer stands on soft 17.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_stand_on_soft_17(false);
    /// assert_eq!(options.stand_on_soft_17, false);
    /// ```
    #[must_use]
    pub const fn with_stand_on_soft_17(mut self, stand: bool) -> Self {
        self.stand_on_soft_17 = stand;
        self
    }

    /// Sets the double down conditions.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::{GameOptions, DoubleOption};
    ///
    /// let options = GameOptions::default().with_double(DoubleOption::NineThrough11);
    /// assert_eq!(options.double, DoubleOption::NineThrough11);
    /// ```
    #[must_use]
    pub const fn with_double(mut self, double: DoubleOption) -> Self {
        self.double = double;
        self
    }

    /// Sets the maximum number of splits allowed.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_split(4);
    /// assert_eq!(options.split, 4);
    /// ```
    #[must_use]
    pub const fn with_split(mut self, split: u8) -> Self {
        self.split = split;
        self
    }

    /// Sets whether double down is allowed after split.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_double_after_split(false);
    /// assert_eq!(options.double_after_split, false);
    /// ```
    #[must_use]
    pub const fn with_double_after_split(mut self, allowed: bool) -> Self {
        self.double_after_split = allowed;
        self
    }

    /// Sets whether aces can only be split once.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_split_aces_only_once(false);
    /// assert_eq!(options.split_aces_only_once, false);
    /// ```
    #[must_use]
    pub const fn with_split_aces_only_once(mut self, only_once: bool) -> Self {
        self.split_aces_only_once = only_once;
        self
    }

    /// Sets whether split aces receive only one card.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_split_aces_receive_one_card(false);
    /// assert_eq!(options.split_aces_receive_one_card, false);
    /// ```
    #[must_use]
    pub const fn with_split_aces_receive_one_card(mut self, one_card: bool) -> Self {
        self.split_aces_receive_one_card = one_card;
        self
    }

    /// Sets whether surrender is allowed.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_surrender(false);
    /// assert_eq!(options.surrender, false);
    /// ```
    #[must_use]
    pub const fn with_surrender(mut self, allowed: bool) -> Self {
        self.surrender = allowed;
        self
    }

    /// Sets whether insurance is offered.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_insurance(false);
    /// assert_eq!(options.insurance, false);
    /// ```
    #[must_use]
    pub const fn with_insurance(mut self, offered: bool) -> Self {
        self.insurance = offered;
        self
    }

    /// Sets the rounding mode for blackjack payouts.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::{GameOptions, RoundingMode};
    ///
    /// let options = GameOptions::default().with_rounding_blackjack(RoundingMode::Up);
    /// assert_eq!(options.rounding_blackjack, RoundingMode::Up);
    /// ```
    #[must_use]
    pub const fn with_rounding_blackjack(mut self, mode: RoundingMode) -> Self {
        self.rounding_blackjack = mode;
        self
    }

    /// Sets the rounding mode for surrender payouts.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::{GameOptions, RoundingMode};
    ///
    /// let options = GameOptions::default().with_rounding_surrender(RoundingMode::Down);
    /// assert_eq!(options.rounding_surrender, RoundingMode::Down);
    /// ```
    #[must_use]
    pub const fn with_rounding_surrender(mut self, mode: RoundingMode) -> Self {
        self.rounding_surrender = mode;
        self
    }

    /// Sets the deck penetration.
    ///
    /// # Example
    ///
    /// ```
    /// use bjrs::GameOptions;
    ///
    /// let options = GameOptions::default().with_penetration(0.80);
    /// assert_eq!(options.penetration, 0.80);
    /// ```
    #[must_use]
    pub const fn with_penetration(mut self, penetration: f64) -> Self {
        self.penetration = penetration;
        self
    }
}
