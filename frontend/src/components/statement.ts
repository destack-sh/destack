import { useFragment, type FragmentType } from "@/gql";
import {
  StatementModifier,
  StatementType,
  SymbolType,
  TypeTag,
  type SimpleTypeNode,
  type TypeNodeCreateInput,
  type TypeNodeUpdateInput,
} from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { FileHeaderType, SimpleTypeNodeType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, inject, watch, type Ref } from "vue";

import { newTypeNodeId } from "@/state/operations/statement";
import { INTEGER_ZERO } from "@/utils/fractional";
import { TYPETAG_KEYWORD } from "@/state/editor";
import { contextOf } from "@/state/runtime";

export const STATEMENT_CONTEXT = Symbol();

export type StatementContext = {
  depth: number;
  xOffset: number;
  lineNumberBase: number;
  readonly: boolean;
  focused: boolean;
  editing: boolean;
  statement: FragmentType<typeof StatementContentType>;
  file: FragmentType<typeof FileHeaderType>;
  reference: FragmentType<typeof StatementHeaderType> | null;
};

export function useStatementContext() {
  const context = inject<Ref<StatementContext>>(STATEMENT_CONTEXT);
  if (context == null) {
    throw new Error("StatementContext is not available.");
  }

  // state

  const statement = computed(() => useFragment(StatementContentType, context.value.statement));
  const file = computed(() => useFragment(FileHeaderType, context.value.file));
  const reference = computed(() => useFragment(StatementHeaderType, context.value.reference));

  const rootTypeTag = computed(() => statement.value.rootTypeTag);
  const typeNodes = computed(() =>
    statement.value.typeNodes
      ?.map((n) => useFragment(SimpleTypeNodeType, n))
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
  );

  // basic actions

  const actions = useActions();

  function navigateUp() {
    actions.apply("statement.moveFocusUp");
  }

  function navigateDown() {
    actions.apply("statement.moveFocusDown");
  }

  function escape() {
    actions.apply("statement.stopEditingCurrent");
  }

  function deleteSelf() {
    actions.apply("statement.deleteCurrent");
  }

  function tryDeleteLeft() {
    // should really be handled here :MissingStatementContext
    actions.apply("statement.deleteCurrentLeft");
  }

  function insertBelow() {
    actions.apply("statement.insertBelowCurrent");
  }

  function insertAbove() {
    actions.apply("statement.insertAboveCurrent");
  }

  // self mutations

  const operations = useOperations();

  async function morphToComment(text?: string) {
    const updateCode = operations.symbol.updateStatementCode(statement.value.id, statement.value.code ?? "", text);
    const morphType = operations.statement.morph(
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: StatementType.Comment }
    );
    await Promise.all([morphType, updateCode]);
  }

  async function morphToDefinition(symbolType: SymbolType | null, name: string) {
    if (statement.value.type != StatementType.Blank || symbolType == null) {
      throw new Error("cannot morph from non-blank without symbol type: " + statement.value.id);
    }
    const defaults = getDefaultSymbolDefinition(symbolType);
    let newTypeTag;
    if (rootTypeTag.value == null) {
      newTypeTag = defaults.rootTypeTag;
    } else {
      newTypeTag = isTypeTagCompatible(rootTypeTag.value, symbolType) ? rootTypeTag.value : defaults.rootTypeTag;
    }
    await operations.statement.morph(
      statement.value.id,
      {
        type: statement.value.type,
        symbolType: statement.value.symbolType ?? undefined,
        name: undefined,
        lang: statement.value.lang ?? undefined,
        rootTypeTag: statement.value.rootTypeTag,
      },
      {
        type: StatementType.Definition,
        symbolType,
        name,
        lang: defaults.language,
        rootTypeTag: newTypeTag,
      }
    );
  }

  async function morphToReference(symbolType: SymbolType, name: string) {
    if (statement.value.type != StatementType.Blank) {
      throw new Error("cannot morph from non-blank to reference: " + statement.value.id);
    }
    await operations.statement.morph(
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: StatementType.Reference, symbolType, name }
    );
  }

  async function setModifier(modifier: StatementModifier | null) {
    await operations.statement.modify(statement.value.id, statement.value.modifier ?? null, modifier);
  }

  async function setSymbolType(symbolType: SymbolType | null) {
    await operations.statement.morph(
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: statement.value.type, symbolType: symbolType ?? undefined }
    );
  }

  async function setSymbolTypeEnum() {
    // morphs to type symbol with an enum as head type node
    await operations.statement.morph(
      statement.value.id,
      {
        type: statement.value.type,
        symbolType: statement.value.symbolType ?? undefined,
        name: statement.value.name ?? undefined,
        rootTypeTag: statement.value.rootTypeTag ?? undefined,
      },
      {
        type: statement.value.type,
        symbolType: SymbolType.Type,
        name: statement.value.name ?? undefined,
        rootTypeTag: TypeTag.Enum,
      }
    );
  }

  async function setReference(reference: { id: string } | null) {
    await operations.statement.setReference(statement.value.id, statement.value.reference?.id, reference?.id ?? null);
  }

  // one-way syncs from current state to backend (:Singleplayer)

  function syncName(content: Ref<string>) {
    function syncName() {
      operations.statement.rename(statement.value.id, statement.value.name ?? null, content.value);
    }
    watch(content, useDebounceFn(syncName, 200, { maxWait: 1000 }));
  }

  function syncCode(content: Ref<string>) {
    function saveCode() {
      operations.symbol.updateStatementCode(statement.value.id, statement.value.code ?? "", content.value);
    }
    watch(content, useDebounceFn(saveCode, 200, { maxWait: 1000 }));
  }

  function syncDescription(content: Ref<string>) {
    function saveDescription() {
      operations.symbol.updateStatementDescription(
        statement.value.id,
        statement.value.description ?? "",
        content.value
      );
    }
    watch(content, useDebounceFn(saveDescription, 200, { maxWait: 1000 }));
  }

  // one-way writes to backend (:Singleplayer)

  async function createTypeNode(typeNode: SimpleType) {
    await operations.statement.createTypeNode(statement.value.id, { ...typeNode, statementId: statement.value.id });
  }

  async function updateTypeNode(typeNode: SimpleType, newTypeNode: SimpleType) {
    const oldTypeNode = typeNodes.value?.find((n) => n.id == typeNode.id);
    if (!oldTypeNode) {
      throw new Error("cannot update type node that doesn't exist");
    }
    newTypeNode = {
      ...oldTypeNode,
      tag: newTypeNode.tag ?? oldTypeNode.tag,
      name: newTypeNode.name ?? oldTypeNode.name,
      description: newTypeNode.description ?? oldTypeNode.description,
      reference: newTypeNode.reference,
      isArray: newTypeNode.isArray,
      isNullable: newTypeNode.isNullable,
      isOutput: newTypeNode.isOutput,
    };
    await operations.statement.updateTypeNode(
      makeTypeNodeUpdate(oldTypeNode as SimpleTypeNode),
      makeTypeNodeUpdate(newTypeNode as SimpleTypeNode)
    );
  }

  async function deleteTypeNode(typeNode: { id: string }) {
    const oldTypeNode = typeNodes.value?.find((n) => n.id == typeNode.id);
    if (!oldTypeNode) {
      throw new Error("cannot delete type node that doesn't exist");
    }
    await operations.statement.deleteTypeNode(statement.value.id, makeTypeNodeInput(statement.value.id, oldTypeNode));
  }

  return {
    // state
    statement,
    file,
    reference,
    depth: computed(() => context.value.depth),
    readonly: computed(() => context.value.readonly),
    focused: computed(() => context.value.focused),
    editing: computed(() => context.value.editing),
    xOffset: computed(() => context.value.xOffset),
    lineNumberBase: computed(() => context.value.lineNumberBase),
    typeRootTag: rootTypeTag,
    typeNodes,
    // actions
    actions,
    navigateUp,
    navigateDown,
    escape,
    morphToComment,
    morphToDefinition,
    morphToReference,
    setModifier,
    setSymbolType,
    setSymbolTypeEnum,
    setReference,
    syncName,
    syncCode,
    syncDescription,
    createTypeNode,
    updateTypeNode,
    deleteTypeNode,
    deleteSelf,
    tryDeleteLeft,
    insertAbove,
    insertBelow,
  };
}

