# bjrs

A blackjack game engine with optional `no_std` support.

## Features

- Full round flow: betting, player actions, insurance, dealer play, showdown
- Configurable rules via `GameOptions`
- Deterministic RNG seeded at game creation
- `std` by default, `no_std + alloc` supported (enable `alloc`)

## Usage

```rust
use bjrs::{Game, GameOptions};

let options = GameOptions::default();
let game = Game::new(options, 42);

let player_id = game.join(1_000);
game.start_betting();
game.bet(player_id, 50).unwrap();
game.deal().unwrap();

// Player actions, dealer play, and showdown omitted here.
```

See `examples/cli_blackjack.rs` for a complete playable CLI example.

## no_std

By default this crate uses `std`. To opt into `no_std`:

```toml
[dependencies]
bjrs = { version = "0.1", default-features = false, features = ["alloc"] }
```

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license
