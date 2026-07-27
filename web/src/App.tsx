import { useEffect, useState } from "react";
import init, { type WasmGameOptions, WasmGame } from "../wasm/pkg/bjrs_wasm";

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

type PhaseCopy = {
  title: string;
};

type TableRules = Required<WasmGameOptions>;
type DoubleRule = TableRules["double"];
type RoundingRule = TableRules["roundingBlackjack"];

const makeSeed = () => Math.floor(Date.now() % 2 ** 32);

const defaultTableRules: TableRules = {
  decks: 2,
  blackjackPays: 1.5,
  standOnSoft17: true,
  double: "any",
  split: 3,
  doubleAfterSplit: true,
  splitAcesOnlyOnce: true,
  splitAcesReceiveOneCard: true,
  insurance: true,
  surrender: true,
  roundingBlackjack: "down",
  roundingSurrender: "nearest",
  penetration: 0.75,
};

const deckOptions = [1, 2, 4, 6, 8];

const blackjackPayoutOptions = [
  { value: 1, label: "1:1" },
  { value: 1.2, label: "6:5" },
  { value: 1.5, label: "3:2" },
  { value: 2, label: "2:1" },
];

const doubleRuleOptions: Array<{ value: DoubleRule; label: string }> = [
  { value: "any", label: "Any total" },
  { value: "nineOrTen", label: "9 or 10" },
  { value: "nineThrough11", label: "9 through 11" },
  { value: "nineThrough15", label: "9 through 15" },
  { value: "none", label: "Never" },
];

const splitOptions = [
  { value: 0, label: "None" },
  { value: 1, label: "1" },
  { value: 2, label: "2" },
  { value: 3, label: "3" },
  { value: 4, label: "4" },
];

const roundingOptions: Array<{ value: RoundingRule; label: string }> = [
  { value: "up", label: "Up" },
  { value: "down", label: "Down" },
  { value: "nearest", label: "Nearest" },
];

const penetrationOptions = [
  { value: 0, label: "Never" },
  { value: 0.5, label: "50%" },
  { value: 0.65, label: "65%" },
  { value: 0.75, label: "75%" },
  { value: 0.8, label: "80%" },
  { value: 0.9, label: "90%" },
];

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

const formatCredits = (value: number | null | undefined) => {
  if (value === null || value === undefined) return "-";
  return value.toLocaleString("en-US");
};

const formatSignedCredits = (value: number) => {
  const formatted = Math.abs(value).toLocaleString("en-US");
  if (value > 0) return `+${formatted}`;
  if (value < 0) return `-${formatted}`;
  return formatted;
};

const formatHandValue = (value: number, isSoft: boolean) =>
  isSoft ? `Soft ${value}` : String(value);

const doubleRuleAllows = (rule: DoubleRule, value: number) => {
  switch (rule) {
    case "any":
      return true;
    case "nineOrTen":
      return value === 9 || value === 10;
    case "nineThrough11":
      return value >= 9 && value <= 11;
    case "nineThrough15":
      return value >= 9 && value <= 15;
    case "none":
      return false;
  }
};

const parseAmount = (value: string) => {
  const amount = Number(value);
  return Number.isFinite(amount) ? Math.max(0, Math.trunc(amount)) : 0;
};

const stateLabel = (state: string | undefined) => {
  switch (state) {
    case "WaitingForPlayers":
      return "Awaiting player";
    case "Betting":
      return "Place wager";
    case "Dealing":
      return "Dealing";
    case "Insurance":
      return "Insurance";
    case "PlayerTurn":
      return "Your turn";
    case "DealerTurn":
      return "Dealer turn";
    case "RoundOver":
      return "Round complete";
    default:
      return "Preparing";
  }
};

const handStatusLabel = (status: string) => {
  switch (status) {
    case "Stand":
      return "Standing";
    case "Bust":
      return "Bust";
    case "Blackjack":
      return "Blackjack";
    case "Surrendered":
      return "Surrendered";
    default:
      return "Live";
  }
};

