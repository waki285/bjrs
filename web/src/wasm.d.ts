import "../wasm/pkg/bjrs_wasm";

declare module "../wasm/pkg/bjrs_wasm" {
  export type WasmDoubleOption =
    | "any"
    | "nineOrTen"
    | "tenOrEleven"
    | "nineThrough11"
    | "nineThrough15"
    | "none";

  export type WasmRoundingMode = "up" | "down" | "nearest";

  export type WasmSurrenderOption = "none" | "early" | "earlyWithoutAce" | "late";

  export type WasmGameOptions = {
    decks?: number;
    blackjackPays?: number;
    standOnSoft17?: boolean;
    double?: WasmDoubleOption;
    split?: number;
    doubleAfterSplit?: boolean;
    splitAcesOnlyOnce?: boolean;
    splitAcesReceiveOneCard?: boolean;
    insurance?: boolean;
    surrender?: WasmSurrenderOption;
    roundingBlackjack?: WasmRoundingMode;
    roundingSurrender?: WasmRoundingMode;
    penetration?: number;
  };

}
