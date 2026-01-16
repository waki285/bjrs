//! Game integration tests.

#![allow(clippy::float_cmp)]

use bjrs::{
    ActionError, BetError, Card, DECK_SIZE, DealError, DoubleOption, Game, GameOptions, GameState,
    Hand, HandStatus, InsuranceError, RoundingMode, ShowdownError, Suit,
};

const fn card(suit: Suit, rank: u8) -> Card {
    Card::new(suit, rank)
}

fn set_deck_from_draws(game: &Game, draws: &[Card]) {
    let mut deck: Vec<Card> = draws.to_vec();
    deck.reverse();
    *game.decks.lock() = deck;
}

#[test]
fn hand_blackjack_and_split_behavior() {
    let mut hand = Hand::new(10);
    hand.add_card(card(Suit::Hearts, 1));
    hand.add_card(card(Suit::Spades, 13));
    assert_eq!(hand.value(), 21);
    assert_eq!(hand.status(), HandStatus::Blackjack);
    assert!(hand.is_soft());

    let mut split_hand = Hand::from_split(card(Suit::Hearts, 1), 10);
    split_hand.add_card(card(Suit::Clubs, 13));
    assert_eq!(split_hand.value(), 21);
    assert_eq!(split_hand.status(), HandStatus::Active);

    let mut bust_hand = Hand::new(5);
    bust_hand.add_card(card(Suit::Hearts, 10));
    bust_hand.add_card(card(Suit::Spades, 10));
    bust_hand.add_card(card(Suit::Diamonds, 2));
    assert_eq!(bust_hand.status(), HandStatus::Bust);
}

#[test]
fn dealer_hand_visibility_and_values() {
    let mut dealer = bjrs::DealerHand::new();
    dealer.add_card(card(Suit::Hearts, 1));
    dealer.add_card(card(Suit::Clubs, 6));

    assert!(!dealer.is_hole_revealed());
    assert_eq!(dealer.visible_value(), 11);

    dealer.reveal_hole();
    assert!(dealer.is_hole_revealed());
    assert_eq!(dealer.visible_value(), 17);
    assert!(dealer.is_soft());
}

#[test]
fn options_builder_sets_fields() {
    let options = GameOptions::default()
        .with_decks(4)
        .with_blackjack_pays(1.2)
        .with_stand_on_soft_17(false)
        .with_double(DoubleOption::NineOrTen)
        .with_split(1)
        .with_double_after_split(false)
        .with_split_aces_only_once(false)
        .with_split_aces_receive_one_card(false)
        .with_surrender(false)
        .with_insurance(false)
        .with_rounding_blackjack(RoundingMode::Up)
        .with_rounding_surrender(RoundingMode::Down)
        .with_penetration(0.5);

    assert_eq!(options.decks, 4);
    assert_eq!(options.blackjack_pays, 1.2);
    assert!(!options.stand_on_soft_17);
    assert_eq!(options.double, DoubleOption::NineOrTen);
    assert_eq!(options.split, 1);
    assert!(!options.double_after_split);
    assert!(!options.split_aces_only_once);
    assert!(!options.split_aces_receive_one_card);
    assert!(!options.surrender);
    assert!(!options.insurance);
    assert_eq!(options.rounding_blackjack, RoundingMode::Up);
    assert_eq!(options.rounding_surrender, RoundingMode::Down);
    assert_eq!(options.penetration, 0.5);
}

#[test]
fn reshuffle_when_penetration_reached() {
    let options = GameOptions::default().with_decks(1).with_penetration(0.5);
    let game = Game::new(options, 1);
    *game.decks.lock() = vec![card(Suit::Hearts, 2); 10];

    assert!(game.needs_reshuffle());
    assert!(game.check_and_reshuffle().unwrap());
    assert_eq!(game.cards_remaining(), DECK_SIZE);
}

