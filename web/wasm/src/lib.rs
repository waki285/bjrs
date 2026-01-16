use bjrs::{
    Card, Game, GameOptions, GameState, Hand, HandOutcome, HandStatus, PlayerResult, RoundResult,
    Suit,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
    player_id: Option<u8>,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32) -> Self {
        Self {
            game: Game::new(GameOptions::default(), seed as u64),
            player_id: None,
        }
    }

    pub fn reset(&mut self, seed: u32) {
        self.game = Game::new(GameOptions::default(), seed as u64);
        self.player_id = None;
    }

    pub fn join(&mut self, money: u32) -> u32 {
        if let Some(id) = self.player_id {
            return id as u32;
        }

        let id = self.game.join(money as usize);
        self.player_id = Some(id);
        id as u32
    }

    pub fn player_id(&self) -> Option<u32> {
        self.player_id.map(|id| id as u32)
    }

    pub fn start_betting(&self) {
        self.game.start_betting();
    }

    pub fn bet(&self, amount: u32) -> Result<(), JsValue> {
        let player_id = self.require_player()?;
        self.game
            .bet(player_id, amount as usize)
            .map_err(js_err)
    }

    pub fn deal(&self) -> Result<(), JsValue> {
        self.game.deal().map_err(js_err)
    }

    pub fn hit(&self, hand_index: u32) -> Result<(), JsValue> {
        let player_id = self.require_player()?;
        self.game
            .hit(player_id, hand_index as usize)
            .map(|_| ())
            .map_err(js_err)
    }

    pub fn stand(&self, hand_index: u32) -> Result<(), JsValue> {
        let player_id = self.require_player()?;
        self.game
            .stand(player_id, hand_index as usize)
            .map_err(js_err)
    }

    pub fn double_down(&self, hand_index: u32) -> Result<(), JsValue> {
        let player_id = self.require_player()?;
        self.game
            .double_down(player_id, hand_index as usize)
            .map(|_| ())
            .map_err(js_err)
    }

    pub fn split(&self, hand_index: u32) -> Result<(), JsValue> {
        let player_id = self.require_player()?;
        self.game
            .split(player_id, hand_index as usize)
            .map_err(js_err)
    }

    pub fn surrender(&self, hand_index: u32) -> Result<u32, JsValue> {
        let player_id = self.require_player()?;
        self.game
            .surrender(player_id, hand_index as usize)
            .map(|refund| refund as u32)
            .map_err(js_err)
    }

    pub fn take_insurance(&self) -> Result<u32, JsValue> {
        let player_id = self.require_player()?;
        self.game
            .take_insurance(player_id)
            .map(|bet| bet as u32)
            .map_err(js_err)
    }

    pub fn decline_insurance(&self) -> Result<(), JsValue> {
        let player_id = self.require_player()?;
        self.game
            .decline_insurance(player_id)
            .map_err(js_err)
    }

    pub fn finish_insurance(&self) -> Result<bool, JsValue> {
        self.game.finish_insurance().map_err(js_err)
    }

    pub fn dealer_play(&self) -> Result<(), JsValue> {
        self.game.dealer_play().map(|_| ()).map_err(js_err)
    }

    pub fn showdown(&self) -> Result<JsValue, JsValue> {
        let result = self.game.showdown().map_err(js_err)?;
        let js_result = JsRoundResult::from(result);
        to_js_value(&js_result)
    }

    pub fn clear_round(&self) {
        self.game.clear_round();
    }

    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        let state = self.game.state();
        let player_id = self.player_id;

        let (money, bet, hands, insurance_bet) = if let Some(id) = player_id {
            let money = self.game.get_money(id).map(|value| value as u32);
            let bet = self.game.get_bet(id).map(|value| value as u32);
            let hands = self
                .game
                .get_hands(id)
                .unwrap_or_default()
                .into_iter()
                .enumerate()
                .map(|(index, hand)| JsHand::from_hand(index as u32, &hand))
                .collect();
            let insurance_bet = self.game.get_insurance_bet(id).map(|value| value as u32);
            (money, bet, hands, insurance_bet)
        } else {
            (None, None, Vec::new(), None)
        };

        let dealer = JsDealer::from(self.game.get_dealer_hand());
        let turn = self.game.current_turn();
        let current_turn = self.game.current_player().map(|player_id| JsTurn {
            player_id: player_id as u32,
            hand_index: turn.hand_index as u32,
        });

        let snapshot = Snapshot {
            state: state_to_str(state),
            player_id: player_id.map(|id| id as u32),
            money,
            bet,
            hands,
            dealer,
            current_turn,
            insurance_offered: self.game.is_insurance_offered(),
            insurance_bet,
            cards_remaining: self.game.cards_remaining() as u32,
        };

        to_js_value(&snapshot)
    }
}

impl WasmGame {
    fn require_player(&self) -> Result<u8, JsValue> {
        self.player_id
            .ok_or_else(|| JsValue::from_str("player is not joined"))
    }
}

#[derive(Serialize)]
struct Snapshot {
    state: &'static str,
    player_id: Option<u32>,
    money: Option<u32>,
    bet: Option<u32>,
    hands: Vec<JsHand>,
    dealer: JsDealer,
    current_turn: Option<JsTurn>,
    insurance_offered: bool,
    insurance_bet: Option<u32>,
    cards_remaining: u32,
}

