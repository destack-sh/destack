import BaseTypeControl from "@/components/statements/BaseTypeControl.vue";
import BlankElement from "@/components/statements/BlankElement.vue";
import CodeElement from "@/components/statements/CodeElement.vue";
import CurrentRunControl from "@/components/statements/CurrentRunControl.vue";
import DatabaseElement from "@/components/statements/DatabaseElement.vue";
import DatabaseInfoControl from "@/components/statements/DatabaseInfoControl.vue";
import DeclarationControl from "@/components/statements/DeclarationControl.vue";
import FunctionTypeElement from "@/components/statements/FunctionTypeElement.vue";
import ListTypeElement from "@/components/statements/ListTypeElement.vue";
import RunElement from "@/components/statements/RunElement.vue";
import TaggingControl from "@/components/statements/TaggingControl.vue";
import TextElement from "@/components/statements/TextElement.vue";
import TriggerControl from "@/components/statements/TriggerControl.vue";
import ValueElement from "@/components/statements/ValueElement.vue";
import { StatementType } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { TypeFlag, type Statement } from "@/state/module";
import type { UseElementBoundingReturn } from "@vueuse/core";

export const STATEMENT_RUNNABLE_TYPES: StatementType[] = [StatementType.Code, StatementType.Task, StatementType.Flow];

export type StatementPartComponent = InstanceType<any> & {
  actions?: StatementAction[];
  focus: (focus: "first" | "last") => void;
  blur: () => void;
  syncNow?: void;
  capturingDrag?: boolean;
  loading?: boolean;
};

export type StatementControlId =
  | "declaration"
  | "bases"
  | "tagging"
  | "trigger"
  | "reference"
  | "run.meta"
  | "database.info"
  | "database.search";
export type StatementElementId =
  | "blank"
  | "text"
  | "type.function"
  | "type.list"
  | "code"
  | "value"
  | "database"
  | "run";
export type StatementPartId = StatementControlId | StatementElementId;

export type StatementPart = {
  id: StatementPartId;
  component: StatementPartComponent;
  notFocusable?: boolean;
  exists: (iface: StatementInterface, statement: Statement) => boolean;
};

export type StatementControl = StatementPart & {
  id: StatementControlId;
  enabled: (iface: StatementInterface, statement: Statement) => boolean;
};

export type StatementElement = StatementPart & {
  id: StatementElementId;
  showIfNotExists?: boolean;
};

export type StatementProps = {
  statement: Statement;
  focused: boolean;
  selected: boolean;
  editing: boolean;
  readonly: boolean;
  visible: boolean;
  bounding: UseElementBoundingReturn;
  xoffset: number;
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
  (e: "enterRight"): void;
  (e: "escape"): void;
  (e: "paste"): void;
  (e: "run", args?: Record<string, any>): void;
  (e: "focus", partId: StatementPartId): void;
  (e: "hide", partId: StatementPartId): void;
  (e: "openActions"): void;
  (e: "launchAssist", text: string): void;
  (e: "illegal", char: string): void;
};

export type StatementEmitDict = {
  // there must be some way to do this with generics, but I can't figure it out
  navigateUp: () => void;
  navigateDown: () => void;
  navigateLeft: () => void;
  navigateRight: () => void;
  deleteLeft: () => void;
  deleteSelf: () => void;
  enterLeft: () => void;
  enter: () => void;
  enterRight: () => void;
  escape: () => void;
  paste: () => void;
  run: (args?: Record<string, any>) => void;
  focus: (partId: StatementPartId) => void;
  hide: (partId: StatementPartId) => void;
  openActions: () => void;
  launchAssist: (text: string) => void;
  illegal: (char: string) => void;
};

export type StatementInterface = {
  type: StatementType;
  foldable?: "function-self" | "function-all" | "list-self" | "list-all";
  needsDeclaration?: boolean;
  showControls?: boolean;
  primaryPart: string;
  hasBases?: boolean;
  hasTags?: boolean;
  hasTriggers?: boolean;
  extraControls?: StatementControl[];
  elements: StatementElement[];
};

