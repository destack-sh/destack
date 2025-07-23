import { Graph } from "destack";

/** A reactive Graph. */
export interface ReactiveGraph extends Graph {
  /** Touch a Node reactively. */
  touch(id: string): void;

  /** 'Subscribe' to a Node reactively. */
  subscribe(id: string): void;

  /** Touch the children of a Node reactively. */
  touchChildren(id: string): void;

  /** 'Subscribe' to the children of a Node reactively. */
  subscribeChildren(id: string): void;
}
