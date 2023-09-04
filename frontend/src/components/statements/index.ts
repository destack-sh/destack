import BlankElement from "@/components/statements/BlankElement.vue";
import CodeElement from "@/components/statements/CodeElement.vue";
import FunctionTypeElement from "@/components/statements/FunctionTypeElement.vue";
import TextElement from "@/components/statements/TextElement.vue";
import { StatementType } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import type { Statement } from "@/state/module";
import type { UseElementBoundingReturn } from "@vueuse/core";
import type { Component } from "vue";

export type StatementElement = Component & {
  actions: StatementAction[];
  focus: (focus: "first" | "last") => void;
  blur: () => void;
};

export type StatementProps = {
  statement: Statement;
  focused: boolean;
  editing: boolean;
  readonly: boolean;
  bounding: UseElementBoundingReturn;
};

export type StatementEmit = {
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "enter"): void;
  (e: "escape"): void;
};

export type StatementInterface = {
  type: StatementType;
  primaryControl: "declaration" | "text" | "blank";
  hasTags?: boolean;
  hasTriggers?: boolean;
  isRunnable?: boolean;
  extraControls?: Component[];
  elements: Component[];
};

export const STATEMENT_INTERFACES: Partial<Record<StatementType, StatementInterface>> = {};

function register(type: StatementType, value: Omit<StatementInterface, "type">) {
  if (STATEMENT_INTERFACES[type] != null) {
    throw new Error(`interface ${type} already registered`);
  }
  STATEMENT_INTERFACES[type] = { ...value, type };
}

register(StatementType.Blank, { primaryControl: "blank", elements: [BlankElement] });
register(StatementType.Text, { primaryControl: "text", hasTags: true, elements: [TextElement] });
register(StatementType.Code, {
  primaryControl: "declaration",
  isRunnable: true,
  hasTags: true,
  hasTriggers: true,
  elements: [TextElement, FunctionTypeElement, CodeElement],
});
register(StatementType.Task, {
  primaryControl: "declaration",
  isRunnable: true,
  hasTags: true,
  hasTriggers: true,
  elements: [TextElement, FunctionTypeElement],
});
// nocheckin: other interfaces (dataset, type, variable, tag)
