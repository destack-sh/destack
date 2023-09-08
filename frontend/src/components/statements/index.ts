import BaseTypeControl from "@/components/statements/BaseTypeControl.vue";
import BlankElement from "@/components/statements/BlankElement.vue";
import CodeElement from "@/components/statements/CodeElement.vue";
import CurrentRunControl from "@/components/statements/CurrentRunControl.vue";
import DatasetElement from "@/components/statements/DatasetElement.vue";
import DatasetSearchControl from "@/components/statements/DatasetSearchControl.vue";
import DeclarationControl from "@/components/statements/DeclarationControl.vue";
import FunctionTypeElement from "@/components/statements/FunctionTypeElement.vue";
import ListTypeElement from "@/components/statements/ListTypeElement.vue";
import ReferenceControl from "@/components/statements/ReferenceControl.vue";
import RunElement from "@/components/statements/RunElement.vue";
import TaggingControl from "@/components/statements/TaggingControl.vue";
import TextElement from "@/components/statements/TextElement.vue";
import TriggerControl from "@/components/statements/TriggerControl.vue";
import VariableElement from "@/components/statements/VariableElement.vue";
import { StatementType } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { TypeFlag, type Statement } from "@/state/module";
import type { UseElementBoundingReturn } from "@vueuse/core";

export const STATEMENT_STANDALONE_TYPES: StatementType[] = [StatementType.Dataset, StatementType.Code];
export const STANDALONE_ENABLED = false; // needs proper support

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
  | "dataset.search";
export type StatementElementId =
  | "blank"
  | "text"
  | "type.function"
  | "type.list"
  | "code"
  | "variable"
  | "dataset"
  | "run";
export type StatementPartId = StatementControlId | StatementElementId;

export type StatementPart = {
  id: StatementPartId;
  component: StatementPartComponent;
  exists: (iface: StatementInterface, statement: Statement) => boolean;
};

export type StatementControl = StatementPart & {
  id: StatementControlId;
  enabled: (iface: StatementInterface, statement: Statement) => boolean;
};

export type StatementElement = StatementPart & {
  id: StatementElementId;
  showIfEmpty?: boolean;
};

export type StatementProps = {
  statement: Statement;
  focused: boolean;
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
  (e: "openActions"): void;
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
  openActions: () => void;
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
      iface.needsDeclaration ||
      (statement.type == StatementType.Text && (statement.headingLevel != null || statement.name != null)),
    exists: (iface, statement) => statement.name != null,
  },
  {
    id: "bases",
    component: BaseTypeControl,
    enabled: (iface, statement) => iface.hasBases ?? false,
    exists: (iface, statement) =>
      statement.fields?.find((b) => b.deletedAt == null && b.flags & TypeFlag.IsUnionWith) != null,
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
    id: "reference",
    component: ReferenceControl,
    enabled: (iface, statement) => statement.type === StatementType.Reference,
    exists: (iface, statement) => true,
  },
];
const RUN_META: StatementControl = {
  id: "run.meta",
  component: CurrentRunControl,
  enabled: (iface, statement) => false,
  exists: (iface, statement) => false,
};
const DATASET_SEARCH: StatementControl = {
  id: "dataset.search",
  component: DatasetSearchControl,
  enabled: (iface, statement) => statement.type === StatementType.Dataset,
  exists: (iface, statement) => true,
};

const BLANK: StatementElement = { id: "blank", component: BlankElement, exists: () => true };
const TEXT: StatementElement = {
  id: "text",
  component: TextElement,
  exists: (iface, statement) => (statement.text ?? "").length > 0,
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
const VARIABLE: StatementElement = {
  id: "variable",
  component: VariableElement,
  exists: (iface, statement) => statement.value != null,
};
const DATASET: StatementElement = {
  id: "dataset",
  component: DatasetElement,
  exists: (iface, statement) => true, // unknown, but doesn't matter
};
const RUN: StatementElement = {
  id: "run",
  component: RunElement,
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
register(StatementType.Text, { primaryPart: "text", hasTags: true, elements: [{ ...TEXT, showIfEmpty: false }] });
register(StatementType.Code, {
  primaryPart: "code",
  foldable: "function-self",
  needsDeclaration: true,
  hasBases: true,
  hasTags: true,
  hasTriggers: true,
  extraControls: [RUN_META],
  elements: [TEXT, FUNCTION_TYPE, { ...CODE, showIfEmpty: true }, RUN],
});
register(StatementType.Type, {
  primaryPart: "type",
  foldable: "list-self",
  needsDeclaration: true,
  hasBases: true,
  hasTags: true,
  elements: [TEXT, { ...LIST_TYPE, showIfEmpty: true }],
});
register(StatementType.Task, {
  primaryPart: "declaration",
  foldable: "function-all",
  needsDeclaration: true,
  hasTags: true,
  hasBases: true,
  hasTriggers: true,
  extraControls: [RUN_META],
  elements: [TEXT, { ...FUNCTION_TYPE, showIfEmpty: true }, RUN],
});
register(StatementType.Reference, {
  primaryPart: "reference",
  needsDeclaration: false,
  hasTags: true,
  hasBases: true,
  hasTriggers: true,
  elements: [TEXT],
});
register(StatementType.Variable, {
  primaryPart: "variable",
  foldable: "list-all",
  needsDeclaration: true,
  hasTags: true,
  elements: [TEXT, { ...VARIABLE, showIfEmpty: true }],
});
register(StatementType.Dataset, {
  primaryPart: "dataset",
  foldable: "list-all",
  needsDeclaration: true,
  hasTags: true,
  hasBases: true,
  extraControls: [DATASET_SEARCH],
  elements: [TEXT, DATASET],
});
register(StatementType.Tag, {
  primaryPart: "declaration",
  foldable: "list-all",
  needsDeclaration: true,
  hasTags: true,
  hasBases: true,
  elements: [TEXT],
});