#[test]
fn bet_errors() {
    let options = GameOptions::default().with_insurance(false);
    let game = Game::new(options, 1);
    let player = game.join(10);

    assert_eq!(game.bet(player, 5).unwrap_err(), BetError::InvalidState);

    game.start_betting();
    assert_eq!(game.bet(player, 0).unwrap_err(), BetError::ZeroBet);
    assert_eq!(
        game.bet(player, 20).unwrap_err(),
        BetError::InsufficientFunds
    );
    assert_eq!(
        game.bet(player + 1, 1).unwrap_err(),
        BetError::PlayerNotFound
    );
}

#[test]
fn deal_errors() {
    let options = GameOptions::default().with_insurance(false);
    let game = Game::new(options, 1);

    assert_eq!(game.deal().unwrap_err(), DealError::InvalidState);

    game.start_betting();
    assert_eq!(game.deal().unwrap_err(), DealError::NoBets);

    let player = game.join(10);
    game.bet(player, 5).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 9),
            card(Suit::Clubs, 5),
            card(Suit::Diamonds, 7),
        ],
    );

    assert_eq!(game.deal().unwrap_err(), DealError::NotEnoughCards);
}

#[test]
fn hit_with_empty_shoe_returns_error() {
    let options = GameOptions::default().with_insurance(false);
    let game = Game::new(options, 7);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 5),   // player
            card(Suit::Clubs, 9),    // dealer up
            card(Suit::Spades, 6),   // player
            card(Suit::Diamonds, 7), // dealer hole
        ],
    );

    game.deal().unwrap();
    assert_eq!(*game.state.lock(), GameState::PlayerTurn);

    assert_eq!(game.hit(player, 0).unwrap_err(), ActionError::NoCards);
}

#[test]
fn basic_round_flow() {
    let options = GameOptions::default().with_insurance(false);
    let game = Game::new(options, 42);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 8),   // player
            card(Suit::Clubs, 6),    // dealer up
            card(Suit::Diamonds, 7), // player
            card(Suit::Spades, 10),  // dealer hole
            card(Suit::Hearts, 4),   // player hit
            card(Suit::Clubs, 5),    // dealer draw
        ],
    );

    game.deal().unwrap();
    assert_eq!(*game.state.lock(), GameState::PlayerTurn);

    let hit_card = game.hit(player, 0).unwrap();
    assert_eq!(hit_card.rank, 4);

    game.stand(player, 0).unwrap();
    assert_eq!(*game.state.lock(), GameState::DealerTurn);

    let drawn = game.dealer_play().unwrap();
    assert_eq!(drawn.len(), 1);
    assert_eq!(*game.state.lock(), GameState::RoundOver);

    let result = game.showdown().unwrap();
    assert_eq!(result.players.len(), 1);
    assert_eq!(result.dealer_value, 21);
    assert_eq!(game.get_money(player), Some(90));
}

#[test]
fn insurance_flow_with_dealer_blackjack() {
    let options = GameOptions::default().with_insurance(true);
    let game = Game::new(options, 99);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 9),
            card(Suit::Spades, 1),
            card(Suit::Diamonds, 7),
            card(Suit::Clubs, 10),
        ],
    );

    game.deal().unwrap();
    assert_eq!(*game.state.lock(), GameState::Insurance);
    assert!(game.is_insurance_offered());

    let insurance_bet = game.take_insurance(player).unwrap();
    assert_eq!(insurance_bet, 5);

    let dealer_blackjack = game.finish_insurance().unwrap();
    assert!(dealer_blackjack);
    assert_eq!(*game.state.lock(), GameState::RoundOver);

    let result = game.showdown().unwrap();
    assert_eq!(result.players[0].insurance_payout, 15);
    assert_eq!(game.get_money(player), Some(100));
}

