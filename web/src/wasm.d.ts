declare module "../wasm/pkg/bjrs_wasm" {
  export default function init(): Promise<void>;

  export class WasmGame {
    constructor(seed: number);
    reset(seed: number): void;
    join(money: number): number;
    player_id(): number | undefined;
    start_betting(): void;
    bet(amount: number): void;
    deal(): void;
    hit(hand_index: number): void;
    stand(hand_index: number): void;
    double_down(hand_index: number): void;
    split(hand_index: number): void;
    surrender(hand_index: number): number;
    take_insurance(): number;
    decline_insurance(): void;
    finish_insurance(): boolean;
    dealer_play(): void;
    showdown(): unknown;
    clear_round(): void;
    snapshot(): unknown;
  }
}
