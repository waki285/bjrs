import { useEffect, useState } from "react";
import init, { WasmGame } from "../wasm/pkg/bjrs_wasm";

type JsCard = {
  suit: string;
  rank: number;
};

type JsHand = {
  index: number;
  cards: JsCard[];
  value: number;
  is_soft: boolean;
  status: string;
  bet: number;
  from_split: boolean;
  can_split: boolean;
};

type JsDealer = {
  cards: Array<JsCard | null>;
  value: number;
  visible_value: number;
  is_soft: boolean;
  is_blackjack: boolean;
  is_bust: boolean;
  hole_revealed: boolean;
};

type JsTurn = {
  player_id: number;
  hand_index: number;
};

type Snapshot = {
  state: string;
  player_id: number | null;
  money: number | null;
  bet: number | null;
  hands: JsHand[];
  dealer: JsDealer;
  current_turn: JsTurn | null;
  insurance_offered: boolean;
  insurance_bet: number | null;
  cards_remaining: number;
};

type JsHandResult = {
  hand_index: number;
  outcome: string;
  bet: number;
  payout: number;
  player_value: number;
  dealer_value: number;
};

type JsPlayerResult = {
  player_id: number;
  hands: JsHandResult[];
  total_payout: number;
  net: number;
  insurance_bet: number;
  insurance_payout: number;
};

type JsRoundResult = {
  players: JsPlayerResult[];
  dealer_value: number;
  dealer_bust: boolean;
  dealer_blackjack: boolean;
};

const makeSeed = () => Math.floor(Date.now() % 2 ** 32);

const rankLabel = (rank: number) => {
  if (rank === 1) return "A";
  if (rank === 11) return "J";
  if (rank === 12) return "Q";
  if (rank === 13) return "K";
  return String(rank);
};

const suitLabel = (suit: string) => {
  if (suit === "Hearts") return "♥";
  if (suit === "Diamonds") return "♦";
  if (suit === "Clubs") return "♣";
  if (suit === "Spades") return "♠";
  return suit.slice(0, 1).toUpperCase();
};

const formatCard = (card: JsCard | null) => {
  if (!card) return "??";
  return `${rankLabel(card.rank)}${suitLabel(card.suit)}`;
};

const cardClass = (card: JsCard | null) => {
  if (!card) return "card hidden";
  const suit = card.suit.toLowerCase();
  return `card ${suit}`;
};

