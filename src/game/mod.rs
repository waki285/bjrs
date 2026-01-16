//! Game engine and state management.

use core::sync::atomic::{AtomicU8, Ordering};

use alloc::vec::Vec;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use hashbrown::HashMap;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
#[cfg(feature = "std")]
use std::collections::HashMap;

use crate::sync::Mutex;

use crate::card::{Card, DECK_SIZE, Suit};
use crate::error::ReshuffleError;
use crate::hand::{DealerHand, Hand, HandStatus};
use crate::options::GameOptions;

mod actions;
mod bet;
mod dealer;
mod insurance;
pub mod state;

pub use state::{GameState, TurnPosition};

/// A blackjack game engine that manages players, betting, and round flow.
///
/// The game owns the shoe, player state, and dealer state. Use [`GameOptions`]
/// to configure rules such as decks, doubling rules, and payout rounding.
pub struct Game {
    /// Cards in the shoe.
    pub decks: Mutex<Vec<Card>>,
    /// Game options.
    pub options: GameOptions,
    /// Current game state.
    pub state: Mutex<GameState>,
    /// Next player ID to assign.
    next_id: AtomicU8,
    /// Active player IDs.
    pub players: Mutex<Vec<u8>>,
    /// Player money (`player_id` -> money amount).
    pub money: Mutex<HashMap<u8, usize>>,
    /// Player bets for current round (`player_id` -> bet amount).
    pub bets: Mutex<HashMap<u8, usize>>,
    /// Player hands (`player_id` -> list of hands for splits).
    pub hands: Mutex<HashMap<u8, Vec<Hand>>>,
    /// Dealer's hand.
    pub dealer_hand: Mutex<DealerHand>,
    /// Ordered list of players who bet this round.
    betting_order: Mutex<Vec<u8>>,
    /// Current turn position.
    current_turn: Mutex<TurnPosition>,
    /// Insurance bets (`player_id` -> insurance bet amount).
    insurance_bets: Mutex<HashMap<u8, usize>>,
    /// Players who have made their insurance decision.
    insurance_decided: Mutex<Vec<u8>>,
    /// Random number generator.
    rng: Mutex<ChaCha8Rng>,
}

impl Game {
    /// Creates a new game with the given seed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use bjrs::{Game, GameOptions};
    ///
    /// let options = GameOptions::default();
    /// let game = Game::new(options, 42);
    /// let _ = game;
    /// ```
    #[must_use]
    pub fn new(options: GameOptions, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let decks = Self::create_shoe(options.decks, &mut rng);

        Self {
            decks: Mutex::new(decks),
            options,
            state: Mutex::new(GameState::WaitingForPlayers),
            next_id: AtomicU8::new(0),
            players: Mutex::new(Vec::new()),
            money: Mutex::new(HashMap::new()),
            bets: Mutex::new(HashMap::new()),
            hands: Mutex::new(HashMap::new()),
            dealer_hand: Mutex::new(DealerHand::new()),
            betting_order: Mutex::new(Vec::new()),
            current_turn: Mutex::new(TurnPosition {
                player_index: 0,
                hand_index: 0,
            }),
            insurance_bets: Mutex::new(HashMap::new()),
            insurance_decided: Mutex::new(Vec::new()),
            rng: Mutex::new(rng),
        }
    }

    /// Creates and shuffles a shoe with the specified number of decks.
    fn create_shoe(num_decks: u8, rng: &mut ChaCha8Rng) -> Vec<Card> {
        let mut cards = Vec::with_capacity(num_decks as usize * DECK_SIZE);

        for _ in 0..num_decks {
            for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
                for rank in 1..=13 {
                    cards.push(Card::new(suit, rank));
                }
            }
        }

