//! A blackjack game engine with optional `no_std` support.
//!
//! The crate provides a [`Game`] type that manages the full round flow,
//! including betting, player actions, insurance, dealer play, and showdown.
//!
//! # Example
//!
//! ```no_run
//! use bjrs::{Game, GameOptions};
//!
//! let options = GameOptions::default();
//! let game = Game::new(options, 42);
//! let _ = game;
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!(
    "`std` is disabled but `alloc` feature is not enabled. Enable `alloc` or keep `std` enabled."
);

extern crate alloc;

pub mod card;
pub mod error;
pub mod game;
pub mod hand;
pub mod options;
pub mod result;
mod sync;

// Re-export main types
pub use card::{Card, DECK_SIZE, Suit};
pub use error::{ActionError, BetError, DealError, InsuranceError, ReshuffleError, ShowdownError};
pub use game::{Game, GameState, TurnPosition};
pub use hand::{DealerHand, Hand, HandStatus};
pub use options::{DoubleOption, GameOptions, RoundingMode};
pub use result::{HandOutcome, HandResult, PlayerResult, RoundResult};