export default function App() {
  const [ready, setReady] = useState(false);
  const [game, setGame] = useState<WasmGame | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [result, setResult] = useState<JsRoundResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [buyIn, setBuyIn] = useState(1000);
  const [bet, setBet] = useState(50);
  const [seed, setSeed] = useState(() => makeSeed());

  const isPlayerTurn = snapshot?.state === "PlayerTurn";
  const activeHandIndex = snapshot?.current_turn?.hand_index ?? 0;

  useEffect(() => {
    let cancelled = false;
    init()
      .then(() => {
        if (cancelled) return;
        const instance = new WasmGame(seed);
        setGame(instance);
        setReady(true);
        refresh(instance);
      })
      .catch((err) => {
        setError(`init failed: ${String(err)}`);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const refresh = (instance: WasmGame) => {
    const next = instance.snapshot() as Snapshot;
    setSnapshot(next);
    return next;
  };

  const autoResolveIfNeeded = (
    instance: WasmGame,
    current: Snapshot,
    hasResult: boolean,
  ) => {
    let snapshot = current;
    if (snapshot.state === "DealerTurn") {
      instance.dealer_play();
      snapshot = refresh(instance);
    }
    if (snapshot.state === "RoundOver" && !hasResult) {
      const round = instance.showdown() as JsRoundResult;
      setResult(round);
      snapshot = refresh(instance);
    }
  };

  const withGame = (
    fn: (instance: WasmGame) => void,
    options?: { autoResolve?: boolean },
  ) => {
    if (!game) return;
    try {
      fn(game);
      setError(null);
      const next = refresh(game);
      if (options?.autoResolve ?? true) {
        autoResolveIfNeeded(game, next, result !== null);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  const handleJoin = () => {
    if (snapshot?.player_id !== null && snapshot?.player_id !== undefined) {
      return;
    }
    withGame((instance) => {
      instance.join(buyIn);
      instance.start_betting();
      setResult(null);
    });
  };

  const handleReset = () => {
    if (!game) return;
    const nextSeed = makeSeed();
    setSeed(nextSeed);
    game.reset(nextSeed);
    setResult(null);
    refresh(game);
  };

  const handleBetAndDeal = () => {
    withGame((instance) => {
      instance.bet(bet);
      instance.deal();
      setResult(null);
    });
  };

  const handleInsurance = (take: boolean) => {
    withGame((instance) => {
      if (take) {
        instance.take_insurance();
      } else {
        instance.decline_insurance();
      }
      instance.finish_insurance();
    });
  };

  const handleDealerPlay = () => {
    withGame((instance) => {
      instance.dealer_play();
    });
  };

  const handleShowdown = () => {
    if (result) return;
    withGame((instance) => {
      const result = instance.showdown() as JsRoundResult;
      setResult(result);
    }, { autoResolve: false });
  };

  const handleNextRound = () => {
    withGame((instance) => {
      instance.clear_round();
      instance.start_betting();
      setResult(null);
    });
  };

  const isActivePlayer = snapshot?.player_id === snapshot?.current_turn?.player_id;
  const isActiveHand = isPlayerTurn && isActivePlayer;
  const actionButtons = isActiveHand ? (
    <div className="actions">
      <button type="button" onClick={() => withGame((g) => g.hit(activeHandIndex))}>
        Hit
      </button>
      <button type="button" onClick={() => withGame((g) => g.stand(activeHandIndex))}>
        Stand
      </button>
      <button type="button" onClick={() => withGame((g) => g.double_down(activeHandIndex))}>
        Double
      </button>
      <button type="button" onClick={() => withGame((g) => g.split(activeHandIndex))}>
        Split
      </button>
      <button type="button" onClick={() => withGame((g) => g.surrender(activeHandIndex))}>
        Surrender
      </button>
    </div>
  ) : null;

  if (!ready) {
    return (
      <div className="app">
        <div className="hero">
          <h1>bjrs web</h1>
          <p>Loading wasm...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      <header className="hero">
        <div>
          <h1>bjrs web</h1>
          <p className="subtitle">Blackjack engine running in wasm</p>
        </div>
        <div className="seed">
          <span className="label">Seed</span>
          <span className="value">{seed}</span>
          <button type="button" className="ghost" onClick={handleReset}>
            New seed
          </button>
        </div>
      </header>

      <main className="layout">
        <section className="panel">
          <h2>Session</h2>
          <div className="field">
            <label htmlFor="buy-in">Buy in</label>
            <input
              id="buy-in"
              type="number"
              min={1}
              value={buyIn}
              onChange={(event) => setBuyIn(Number(event.target.value))}
            />
            <button
              type="button"
              onClick={handleJoin}
              disabled={snapshot?.player_id !== null && snapshot?.player_id !== undefined}
            >
              Join + Start betting
            </button>
          </div>

          <div className="field">
            <label htmlFor="bet">Bet</label>
            <input
              id="bet"
              type="number"
              min={1}
              value={bet}
              onChange={(event) => setBet(Number(event.target.value))}
            />
            <button type="button" onClick={handleBetAndDeal}>
              Place bet + Deal
            </button>
          </div>

          <div className="status">
            <div>
              <span className="label">State</span>
              <span className="chip">{snapshot?.state ?? "-"}</span>
            </div>
            <div>
              <span className="label">Money</span>
              <span className="chip">{snapshot?.money ?? "-"}</span>
            </div>
            <div>
              <span className="label">Bet</span>
              <span className="chip">{snapshot?.bet ?? "-"}</span>
            </div>
            <div>
              <span className="label">Cards left</span>
              <span className="chip">{snapshot?.cards_remaining ?? "-"}</span>
            </div>
          </div>

          {snapshot?.insurance_offered && (
            <div className="insurance">
              <p>Insurance is offered.</p>
              <div className="actions">
                <button type="button" onClick={() => handleInsurance(true)}>
                  Take insurance
                </button>
                <button type="button" className="secondary" onClick={() => handleInsurance(false)}>
                  Decline
                </button>
              </div>
            </div>
          )}

          {error && (
            <div className="error">
              <span>Error:</span> {error}
            </div>
          )}
        </section>

        <section className="table">
          <div className="table-header">
            <h2>Table</h2>
            <div className="table-actions">
              {snapshot?.state === "DealerTurn" && (
                <button type="button" onClick={handleDealerPlay}>
                  Dealer play
                </button>
              )}
              {snapshot?.state === "RoundOver" && (
                <>
                  {!result && (
                    <button type="button" onClick={handleShowdown}>
                      Showdown
                    </button>
                  )}
                  <button type="button" className="secondary" onClick={handleNextRound}>
                    Next round
                  </button>
                </>
              )}
            </div>
          </div>

          <div className="dealer">
            <div className="section-title">Dealer</div>
            <div className="cards">
              {snapshot?.dealer.cards.map((card, index) => (
                <div className={cardClass(card)} key={`dealer-${index}`}>
                  {formatCard(card)}
                </div>
              ))}
            </div>
            <div className="meta">
              <span>Visible: {snapshot?.dealer.visible_value ?? 0}</span>
              <span>Value: {snapshot?.dealer.hole_revealed ? snapshot?.dealer.value : "?"}</span>
              <span>Soft: {snapshot?.dealer.is_soft ? "yes" : "no"}</span>
              <span>Bust: {snapshot?.dealer.is_bust ? "yes" : "no"}</span>
            </div>
          </div>

          <div className="player">
            <div className="section-title">Player</div>
            {snapshot?.hands.length === 0 ? (
              <div className="empty">No hands yet.</div>
            ) : (
              snapshot?.hands.map((hand) => {
                const isActive =
                  snapshot.current_turn?.hand_index === hand.index &&
                  snapshot.player_id === snapshot.current_turn?.player_id &&
                  snapshot.state === "PlayerTurn";
                return (
                  <div
                    className={`hand ${isActive ? "active" : ""}`}
                    key={`hand-${hand.index}`}
                  >
                    <div className="hand-header">
                      <span>Hand {hand.index + 1}</span>
                      <span className="chip">{hand.status}</span>
                    </div>
                    <div className="cards">
                      {hand.cards.map((card, index) => (
                        <div className={cardClass(card)} key={`hand-${hand.index}-${index}`}>
                          {formatCard(card)}
                        </div>
                      ))}
                    </div>
                    <div className="meta">
                      <span>Value: {hand.value}</span>
                      <span>Bet: {hand.bet}</span>
                      <span>Soft: {hand.is_soft ? "yes" : "no"}</span>
                      <span>Split: {hand.from_split ? "yes" : "no"}</span>
                    </div>
                  </div>
                );
              })
            )}

            {actionButtons}
          </div>
        </section>

        <section className="panel results">
          <h2>Round result</h2>
          {!result ? (
            <p className="muted">No results yet.</p>
          ) : (
            result.players.map((player) => (
              <div key={`result-${player.player_id}`} className="result-card">
                <div className="result-header">
                  <span>Player {player.player_id}</span>
                  <span className={`chip ${player.net >= 0 ? "win" : "lose"}`}>
                    Net {player.net}
                  </span>
                </div>
                <div className="result-meta">
                  <span>Total payout: {player.total_payout}</span>
                  <span>Insurance bet: {player.insurance_bet}</span>
                  <span>Insurance payout: {player.insurance_payout}</span>
                </div>
                {player.hands.map((hand) => (
                  <div className="result-hand" key={`result-hand-${hand.hand_index}`}>
                    <span>Hand {hand.hand_index + 1}</span>
                    <span>{hand.outcome}</span>
                    <span>Bet {hand.bet}</span>
                    <span>Payout {hand.payout}</span>
                  </div>
                ))}
              </div>
            ))
          )}
        </section>
      </main>
    </div>
  );
}