const outcomeLabel = (outcome: string) => {
  switch (outcome) {
    case "Win":
      return "Won";
    case "Lose":
      return "Lost";
    case "Push":
      return "Push";
    case "Blackjack":
      return "Blackjack";
    case "Surrendered":
      return "Surrendered";
    default:
      return outcome;
  }
};

const outcomeTone = (outcome: string) => {
  if (outcome === "Win" || outcome === "Blackjack") return "positive";
  if (outcome === "Lose" || outcome === "Surrendered") return "negative";
  return "neutral";
};

const getPhaseCopy = (snapshot: Snapshot | null, result: JsRoundResult | null): PhaseCopy => {
  if (!snapshot) {
    return {
      title: "Loading",
    };
  }

  if (snapshot.player_id === null) {
    return {
      title: "Set bankroll",
    };
  }

  switch (snapshot.state) {
    case "Betting":
      return {
        title: "Set wager",
      };
    case "Insurance":
      return {
        title: "Dealer shows ace",
      };
    case "PlayerTurn":
      return {
        title: `Hand ${(snapshot.current_turn?.hand_index ?? 0) + 1}`,
      };
    case "DealerTurn":
      return {
        title: "Dealer turn",
      };
    case "RoundOver":
      return result
          ? {
            title: "Round settled",
          }
        : {
            title: "Settle round",
          };
    default:
      return {
        title: "Dealing",
      };
  }
};

function PlayingCard({ card }: { card: JsCard | null }) {
  if (!card) {
    return (
      <div className="playing-card card-back" aria-label="Dealer card face down" role="img">
        <span aria-hidden="true" className="card-back-mark">
          bjrs
        </span>
      </div>
    );
  }

  const suit = suitLabel(card.suit);
  const color = card.suit === "Hearts" || card.suit === "Diamonds" ? "red" : "black";
  const rank = rankLabel(card.rank);

  return (
    <div
      aria-label={`${rank} of ${card.suit}`}
      className={`playing-card ${color}`}
      role="img"
    >
      <span aria-hidden="true" className="card-corner card-corner-top">
        <strong>{rank}</strong>
        <small>{suit}</small>
      </span>
      <span aria-hidden="true" className="card-suit">
        {suit}
      </span>
      <span aria-hidden="true" className="card-corner card-corner-bottom">
        <strong>{rank}</strong>
        <small>{suit}</small>
      </span>
    </div>
  );
}

function PlayerHand({ hand, isActive }: { hand: JsHand; isActive: boolean }) {
  return (
    <article className={`player-hand ${isActive ? "is-active" : ""}`}>
      <header className="hand-heading">
        <div>
          <p className="seat-label">Hand {hand.index + 1}</p>
        </div>
        <span className={`status-badge status-${hand.status.toLowerCase()}`}>
          {handStatusLabel(hand.status)}
        </span>
      </header>

      <div className="card-row" aria-label={`Cards in hand ${hand.index + 1}`}>
        {hand.cards.map((card, index) => (
          <PlayingCard card={card} key={`hand-${hand.index}-card-${index}`} />
        ))}
      </div>

      <dl className="hand-facts">
        <div>
          <dt>Value</dt>
          <dd>{formatHandValue(hand.value, hand.is_soft)}</dd>
        </div>
        <div>
          <dt>Wager</dt>
          <dd>{formatCredits(hand.bet)}</dd>
        </div>
      </dl>
    </article>
  );
}

