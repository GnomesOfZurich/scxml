/**
 * Typed interfaces for the scxml WASM package.
 *
 * The WASM functions pass JSON strings across the boundary.
 * These types describe the shape of those JSON payloads,
 * so TypeScript consumers get full type safety:
 *
 * ```typescript
 * import init, { parseXml, flatten } from '@gnomes/scxml';
 * import type { Statechart, FlatState, FlatTransition } from '@gnomes/scxml/types';
 *
 * await init();
 * const chart: Statechart = JSON.parse(parseXml(xml));
 * const { states, transitions }: FlatResult = JSON.parse(flatten(json));
 * ```
 */

// ── Model types ─────────────────────────────────────────────────────────────

export interface Statechart {
  name?: string;
  initial: string;
  states: State[];
  datamodel: DataModel;
  binding: 'early' | 'late';
  version: string;
  xmlns: string;
}

export interface State {
  id: string;
  kind: StateKind;
  transitions: Transition[];
  on_entry: Action[];
  on_exit: Action[];
  children: State[];
  initial?: string;
}

export type StateKind =
  | 'atomic'
  | 'compound'
  | 'parallel'
  | 'final'
  | { history: 'shallow' | 'deep' };

export interface Transition {
  event?: string;
  guard?: string;
  targets: string[];
  transition_type: 'external' | 'internal';
  actions: Action[];
  delay?: string;
  quorum?: number;
}

export interface Action {
  kind: ActionKind;
}

export type ActionKind =
  | { type: 'raise'; event: string }
  | { type: 'send'; event: string; target?: string; delay?: string }
  | { type: 'assign'; location: string; expr: string }
  | { type: 'log'; label?: string; expr?: string }
  | { type: 'custom'; name: string; params: [string, string][] };

export interface DataModel {
  items: DataItem[];
}

export interface DataItem {
  id: string;
  expr?: string;
  src?: string;
}

// ── Flatten output ──────────────────────────────────────────────────────────

export interface FlatResult {
  states: FlatState[];
  transitions: FlatTransition[];
}

export interface FlatState {
  id: string;
  kind: string;
  parent?: string;
  initial: boolean;
  depth: number;
}

export interface FlatTransition {
  source: string;
  target: string;
  event?: string;
  guard?: string;
}
