//! CLI blackjack example.

#![allow(clippy::missing_docs_in_private_items)]

use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use bjrs::{Card, DoubleOption, Game, GameOptions, GameState, Hand, HandStatus, Suit};

fn main() {
    println!("Blackjack CLI example (type 'q' to quit)");

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let options = GameOptions::default();
    let game = Game::new(options, seed);

    let player_id = game.join(500);

    loop {
        let money = game.get_money(player_id).unwrap_or(0);
        if money == 0 {
            println!("You are out of money. Game over.");
            break;
        }

        if game.check_and_reshuffle() == Ok(true) {
            println!("Shoe reshuffled.");
        }

        game.start_betting();

        let Some(bet) = prompt_usize(&format!("Bet amount (1-{money}, 0 to quit): ")) else {
            break;
        };

        if bet == 0 {
            println!("Goodbye.");
            break;
        }

        if let Err(err) = game.bet(player_id, bet) {
            println!("Bet error: {err:?}");
            continue;
        }

        if let Err(err) = game.deal() {
            println!("Deal error: {err:?}");
            game.clear_round();
            continue;
        }

        if game.is_insurance_offered() {
            println!("Dealer shows an Ace. Insurance offered.");
            match prompt_line("Take insurance? (y/n): ").as_str() {
                "y" | "yes" => match game.take_insurance(player_id) {
                    Ok(amount) => println!("Insurance bet placed: {amount}"),
                    Err(err) => println!("Insurance error: {err:?}"),
                },
                _ => {
                    if let Err(err) = game.decline_insurance(player_id) {
                        println!("Insurance error: {err:?}");
                    }
                }
            }

            match game.finish_insurance() {
                Ok(true) => {
                    println!("Dealer has blackjack.");
                }
                Ok(false) => {}
                Err(err) => {
                    println!("Insurance finish error: {err:?}");
                }
            }
        }

        // If there is no active player turn (e.g., initial blackjack), move to dealer.
        if *game.state.lock() == GameState::PlayerTurn && game.current_player().is_none() {
            *game.state.lock() = GameState::DealerTurn;
        }

        while *game.state.lock() == GameState::PlayerTurn {
            print_table(&game, player_id);

            println!("{}", format_actions(&game, player_id));
            let action = prompt_line("Action: ");
            let turn = game.current_turn();

            let result = match action.as_str() {
                "h" | "hit" => game.hit(player_id, turn.hand_index).map(|_| ()),
                "s" | "stand" => game.stand(player_id, turn.hand_index),
                "d" | "double" => game.double_down(player_id, turn.hand_index).map(|_| ()),
                "p" | "split" => game.split(player_id, turn.hand_index),
                "u" | "surrender" => game.surrender(player_id, turn.hand_index).map(|_| ()),
                "q" | "quit" => return,
                _ => {
                    println!("Unknown action.");
                    continue;
                }
            };

            if let Err(err) = result {
                println!("Action error: {err:?}");
            }
        }

        if *game.state.lock() == GameState::DealerTurn {
            match game.dealer_play() {
                Ok(drawn) => {
                    if !drawn.is_empty() {
                        println!("Dealer draws {} card(s).", drawn.len());
                    }
                }
                Err(err) => println!("Dealer error: {err:?}"),
            }
        }

        if *game.state.lock() == GameState::RoundOver {
            match game.showdown() {
                Ok(result) => {
                    print_table_final(&game, player_id);
                    println!("Round complete.");
                    for player in result.players {
                        if player.player_id == player_id {
                            println!("Payout: {} (net {})", player.total_payout, player.net);
                            if player.insurance_bet > 0 {
                                println!("Insurance payout: {}", player.insurance_payout);
                            }
                        }
                    }
                }
                Err(err) => println!("Showdown error: {err:?}"),
            }
        }

        game.clear_round();

        // Always continue to the next round without prompting.
    }
}

fn prompt_line(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return String::new();
    }
    input.trim().to_lowercase()
}

fn prompt_usize(prompt: &str) -> Option<usize> {
    loop {
        let input = prompt_line(prompt);
        if input == "q" || input == "quit" {
            return None;
        }
        match input.parse::<usize>() {
            Ok(value) => return Some(value),
            Err(_) => println!("Please enter a number."),
        }
    }
}

fn print_table(game: &Game, player_id: u8) {
    let remaining = game.cards_remaining();
    println!("\nShoe: {remaining} cards remaining");

    let dealer = game.get_dealer_hand();
    let dealer_view = format_dealer(&dealer);
    let dealer_value = if dealer.is_hole_revealed() {
        dealer.value()
    } else {
        dealer.visible_value()
    };
    println!("\nDealer: {dealer_view} (value {dealer_value})");

    let hands = game.get_hands(player_id).unwrap_or_default();
    let turn = game.current_turn();
    for (index, hand) in hands.iter().enumerate() {
        let marker = if index == turn.hand_index { "*" } else { " " };
        println!(
            "{} Hand {}: {} | value {} | bet {} | {:?}",
            marker,
            index,
            format_hand(hand),
            hand.value(),
            hand.bet(),
            hand.status()
        );
    }
    println!();
}