function Settlement({ result }: { result: JsRoundResult }) {
  const player = result.players[0];

  if (!player) return null;

  const netTone = player.net >= 0 ? "positive" : "negative";

  return (
    <div className="settlement">
      <div className="settlement-summary">
        <div>
          <p className="section-kicker">Round result</p>
          <p className="settlement-copy">
            Dealer {result.dealer_value}
            {result.dealer_blackjack ? " / blackjack" : result.dealer_bust ? " / bust" : ""}
          </p>
        </div>
        <div className={`net-result ${netTone}`}>
          <span>Net</span>
          <strong>{formatSignedCredits(player.net)}</strong>
        </div>
      </div>

      <div className="settlement-lines">
        {player.hands.map((hand) => (
          <div className="settlement-line" key={`result-hand-${hand.hand_index}`}>
            <span>Hand {hand.hand_index + 1}</span>
            <span className={`outcome ${outcomeTone(hand.outcome)}`}>
              {outcomeLabel(hand.outcome)}
            </span>
            <span>
              {hand.player_value} vs {hand.dealer_value}
            </span>
            <span>Payout {formatCredits(hand.payout)}</span>
          </div>
        ))}
      </div>

      {(player.insurance_bet > 0 || player.insurance_payout > 0) && (
        <p className="settlement-note">
          Insurance: {formatCredits(player.insurance_bet)} wagered, {formatCredits(player.insurance_payout)} paid.
        </p>
      )}
    </div>
  );
}

