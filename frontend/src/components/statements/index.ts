import BlankElement from "@/components/statements/BlankElement.vue";
import CodeElement from "@/components/statements/CodeElement.vue";
import DeclarationControl from "@/components/statements/DeclarationControl.vue";
import FunctionTypeElement from "@/components/statements/FunctionTypeElement.vue";
import ListTypeElement from "@/components/statements/ListTypeElement.vue";
import TaggingControl from "@/components/statements/TaggingControl.vue";
import TextElement from "@/components/statements/TextElement.vue";
import TriggerControl from "@/components/statements/TriggerControl.vue";
import { StatementType } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { TypeFlag, type Statement } from "@/state/module";
import type { UseElementBoundingReturn } from "@vueuse/core";

export type StatementPartComponent = InstanceType<any> & {
  actions?: StatementAction[];
  focus: (focus: "first" | "last") => void;
  blur: () => void;
  syncNow?: void;
  capturingDrag?: boolean;
  loading?: boolean;
};

export type StatementPart = {
  id: string;
  component: StatementPartComponent;
  actions?: StatementAction[];
  exists: (iface: StatementInterface, statement: Statement) => boolean;
};

export type StatementControl = StatementPart & {
  enabled: (iface: StatementInterface, statement: Statement) => boolean;
};

export type StatementElement = StatementPart & {
  showIfEmpty?: boolean;
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
  (e: "run", args?: Record<string, any>): void;
  (e: "openActions"): void;
};

export type StatementInterface = {
  type: StatementType;
  needsName?: boolean;
  primaryPart: string;
  hasBases?: boolean;
  hasTags?: boolean;
  hasTriggers?: boolean;
  isRunnable?: boolean;
  extraControls?: StatementControl[];
  elements: StatementElement[];
};

export const BASIC_CONTROL_PARTS: StatementControl[] = [
  {
    id: "declaration",
    component: DeclarationControl,
    enabled: (iface, statement) => iface.needsName || statement.name != null,
    exists: (iface, statement) => statement.name != null,
  },
  {
    id: "bases",
    component: null,
    enabled: (iface, statement) => iface.hasBases ?? false,
    exists: (iface, statement) =>
      statement.fields?.find((b) => b.deletedAt != null && b.flags & TypeFlag.IsUnionWith) != null,
  },
  {
    id: "tagging",
    component: TaggingControl,
    enabled: (iface, statement) => iface.hasTags ?? false,
    exists: (iface, statement) => statement.tags?.find((t) => t.deletedAt != null) != null,
  },
  {
    id: "trigger",
    component: TriggerControl,
    enabled: (iface, statement) => iface.hasTriggers ?? false,
    exists: (iface, statement) => statement.triggers?.find((t) => t.deletedAt != null) != null,
  },
];

const BLANK: StatementElement = { id: "blank", component: BlankElement, exists: () => true }; // nocheckin: type statement parts
const TEXT: StatementElement = {
  id: "text",
  component: TextElement,
  exists: (iface, statement) => (statement.text ?? "").length > 0,
};
const FUNCTION_TYPE: StatementElement = {
  id: "type.function",
  component: FunctionTypeElement,
  exists: (iface, statement) => statement.fields?.find((f) => f.deletedAt != null) != null,
};
const LIST_TYPE: StatementElement = {
  id: "type.list",
  component: ListTypeElement,
  exists: (iface, statement) => statement.fields?.find((f) => f.deletedAt != null) != null,
};
const CODE: StatementElement = {
  id: "code",
  component: CodeElement,
  exists: (iface, statement) => (statement.code ?? "").length > 0,
};

export const STATEMENT_INTERFACES: Partial<Record<StatementType, StatementInterface>> = {};

function register(type: StatementType, value: Omit<StatementInterface, "type">) {
  if (STATEMENT_INTERFACES[type] != null) {
    throw new Error(`interface ${type} already registered`);
  }
  STATEMENT_INTERFACES[type] = { ...value, type };
}

register(StatementType.Blank, { primaryPart: "blank", elements: [BLANK] });
register(StatementType.Text, { primaryPart: "text", hasTags: true, elements: [TEXT] });
register(StatementType.Code, {
  primaryPart: "code",
  needsName: true,
  isRunnable: true,
  hasTags: true,
  hasTriggers: true,
  elements: [TEXT, FUNCTION_TYPE, { ...CODE, showIfEmpty: true }],
});
register(StatementType.Type, {
  primaryPart: "type",
  needsName: true,
  hasTags: true,
  elements: [TEXT, { ...LIST_TYPE, showIfEmpty: true }],
});
// register(StatementType.Task, {
//   primaryControl: "declaration",
//   isRunnable: true,
//   hasTags: true,
//   hasTriggers: true,
//   elements: [TextElement, FunctionTypeElement],
// });
// nocheckin: cover all interfaces (dataset, type, variable, tag)