        cards.shuffle(rng);
        cards
    }

    /// Reshuffles the shoe.
    ///
    /// # Errors
    ///
    /// Returns an error if the game is in progress (not in `WaitingForPlayers` or Betting state).
    #[expect(
        clippy::significant_drop_tightening,
        reason = "locks are held for entire operation"
    )]
    pub fn reshuffle(&self) -> Result<(), ReshuffleError> {
        let state = *self.state.lock();
        if state != GameState::WaitingForPlayers && state != GameState::Betting {
            return Err(ReshuffleError::InvalidState);
        }

        let mut decks = self.decks.lock();
        let mut rng = self.rng.lock();

        *decks = Self::create_shoe(self.options.decks, &mut rng);

        Ok(())
    }

    /// Returns whether the shoe needs reshuffling based on penetration.
    ///
    /// Returns `true` if the remaining cards are below the penetration threshold.
    /// If penetration is 0, always returns `false`.
    pub fn needs_reshuffle(&self) -> bool {
        if self.options.penetration == 0.0 {
            return false;
        }

        let total_cards = self.options.decks as usize * DECK_SIZE;
        let remaining = self.cards_remaining();
        #[expect(
            clippy::cast_precision_loss,
            reason = "f64 has sufficient precision for card counts"
        )]
        let used_ratio = 1.0 - (remaining as f64 / total_cards as f64);

        used_ratio >= self.options.penetration
    }

    /// Checks penetration and reshuffles if needed.
    ///
    /// This should be called at the start of a new round (before betting or dealing).
    /// Returns `true` if a reshuffle was performed.
    ///
    /// # Errors
    ///
    /// Returns an error if the game is in progress.
    pub fn check_and_reshuffle(&self) -> Result<bool, ReshuffleError> {
        if self.needs_reshuffle() {
            self.reshuffle()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Draws a card from the shoe.
    fn draw(&self) -> Option<Card> {
        self.decks.lock().pop()
    }

    fn current_hand_inactive(&self) -> bool {
        let turn = self.current_turn.lock();
        let order = self.betting_order.lock();
        let hands = self.hands.lock();

        order
            .get(turn.player_index)
            .and_then(|player_id| hands.get(player_id))
            .and_then(|player_hands| player_hands.get(turn.hand_index))
            .is_some_and(|hand| hand.status() != HandStatus::Active)
    }

    fn advance_if_current_inactive(&self) {
        if self.current_hand_inactive() {
            self.advance_to_next_active_hand();
        }
    }

    /// Joins the game with the specified money amount.
    ///
    /// Returns the assigned player ID.
    pub fn join(&self, money: usize) -> u8 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.players.lock().push(id);
        self.money.lock().insert(id, money);
        id
    }

    /// Leaves the game.
    pub fn leave(&self, player_id: u8) {
        self.players.lock().retain(|&id| id != player_id);
        self.money.lock().remove(&player_id);
        self.bets.lock().remove(&player_id);
        self.hands.lock().remove(&player_id);
    }

    /// Returns the number of active players.
    pub fn player_count(&self) -> usize {
        self.players.lock().len()
    }

    /// Returns the number of cards remaining in the shoe.
    pub fn cards_remaining(&self) -> usize {
        self.decks.lock().len()
    }

    /// Starts the betting phase.
    pub fn start_betting(&self) {
        let mut state = self.state.lock();
        *state = GameState::Betting;
    }

    /// Returns the current game state.
    pub fn state(&self) -> GameState {
        *self.state.lock()
    }

    /// Returns the current turn position.
    pub fn current_turn(&self) -> TurnPosition {
        *self.current_turn.lock()
    }

    /// Returns the player ID whose turn it is.
    ///
    /// Returns `None` if there is no active turn (e.g., before dealing or after
    /// all hands have finished).
    pub fn current_player(&self) -> Option<u8> {
        let turn = self.current_turn.lock();
        let order = self.betting_order.lock();
        order.get(turn.player_index).copied()
    }

    /// Returns the current bet for the specified player.
    pub fn get_bet(&self, player_id: u8) -> Option<usize> {
        self.bets.lock().get(&player_id).copied()
    }

    /// Returns the current money for the specified player.
    pub fn get_money(&self, player_id: u8) -> Option<usize> {
        self.money.lock().get(&player_id).copied()
    }

    /// Returns the player's hands.
    ///
    /// Returns `None` if the player ID is not found.
    pub fn get_hands(&self, player_id: u8) -> Option<Vec<Hand>> {
        self.hands.lock().get(&player_id).cloned()
    }

    /// Returns a clone of the dealer's hand.
    pub fn get_dealer_hand(&self) -> DealerHand {
        self.dealer_hand.lock().clone()
    }

    /// Clears all hands and bets (called at the end of a round).
    ///
    /// This also resets the turn position and returns the game to the
    /// `WaitingForPlayers` state.
    pub fn clear_round(&self) {
        self.bets.lock().clear();
        self.hands.lock().clear();
        self.dealer_hand.lock().clear();
        self.betting_order.lock().clear();
        self.insurance_bets.lock().clear();
        self.insurance_decided.lock().clear();
        *self.current_turn.lock() = TurnPosition {
            player_index: 0,
            hand_index: 0,
        };
        *self.state.lock() = GameState::WaitingForPlayers;
    }
}
