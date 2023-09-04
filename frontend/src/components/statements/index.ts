import BlankElement from "@/components/statements/BlankElement.vue";
import CodeElement from "@/components/statements/CodeElement.vue";
import FunctionTypeElement from "@/components/statements/FunctionTypeElement.vue";
import TextElement from "@/components/statements/TextElement.vue";
import { StatementType } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import type { Statement } from "@/state/module";
import type { UseElementBoundingReturn } from "@vueuse/core";
import type { Component, ComputedRef } from "vue";

export type StatementPartComponent = Component & {
  actions: StatementAction[];
  focus: (focus: "first" | "last") => void;
  blur: () => void;
  innerDrag?: boolean;
  loading?: boolean;
};

export type StatementPart = {
  id: string;
  component: StatementPartComponent;
};

export type StatementControl = StatementPart & {
  enabled: ComputedRef<boolean>;
  exists: ComputedRef<boolean>;
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
  (e: "deleteSelf"): void;
  (e: "enterLeft"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "paste"): void;
  (e: "run"): void;
  (e: "openActions"): void;
};

export type StatementInterface = {
  type: StatementType;
  primaryPart: "declaration" | "text" | "blank";
  hasBases?: boolean;
  hasTags?: boolean;
  hasTriggers?: boolean;
  isRunnable?: boolean;
  extraControls?: StatementPart[];
  elements: StatementPart[];
};

const BLANK: StatementPart = { id: "blank", component: BlankElement }; // nocheckin: type statement parts
const TEXT: StatementPart = { id: "text", component: TextElement };

export const STATEMENT_INTERFACES: Partial<Record<StatementType, StatementInterface>> = {};

function register(type: StatementType, value: Omit<StatementInterface, "type">) {
  if (STATEMENT_INTERFACES[type] != null) {
    throw new Error(`interface ${type} already registered`);
  }
  STATEMENT_INTERFACES[type] = { ...value, type };
}

register(StatementType.Blank, { primaryPart: "blank", elements: [BLANK] });
register(StatementType.Text, { primaryPart: "text", hasTags: true, elements: [TEXT] });
// register(StatementType.Code, {
//   primaryControl: "declaration",
//   isRunnable: true,
//   hasTags: true,
//   hasTriggers: true,
//   elements: [TextElement, FunctionTypeElement, CodeElement],
// });
// register(StatementType.Task, {
//   primaryControl: "declaration",
//   isRunnable: true,
//   hasTags: true,
//   hasTriggers: true,
//   elements: [TextElement, FunctionTypeElement],
// });
// nocheckin: cover all interfaces (dataset, type, variable, tag)