#[derive(Serialize)]
struct JsTurn {
    player_id: u32,
    hand_index: u32,
}

#[derive(Serialize)]
struct JsCard {
    suit: &'static str,
    rank: u8,
}

#[derive(Serialize)]
struct JsHand {
    index: u32,
    cards: Vec<JsCard>,
    value: u8,
    is_soft: bool,
    status: &'static str,
    bet: u32,
    from_split: bool,
    can_split: bool,
}

impl JsHand {
    fn from_hand(index: u32, hand: &Hand) -> Self {
        Self {
            index,
            cards: hand.cards().iter().copied().map(card_to_js).collect(),
            value: hand.value(),
            is_soft: hand.is_soft(),
            status: hand_status_to_str(hand.status()),
            bet: hand.bet() as u32,
            from_split: hand.is_from_split(),
            can_split: hand.can_split(),
        }
    }
}

#[derive(Serialize)]
struct JsDealer {
    cards: Vec<Option<JsCard>>,
    value: u8,
    visible_value: u8,
    is_soft: bool,
    is_blackjack: bool,
    is_bust: bool,
    hole_revealed: bool,
}

impl From<bjrs::DealerHand> for JsDealer {
    fn from(dealer: bjrs::DealerHand) -> Self {
        let hole_revealed = dealer.is_hole_revealed();
        let cards = dealer
            .cards()
            .iter()
            .enumerate()
            .map(|(index, card)| {
                if hole_revealed || index == 0 {
                    Some(card_to_js(*card))
                } else {
                    None
                }
            })
            .collect();

        Self {
            cards,
            value: dealer.value(),
            visible_value: dealer.visible_value(),
            is_soft: dealer.is_soft(),
            is_blackjack: dealer.is_blackjack(),
            is_bust: dealer.is_bust(),
            hole_revealed,
        }
    }
}

#[derive(Serialize)]
struct JsRoundResult {
    players: Vec<JsPlayerResult>,
    dealer_value: u8,
    dealer_bust: bool,
    dealer_blackjack: bool,
}

impl From<RoundResult> for JsRoundResult {
    fn from(result: RoundResult) -> Self {
        Self {
            players: result.players.into_iter().map(JsPlayerResult::from).collect(),
            dealer_value: result.dealer_value,
            dealer_bust: result.dealer_bust,
            dealer_blackjack: result.dealer_blackjack,
        }
    }
}

#[derive(Serialize)]
struct JsPlayerResult {
    player_id: u32,
    hands: Vec<JsHandResult>,
    total_payout: u32,
    net: i32,
    insurance_bet: u32,
    insurance_payout: u32,
}

impl From<PlayerResult> for JsPlayerResult {
    fn from(result: PlayerResult) -> Self {
        Self {
            player_id: result.player_id as u32,
            hands: result.hands.into_iter().map(JsHandResult::from).collect(),
            total_payout: result.total_payout as u32,
            net: result.net as i32,
            insurance_bet: result.insurance_bet as u32,
            insurance_payout: result.insurance_payout as u32,
        }
    }
}

#[derive(Serialize)]
struct JsHandResult {
    hand_index: u32,
    outcome: &'static str,
    bet: u32,
    payout: u32,
    player_value: u8,
    dealer_value: u8,
}

impl From<bjrs::HandResult> for JsHandResult {
    fn from(result: bjrs::HandResult) -> Self {
        Self {
            hand_index: result.hand_index as u32,
            outcome: outcome_to_str(result.outcome),
            bet: result.bet as u32,
            payout: result.payout as u32,
            player_value: result.player_value,
            dealer_value: result.dealer_value,
        }
    }
}

fn card_to_js(card: Card) -> JsCard {
    JsCard {
        suit: suit_to_str(card.suit),
        rank: card.rank,
    }
}

fn suit_to_str(suit: Suit) -> &'static str {
    match suit {
        Suit::Hearts => "Hearts",
        Suit::Diamonds => "Diamonds",
        Suit::Clubs => "Clubs",
        Suit::Spades => "Spades",
    }
}

fn state_to_str(state: GameState) -> &'static str {
    match state {
        GameState::WaitingForPlayers => "WaitingForPlayers",
        GameState::Betting => "Betting",
        GameState::Dealing => "Dealing",
        GameState::Insurance => "Insurance",
        GameState::PlayerTurn => "PlayerTurn",
        GameState::DealerTurn => "DealerTurn",
        GameState::RoundOver => "RoundOver",
    }
}

fn hand_status_to_str(status: HandStatus) -> &'static str {
    match status {
        HandStatus::Active => "Active",
        HandStatus::Stand => "Stand",
        HandStatus::Bust => "Bust",
        HandStatus::Blackjack => "Blackjack",
        HandStatus::Surrendered => "Surrendered",
    }
}

fn outcome_to_str(outcome: HandOutcome) -> &'static str {
    match outcome {
        HandOutcome::Win => "Win",
        HandOutcome::Lose => "Lose",
        HandOutcome::Push => "Push",
        HandOutcome::Blackjack => "Blackjack",
        HandOutcome::Surrendered => "Surrendered",
    }
}

fn js_err<E: core::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|err| JsValue::from_str(&err.to_string()))
}