function makeTypeNodeInput(id: string, typeNode: SimpleType): TypeNodeCreateInput {
  return {
    statementId: id,
    id: typeNode.id,
    tag: typeNode.tag,
    orderKey: typeNode.orderKey,
    referenceId: typeNode.reference?.id ?? null,
    description: typeNode.description ?? null,
    name: typeNode.name ?? null,
    value: typeNode.value ?? null,
    isArray: typeNode.isArray ?? false,
    isNullable: typeNode.isNullable ?? false,
    isOutput: typeNode.isOutput ?? false,
  };
}

function makeTypeNodeUpdate(typeNode: SimpleType): TypeNodeUpdateInput {
  return {
    id: typeNode.id,
    tag: typeNode.tag,
    referenceId: typeNode.reference?.id ?? null,
    description: typeNode.description ?? null,
    name: typeNode.name ?? null,
    value: typeNode.value ?? null,
    isArray: typeNode.isArray ?? false,
    isNullable: typeNode.isNullable ?? false,
    isOutput: typeNode.isOutput ?? false,
  };
}

export function getDefaultSymbolDefinition(symbolType: SymbolType): {
  language?: string;
  rootTypeTag?: TypeTag;
} {
  if (symbolType == SymbolType.Code) {
    return {
      language: "python",
      rootTypeTag: TypeTag.Function,
    };
  } else if (symbolType == SymbolType.Task) {
    return {
      rootTypeTag: TypeTag.Function,
    };
  } else if (symbolType == SymbolType.Dataset) {
    return {
      language: "jsonl",
      rootTypeTag: TypeTag.Struct,
    };
  } else if (symbolType == SymbolType.Type) {
    // default to struct
    return {
      rootTypeTag: TypeTag.Struct,
    };
  } else {
    // no special content for other symbol types
    return {};
  }
}