#[test]
fn insurance_keeps_player_turn_when_active() {
    let options = GameOptions::default().with_insurance(true);
    let game = Game::new(options, 77);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 7),   // player
            card(Suit::Spades, 1),   // dealer up (Ace)
            card(Suit::Clubs, 8),    // player
            card(Suit::Diamonds, 9), // dealer hole (no blackjack)
        ],
    );

    game.deal().unwrap();
    assert_eq!(*game.state.lock(), GameState::Insurance);

    game.take_insurance(player).unwrap();
    let dealer_blackjack = game.finish_insurance().unwrap();
    assert!(!dealer_blackjack);
    assert_eq!(*game.state.lock(), GameState::PlayerTurn);
    assert_eq!(game.current_player(), Some(player));
}

#[test]
fn double_down_allowed_and_updates_bet() {
    let options = GameOptions::default()
        .with_insurance(false)
        .with_double(DoubleOption::NineOrTen);
    let game = Game::new(options, 5);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 5),   // player
            card(Suit::Clubs, 2),    // dealer up
            card(Suit::Diamonds, 4), // player
            card(Suit::Spades, 3),   // dealer hole
            card(Suit::Hearts, 10),  // double draw
        ],
    );

    game.deal().unwrap();
    let drawn = game.double_down(player, 0).unwrap();
    assert_eq!(drawn.rank, 10);
    assert_eq!(*game.state.lock(), GameState::DealerTurn);

    let hands = game.get_hands(player).unwrap();
    assert_eq!(hands[0].bet(), 20);
}

#[test]
fn double_down_rejected_when_value_not_allowed() {
    let options = GameOptions::default()
        .with_insurance(false)
        .with_double(DoubleOption::NineOrTen);
    let game = Game::new(options, 6);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 5),   // player
            card(Suit::Clubs, 2),    // dealer up
            card(Suit::Diamonds, 3), // player
            card(Suit::Spades, 7),   // dealer hole
        ],
    );

    game.deal().unwrap();
    assert_eq!(
        game.double_down(player, 0).unwrap_err(),
        ActionError::CannotDouble
    );
}

#[test]
fn split_creates_two_hands() {
    let options = GameOptions::default().with_insurance(false);
    let game = Game::new(options, 11);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 8),   // player
            card(Suit::Clubs, 5),    // dealer up
            card(Suit::Diamonds, 8), // player
            card(Suit::Spades, 9),   // dealer hole
            card(Suit::Hearts, 2),   // split hand 1 draw
            card(Suit::Clubs, 3),    // split hand 2 draw
        ],
    );

    game.deal().unwrap();
    game.split(player, 0).unwrap();

    let hands = game.get_hands(player).unwrap();
    assert_eq!(hands.len(), 2);
    assert_eq!(hands[0].len(), 2);
    assert_eq!(hands[1].len(), 2);
    assert_eq!(game.get_money(player), Some(80));
}

#[test]
fn surrender_refunds_half_bet() {
    let options = GameOptions::default()
        .with_insurance(false)
        .with_surrender(true);
    let game = Game::new(options, 21);
    let player = game.join(100);

    game.start_betting();
    game.bet(player, 10).unwrap();

    set_deck_from_draws(
        &game,
        &[
            card(Suit::Hearts, 10),  // player
            card(Suit::Clubs, 7),    // dealer up
            card(Suit::Diamonds, 6), // player
            card(Suit::Spades, 8),   // dealer hole
        ],
    );

    game.deal().unwrap();
    let refund = game.surrender(player, 0).unwrap();
    assert_eq!(refund, 5);
    assert_eq!(game.get_money(player), Some(95));
    assert_eq!(*game.state.lock(), GameState::DealerTurn);
}

#[test]
fn showdown_rejects_wrong_state() {
    let game = Game::new(GameOptions::default(), 1);
    assert_eq!(game.showdown().unwrap_err(), ShowdownError::InvalidState);
}

#[test]
fn insurance_rejects_wrong_state() {
    let game = Game::new(GameOptions::default(), 1);
    assert_eq!(
        game.take_insurance(0).unwrap_err(),
        InsuranceError::InvalidState
    );
}