export default function App() {
  const [ready, setReady] = useState(false);
  const [game, setGame] = useState<WasmGame | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [result, setResult] = useState<JsRoundResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [buyIn, setBuyIn] = useState(1000);
  const [bet, setBet] = useState(50);
  const [seed, setSeed] = useState(() => makeSeed());
  const [rules, setRules] = useState<TableRules>(defaultTableRules);

  const joined = snapshot?.player_id !== null && snapshot?.player_id !== undefined;
  const isPlayerTurn = snapshot?.state === "PlayerTurn";
  const activeHandIndex = snapshot?.current_turn?.hand_index ?? 0;
  const activeHand = snapshot?.hands.find((hand) => hand.index === activeHandIndex);
  const displayedPlayerValue = activeHand
    ? formatHandValue(activeHand.value, activeHand.is_soft)
    : "-";
  const isActivePlayer = snapshot?.player_id === snapshot?.current_turn?.player_id;
  const isActiveHand = isPlayerTurn && isActivePlayer && activeHand?.status === "Active";
  const availableBalance = snapshot?.money ?? 0;
  const canPlaceBet = Boolean(
    joined && snapshot?.state === "Betting" && bet >= 1 && availableBalance >= bet,
  );
  const canDouble = Boolean(
    isActiveHand &&
      activeHand &&
      activeHand.cards.length === 2 &&
      (!activeHand.from_split || rules.doubleAfterSplit) &&
      doubleRuleAllows(rules.double, activeHand.value) &&
      availableBalance >= activeHand.bet,
  );
  const canSplit = Boolean(
    isActiveHand &&
      activeHand?.can_split &&
      snapshot &&
      snapshot.hands.length <= rules.split &&
      (!rules.splitAcesOnlyOnce ||
        !activeHand.from_split ||
        activeHand.cards[0]?.rank !== 1) &&
      availableBalance >= (activeHand?.bet ?? 0),
  );
  const canSurrender = Boolean(
    isActiveHand &&
      activeHand &&
      rules.surrender &&
      activeHand.cards.length === 2 &&
      !activeHand.from_split,
  );
  const phase = getPhaseCopy(snapshot, result);

  useEffect(() => {
    let cancelled = false;

    init()
      .then(() => {
        if (cancelled) return;
        const instance = new WasmGame(seed, rules);
        setGame(instance);
        setReady(true);
        refresh(instance);
      })
      .catch((initError) => {
        if (!cancelled) {
          setError(`Unable to load the game engine: ${String(initError)}`);
        }
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

  const autoResolveIfNeeded = (instance: WasmGame, current: Snapshot) => {
    let next = current;

    if (next.state === "DealerTurn") {
      instance.dealer_play();
      next = refresh(instance);
    }

    if (next.state === "RoundOver") {
      const round = instance.showdown() as JsRoundResult;
      setResult(round);
      refresh(instance);
    }
  };

  const withGame = (fn: (instance: WasmGame) => void, options?: { autoResolve?: boolean }) => {
    if (!game) return;

    try {
      fn(game);
      setError(null);
      const next = refresh(game);

      if (options?.autoResolve ?? true) {
        autoResolveIfNeeded(game, next);
      }
    } catch (actionError) {
      setError(`The engine rejected that action: ${String(actionError)}`);
    }
  };

  const handleJoin = () => {
    if (joined || buyIn < 1) return;

    withGame((instance) => {
      instance.join(buyIn);
      instance.start_betting();
      setResult(null);
    });
  };

  const replaceTable = (nextSeed: number, nextRules: TableRules) => {
    const instance = new WasmGame(nextSeed, nextRules);
    setSeed(nextSeed);
    setGame(instance);
    setResult(null);
    setError(null);
    refresh(instance);
  };

  const handleReset = () => {
    if (!game) return;

    try {
      replaceTable(makeSeed(), rules);
    } catch (resetError) {
      setError(`Unable to create the table: ${String(resetError)}`);
    }
  };

  const handleRuleChange = (nextRules: TableRules) => {
    if (!game) return;

    try {
      replaceTable(seed, nextRules);
      setRules(nextRules);
    } catch (ruleError) {
      setError(`Unable to apply rules: ${String(ruleError)}`);
    }
  };

  const updateRules = (change: Partial<TableRules>) => {
    handleRuleChange({ ...rules, ...change });
  };

  const handleBetAndDeal = () => {
    if (!canPlaceBet) return;

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

    withGame(
      (instance) => {
        const nextResult = instance.showdown() as JsRoundResult;
        setResult(nextResult);
      },
      { autoResolve: false },
    );
  };

  const handleNextRound = () => {
    withGame((instance) => {
      instance.clear_round();
      instance.start_betting();
      setResult(null);
    });
  };

  if (!ready) {
    return (
      <div className="app-shell loading-shell">
        <main className="loading-view">
          <p className="section-kicker">bjrs</p>
          <h1>Loading</h1>
          {error ? (
            <>
              <p className="engine-error" role="alert">
                {error}
              </p>
              <button type="button" className="button button-primary" onClick={() => window.location.reload()}>
                Reload sample
              </button>
            </>
          ) : (
            <p>Loading the blackjack engine.</p>
          )}
        </main>
      </div>
    );
  }

  const renderDecision = () => {
    if (!joined) {
      return (
        <form
          className="decision-form"
          onSubmit={(event) => {
            event.preventDefault();
            handleJoin();
          }}
        >
          <label className="amount-field" htmlFor="buy-in">
            <span>Starting bankroll</span>
            <span className="amount-input-wrap">
              <input
                id="buy-in"
                min={1}
                onChange={(event) => setBuyIn(parseAmount(event.target.value))}
                step={1}
                type="number"
                value={buyIn}
              />
              <span aria-hidden="true">credits</span>
            </span>
          </label>
          <div className="decision-action">
            <button className="button button-primary" disabled={buyIn < 1} type="submit">
              Open table
            </button>
          </div>
        </form>
      );
    }

    if (snapshot?.state === "Betting") {
      return (
        <form
          className="decision-form"
          onSubmit={(event) => {
            event.preventDefault();
            handleBetAndDeal();
          }}
        >
          <label className="amount-field" htmlFor="bet">
            <span>Opening wager</span>
            <span className="amount-input-wrap">
              <input
                id="bet"
                min={1}
                onChange={(event) => setBet(parseAmount(event.target.value))}
                step={1}
                type="number"
                value={bet}
              />
              <span aria-hidden="true">credits</span>
            </span>
          </label>
          <div className="decision-action">
            <button className="button button-primary" disabled={!canPlaceBet} type="submit">
              Deal cards
            </button>
            <p>
              {bet > availableBalance
                ? "Insufficient balance"
                : `Available: ${formatCredits(availableBalance)}`}
            </p>
          </div>
        </form>
      );
    }

    if (snapshot?.state === "Insurance") {
      return (
        <div className="decision-actions">
          <button className="button button-primary" onClick={() => handleInsurance(true)} type="button">
            Take insurance
          </button>
          <button className="button button-secondary" onClick={() => handleInsurance(false)} type="button">
            Continue without it
          </button>
        </div>
      );
    }

    if (snapshot?.state === "PlayerTurn" && activeHand) {
      return (
        <div className="decision-actions">
          <div className="primary-actions">
            <button
              className="button button-primary"
              disabled={!isActiveHand}
              onClick={() => withGame((instance) => instance.hit(activeHandIndex))}
              type="button"
            >
              Hit
            </button>
            <button
              className="button button-secondary"
              disabled={!isActiveHand}
              onClick={() => withGame((instance) => instance.stand(activeHandIndex))}
              type="button"
            >
              Stand
            </button>
          </div>
          <div className="secondary-actions" aria-label="Optional actions">
            <button
              className="button button-quiet"
              disabled={!canDouble}
              onClick={() => withGame((instance) => instance.double_down(activeHandIndex))}
              type="button"
            >
              Double
            </button>
            <button
              className="button button-quiet"
              disabled={!canSplit}
              onClick={() => withGame((instance) => instance.split(activeHandIndex))}
              type="button"
            >
              Split
            </button>
            <button
              className="button button-quiet"
              disabled={!canSurrender}
              onClick={() => withGame((instance) => instance.surrender(activeHandIndex))}
              type="button"
            >
              Surrender
            </button>
          </div>
        </div>
      );
    }

    if (snapshot?.state === "DealerTurn") {
      return (
        <div className="decision-actions">
          <button className="button button-primary" onClick={handleDealerPlay} type="button">
            Resolve dealer hand
          </button>
        </div>
      );
    }

    if (snapshot?.state === "RoundOver") {
      return (
        <div className="round-over-actions">
          {result ? <Settlement result={result} /> : null}
          <div className="decision-actions">
            {!result && (
              <button className="button button-primary" onClick={handleShowdown} type="button">
                Calculate payout
              </button>
            )}
            <button className="button button-secondary" onClick={handleNextRound} type="button">
              Start next round
            </button>
          </div>
        </div>
      );
    }

    return null;
  };

  return (
    <div className="app-shell">
      <header className="masthead">
        <div className="brand-lockup">
          <p className="section-kicker">bjrs</p>
          <h1>Blackjack</h1>
        </div>
        <div className="table-identity">
          <dl>
            <div>
              <dt>Table seed</dt>
              <dd>{seed}</dd>
            </div>
          </dl>
          <button className="button button-quiet reset-button" onClick={handleReset} type="button">
            New table
          </button>
        </div>
      </header>

      <main className="game-layout">
        <section className="board-column" aria-labelledby="table-heading">
          <section className="round-brief" aria-live="polite">
            <h2 id="table-heading">{phase.title}</h2>
          </section>

          <section className="tabletop" aria-label="Blackjack game table">
            <section className="dealer-area" aria-labelledby="dealer-heading">
              <header className="seat-heading">
                <div>
                  <p className="seat-label" id="dealer-heading">
                    Dealer
                  </p>
                  <p className="seat-detail">
                    {snapshot?.dealer.hole_revealed
                      ? snapshot.dealer.is_blackjack
                        ? "Blackjack"
                        : snapshot.dealer.is_bust
                          ? "Bust"
                          : snapshot.dealer.is_soft
                            ? "Soft hand"
                            : "Final hand"
                      : "Visible hand"}
                  </p>
                </div>
                <div className="seat-total">
                  <span>Value</span>
                  <strong>
                    {snapshot?.dealer.hole_revealed
                      ? formatCredits(snapshot.dealer.value)
                      : formatCredits(snapshot?.dealer.visible_value)}
                  </strong>
                </div>
              </header>
              {snapshot?.dealer.cards.length ? (
                <div className="card-row" aria-label="Dealer cards">
                  {snapshot.dealer.cards.map((card, index) => (
                    <PlayingCard card={card} key={`dealer-card-${index}`} />
                  ))}
                </div>
              ) : (
                <p className="empty-seat">No cards</p>
              )}
            </section>

            <div className="table-divider" aria-hidden="true">
              <span>vs</span>
            </div>

            <section className="player-area" aria-labelledby="player-heading">
              <header className="seat-heading">
                <div>
                  <p className="seat-label" id="player-heading">
                    Player
                  </p>
                  <p className="seat-detail">
                    {joined ? `${snapshot?.hands.length ?? 0} hand${snapshot?.hands.length === 1 ? "" : "s"} on table` : "Seat open"}
                  </p>
                </div>
                <div className="seat-total">
                  <span>Value</span>
                  <strong>{displayedPlayerValue}</strong>
                </div>
              </header>
              {snapshot?.hands.length ? (
                <div className="hands-stack">
                  {snapshot.hands.map((hand) => (
                    <PlayerHand
                      hand={hand}
                      isActive={Boolean(
                        snapshot.state === "PlayerTurn" &&
                          snapshot.current_turn?.hand_index === hand.index &&
                          snapshot.player_id === snapshot.current_turn?.player_id,
                      )}
                      key={`hand-${hand.index}`}
                    />
                  ))}
                </div>
              ) : (
                <p className="empty-seat">
                  {joined ? "No cards" : "Seat open"}
                </p>
              )}
            </section>
          </section>

          <section className="decision-dock" aria-labelledby="decision-heading">
            <div className="decision-heading">
              <p className="section-kicker" id="decision-heading">
                Actions
              </p>
              {snapshot?.insurance_bet ? <span className="context-note">Insurance {formatCredits(snapshot.insurance_bet)}</span> : null}
            </div>
            {renderDecision()}
            {error ? (
              <p className="engine-error" role="alert">
                {error}
              </p>
            ) : null}
          </section>
        </section>

        <aside className="table-rail" aria-label="Table details">
          <section className="rail-section ledger-section" aria-labelledby="ledger-heading">
            <div className="rail-heading">
              <h2 id="ledger-heading">Table ledger</h2>
            </div>
            <dl className="ledger-list">
              <div>
                <dt>Engine state</dt>
                <dd>{stateLabel(snapshot?.state)}</dd>
              </div>
              <div>
                <dt>Balance</dt>
                <dd>{formatCredits(snapshot?.money)}</dd>
              </div>
              <div>
                <dt>Current wager</dt>
                <dd>{formatCredits(snapshot?.bet)}</dd>
              </div>
              <div>
                <dt>Cards remaining</dt>
                <dd>{formatCredits(snapshot?.cards_remaining)}</dd>
              </div>
            </dl>
          </section>

          <section className="rail-section rules-section" aria-labelledby="rules-heading">
            <div className="rail-heading">
              <h2 id="rules-heading">Table rules</h2>
            </div>
            <div className="rule-groups">
              <fieldset className="rule-group">
                <legend>Table</legend>
                <div className="rule-controls">
                  <label className="rule-control" htmlFor="decks">
                    <span>Decks</span>
                    <select
                      disabled={joined}
                      id="decks"
                      onChange={(event) => updateRules({ decks: Number(event.target.value) })}
                      value={rules.decks}
                    >
                      {deckOptions.map((decks) => (
                        <option key={decks} value={decks}>
                          {decks}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="rule-control" htmlFor="penetration">
                    <span>Penetration</span>
                    <select
                      disabled={joined}
                      id="penetration"
                      onChange={(event) => updateRules({ penetration: Number(event.target.value) })}
                      value={rules.penetration}
                    >
                      {penetrationOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="rule-control" htmlFor="soft-17">
                    <span>Dealer stands on soft 17</span>
                    <input
                      checked={rules.standOnSoft17}
                      disabled={joined}
                      id="soft-17"
                      onChange={(event) => updateRules({ standOnSoft17: event.target.checked })}
                      type="checkbox"
                    />
                  </label>
                </div>
              </fieldset>

              <fieldset className="rule-group">
                <legend>Payouts</legend>
                <div className="rule-controls">
                  <label className="rule-control" htmlFor="blackjack-pays">
                    <span>Blackjack pays</span>
                    <select
                      disabled={joined}
                      id="blackjack-pays"
                      onChange={(event) => updateRules({ blackjackPays: Number(event.target.value) })}
                      value={rules.blackjackPays}
                    >
                      {blackjackPayoutOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="rule-control" htmlFor="blackjack-rounding">
                    <span>Blackjack rounding</span>
                    <select
                      disabled={joined}
                      id="blackjack-rounding"
                      onChange={(event) =>
                        updateRules({ roundingBlackjack: event.target.value as RoundingRule })
                      }
                      value={rules.roundingBlackjack}
                    >
                      {roundingOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="rule-control" htmlFor="surrender-rounding">
                    <span>Surrender rounding</span>
                    <select
                      disabled={joined}
                      id="surrender-rounding"
                      onChange={(event) =>
                        updateRules({ roundingSurrender: event.target.value as RoundingRule })
                      }
                      value={rules.roundingSurrender}
                    >
                      {roundingOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
              </fieldset>

              <fieldset className="rule-group">
                <legend>Actions</legend>
                <div className="rule-controls">
                  <label className="rule-control" htmlFor="double-rule">
                    <span>Double down</span>
                    <select
                      disabled={joined}
                      id="double-rule"
                      onChange={(event) =>
                        updateRules({ double: event.target.value as DoubleRule })
                      }
                      value={rules.double}
                    >
                      {doubleRuleOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="rule-control" htmlFor="double-after-split">
                    <span>Double after split</span>
                    <input
                      checked={rules.doubleAfterSplit}
                      disabled={joined}
                      id="double-after-split"
                      onChange={(event) => updateRules({ doubleAfterSplit: event.target.checked })}
                      type="checkbox"
                    />
                  </label>
                  <label className="rule-control" htmlFor="max-splits">
                    <span>Maximum splits</span>
                    <select
                      disabled={joined}
                      id="max-splits"
                      onChange={(event) => updateRules({ split: Number(event.target.value) })}
                      value={rules.split}
                    >
                      {splitOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="rule-control" htmlFor="split-aces-once">
                    <span>Split aces once only</span>
                    <input
                      checked={rules.splitAcesOnlyOnce}
                      disabled={joined}
                      id="split-aces-once"
                      onChange={(event) =>
                        updateRules({ splitAcesOnlyOnce: event.target.checked })
                      }
                      type="checkbox"
                    />
                  </label>
                  <label className="rule-control" htmlFor="split-aces-one-card">
                    <span>Split aces receive one card</span>
                    <input
                      checked={rules.splitAcesReceiveOneCard}
                      disabled={joined}
                      id="split-aces-one-card"
                      onChange={(event) =>
                        updateRules({ splitAcesReceiveOneCard: event.target.checked })
                      }
                      type="checkbox"
                    />
                  </label>
                  <label className="rule-control" htmlFor="insurance">
                    <span>Insurance</span>
                    <input
                      checked={rules.insurance}
                      disabled={joined}
                      id="insurance"
                      onChange={(event) => updateRules({ insurance: event.target.checked })}
                      type="checkbox"
                    />
                  </label>
                  <label className="rule-control" htmlFor="surrender">
                    <span>Surrender</span>
                    <input
                      checked={rules.surrender}
                      disabled={joined}
                      id="surrender"
                      onChange={(event) => updateRules({ surrender: event.target.checked })}
                      type="checkbox"
                    />
                  </label>
                </div>
              </fieldset>
            </div>
          </section>

        </aside>
      </main>
    </div>
  );
}