export function isTypeTagCompatible(tag: TypeTag, symbolType: SymbolType): boolean {
  if (symbolType == SymbolType.Code || symbolType == SymbolType.Task) {
    return tag == TypeTag.Function;
  } else if (symbolType == SymbolType.Dataset) {
    return tag == TypeTag.Struct;
  } else if (symbolType == SymbolType.Type) {
    return tag == TypeTag.Struct || tag == TypeTag.Enum;
  } else {
    return false;
  }
}

export function makeTypeNode(data: {
  name?: string | null;
  tag: TypeTag;
  orderKey?: string;
  value?: any;
  reference?: { id: string; name?: string };
  isOutput?: boolean;
  isNullable?: boolean;
  isArray?: boolean;
}): SimpleType {
  const typeNodeData: SimpleType = {
    id: newTypeNodeId(),
    name: data.name ?? null,
    tag: data.tag,
    orderKey: data.orderKey ?? INTEGER_ZERO,
    value: data.value ?? null,
    reference: data.reference,
    isOutput: data.isOutput ?? false,
    isNullable: data.isNullable ?? false,
    isArray: data.isArray ?? false,
  };
  return typeNodeData;
}

export type SimpleType = Omit<SimpleTypeNode, "statement" | "createdAt" | "updatedAt" | "__typename">;

export const STRING_TYPE_NODE = makeTypeNode({ tag: TypeTag.String });
export const NUMBER_TYPE_NODE = makeTypeNode({ tag: TypeTag.Number });
export const BOOLEAN_TYPE_NODE = makeTypeNode({ tag: TypeTag.Boolean });
export const ANY_TYPE_NODE = makeTypeNode({ tag: TypeTag.Any });
export const NULL_TYPE_NODE = makeTypeNode({ tag: TypeTag.Null });

export const PRIMITIVE_TYPES = [TypeTag.Any, TypeTag.String, TypeTag.Boolean, TypeTag.Number, TypeTag.Null];
export const PRIMITIVE_TYPE_NODES = PRIMITIVE_TYPES.map((tag) => makeTypeNode({ tag }));

export function renderSimpleType(node: SimpleType): string {
  let renderedElement: string;
  if (PRIMITIVE_TYPES.includes(node.tag)) {
    renderedElement = TYPETAG_KEYWORD[node.tag];
  } else if (node.tag == TypeTag.TypeReference || node.reference != null) {
    if (node.reference != null) {
      renderedElement = contextOf(node.reference)?.symbol.name ?? "???";
    } else {
      renderedElement = node.reference?.name ?? "...";
    }
  } else {
    throw new Error(`unexpected type node ${node.tag}`);
  }

  let rendered: string = renderedElement;
  if (node.isArray) {
    rendered = "list " + renderedElement;
  }
  if (node.isNullable) {
    rendered = rendered + "?";
  }

  return rendered;
}