fn print_table_final(game: &Game, player_id: u8) {
    let remaining = game.cards_remaining();
    println!("\nShoe: {remaining} cards remaining");

    let dealer = game.get_dealer_hand();
    println!(
        "\nDealer: {} (value {})",
        format_dealer(&dealer),
        dealer.value()
    );

    let hands = game.get_hands(player_id).unwrap_or_default();
    for (index, hand) in hands.iter().enumerate() {
        println!(
            "Hand {}: {} | value {} | bet {} | {:?}",
            index,
            format_hand(hand),
            hand.value(),
            hand.bet(),
            hand.status()
        );
    }
    println!();
}

fn format_actions(game: &Game, player_id: u8) -> String {
    let availability = available_actions(game, player_id);
    let mut parts = Vec::new();
    parts.push(format_action("hit", "h", availability.hit));
    parts.push(format_action("stand", "s", availability.stand));
    parts.push(format_action("double", "d", availability.double));
    parts.push(format_action("split", "p", availability.split));
    parts.push(format_action("surrender", "u", availability.surrender));
    format!("Actions: {}", parts.join(" "))
}

fn format_action(label: &str, key: &str, allowed: bool) -> String {
    let text = format!("[{key}]{label}");
    if allowed {
        colorize(&text, "32")
    } else {
        colorize(&text, "90")
    }
}

fn colorize(text: &str, code: &str) -> String {
    format!("\u{1b}[{code}m{text}\u{1b}[0m")
}

struct ActionAvailability {
    hit: bool,
    stand: bool,
    double: bool,
    split: bool,
    surrender: bool,
}

fn available_actions(game: &Game, player_id: u8) -> ActionAvailability {
    if *game.state.lock() != GameState::PlayerTurn {
        return ActionAvailability {
            hit: false,
            stand: false,
            double: false,
            split: false,
            surrender: false,
        };
    }

    if game.current_player() != Some(player_id) {
        return ActionAvailability {
            hit: false,
            stand: false,
            double: false,
            split: false,
            surrender: false,
        };
    }

    let hands = game.get_hands(player_id).unwrap_or_default();
    let turn = game.current_turn();
    let Some(hand) = hands.get(turn.hand_index) else {
        return ActionAvailability {
            hit: false,
            stand: false,
            double: false,
            split: false,
            surrender: false,
        };
    };

    if hand.status() != HandStatus::Active {
        return ActionAvailability {
            hit: false,
            stand: false,
            double: false,
            split: false,
            surrender: false,
        };
    }

    let money = game.get_money(player_id).unwrap_or(0);
    let bet = hand.bet();
    let has_funds_for_double = money >= bet;
    let has_funds_for_split = money >= bet;

    let can_double_value = match game.options.double {
        DoubleOption::Any => true,
        DoubleOption::NineOrTen => hand.value() == 9 || hand.value() == 10,
        DoubleOption::NineThrough11 => (9..=11).contains(&hand.value()),
        DoubleOption::NineThrough15 => (9..=15).contains(&hand.value()),
        DoubleOption::None => false,
        _ => panic!("unhandled double option"),
    };

    let can_double = hand.len() == 2
        && (!hand.is_from_split() || game.options.double_after_split)
        && can_double_value
        && has_funds_for_double;

    let is_ace = hand.cards().first().is_some_and(|c| c.rank == 1);
    let max_splits_reached = hands.len() > game.options.split as usize;
    let can_split = hand.can_split()
        && !max_splits_reached
        && has_funds_for_split
        && !(is_ace && hand.is_from_split() && game.options.split_aces_only_once);

    let can_surrender = game.options.surrender && hand.len() == 2 && !hand.is_from_split();

    ActionAvailability {
        hit: true,
        stand: true,
        double: can_double,
        split: can_split,
        surrender: can_surrender,
    }
}

fn format_dealer(dealer: &bjrs::DealerHand) -> String {
    if dealer.cards().is_empty() {
        return "(no cards)".to_string();
    }

    if dealer.is_hole_revealed() {
        dealer
            .cards()
            .iter()
            .map(format_card)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        let mut parts = Vec::new();
        if let Some(card) = dealer.up_card() {
            parts.push(format_card(card));
        }
        if dealer.len() > 1 {
            parts.push("??".to_string());
        }
        parts.join(" ")
    }
}

fn format_hand(hand: &Hand) -> String {
    if hand.is_empty() {
        return "(empty)".to_string();
    }
    hand.cards()
        .iter()
        .map(format_card)
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_card(card: &Card) -> String {
    let (suit, color_code) = match card.suit {
        Suit::Hearts => ("H", "31"),
        Suit::Diamonds => ("D", "31"),
        Suit::Clubs => ("C", "32"),
        Suit::Spades => ("S", "34"),
    };

    let (rank, is_face) = match card.rank {
        1 => ("A".to_string(), true),
        11 => ("J".to_string(), true),
        12 => ("Q".to_string(), true),
        13 => ("K".to_string(), true),
        _ => (card.rank.to_string(), false),
    };

    let colored_rank = if is_face {
        colorize(&rank, color_code)
    } else {
        rank
    };
    let colored_suit = colorize(suit, color_code);
    format!("{colored_rank}{colored_suit}")
}