export const BASIC_CONTROL_PARTS: StatementControl[] = [
  {
    id: "declaration",
    component: DeclarationControl,
    enabled: (iface, statement) =>
      iface.needsDeclaration || (statement.type == StatementType.Text && statement.name != null),
    exists: (iface, statement) => statement.name != null,
  },
  {
    id: "bases",
    component: BaseTypeControl,
    enabled: (iface, statement) => iface.hasBases ?? false,
    exists: (iface, statement) =>
      statement.fields?.find((b) => b.deletedAt == null && b.flags & TypeFlag.IS_UNION_WITH) != null,
  },
  {
    id: "tagging",
    component: TaggingControl,
    enabled: (iface, statement) => iface.hasTags ?? false,
    exists: (iface, statement) => statement.tags?.find((t) => t.deletedAt == null) != null,
  },
  {
    id: "trigger",
    component: TriggerControl,
    enabled: (iface, statement) => iface.hasTriggers ?? false,
    exists: (iface, statement) => statement.triggers?.find((t) => t.deletedAt == null) != null,
  },
  {
    id: "database.info",
    component: DatabaseInfoControl,
    enabled: (iface, statement) => statement.type === StatementType.Database,
    exists: (iface, statement) => true,
  },
];
const RUN_META: StatementControl = {
  id: "run.meta",
  component: CurrentRunControl,
  notFocusable: true,
  // must click to enable
  enabled: (iface, statement) => false,
  exists: (iface, statement) => false,
};

const BLANK: StatementElement = { id: "blank", component: BlankElement, exists: () => true };
const TEXT: StatementElement = {
  id: "text",
  component: TextElement,
  exists: (iface, statement) =>
    (statement.text ?? "").length > 0 || (statement.type == StatementType.Text && statement.name == null),
};
const FUNCTION_TYPE: StatementElement = {
  id: "type.function",
  component: FunctionTypeElement,
  exists: (iface, statement) => statement.fields?.find((f) => f.deletedAt == null) != null,
};
const LIST_TYPE: StatementElement = {
  id: "type.list",
  component: ListTypeElement,
  exists: (iface, statement) => statement.fields?.find((f) => f.deletedAt == null) != null,
};
const CODE: StatementElement = {
  id: "code",
  component: CodeElement,
  exists: (iface, statement) => (statement.code ?? "").length > 0,
};
const VALUE: StatementElement = {
  id: "value",
  component: ValueElement,
  exists: (iface, statement) => statement.value != null,
};
const DATABASE: StatementElement = {
  id: "database",
  component: DatabaseElement,
  exists: (iface, statement) => true, // unknown, but doesn't matter
};
const RUN: StatementElement = {
  id: "run",
  component: RunElement,
  notFocusable: true,
  exists: (iface, statement) => false, // only shown manually,
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
  foldable: "function-self",
  needsDeclaration: true,
  hasBases: true,
  hasTags: true,
  hasTriggers: true,
  extraControls: [RUN_META],
  elements: [TEXT, FUNCTION_TYPE, { ...CODE, showIfNotExists: true }, RUN],
});
register(StatementType.Class, {
  primaryPart: "type",
  foldable: "list-self",
  needsDeclaration: true,
  hasBases: true,
  hasTags: true,
  elements: [TEXT, { ...LIST_TYPE, showIfNotExists: true }],
});
register(StatementType.Choice, {
  primaryPart: "type",
  foldable: "list-self",
  needsDeclaration: true,
  hasBases: false,
  hasTags: true,
  elements: [TEXT, { ...LIST_TYPE, showIfNotExists: true }],
});
register(StatementType.Task, {
  primaryPart: "declaration",
  foldable: "function-all",
  needsDeclaration: true,
  hasTags: true,
  hasBases: true,
  hasTriggers: true,
  extraControls: [RUN_META],
  elements: [TEXT, { ...FUNCTION_TYPE, showIfNotExists: true }, RUN],
});
register(StatementType.Variable, {
  primaryPart: "value",
  foldable: "list-all",
  needsDeclaration: true,
  hasTags: true,
  elements: [TEXT, { ...VALUE, showIfNotExists: true }],
});
register(StatementType.Database, {
  primaryPart: "database",
  foldable: "list-all",
  needsDeclaration: true,
  hasTags: true,
  hasBases: true,
  extraControls: [],
  elements: [TEXT, DATABASE],
});
register(StatementType.Tag, {
  primaryPart: "declaration",
  foldable: "list-all",
  needsDeclaration: true,
  hasTags: true,
  hasBases: true,
  elements: [TEXT],
});
register(StatementType.Group, {
  primaryPart: "declaration",
  needsDeclaration: true,
  hasTags: true,
  elements: [TEXT],
});
