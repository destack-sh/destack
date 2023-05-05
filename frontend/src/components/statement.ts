import { useFragment, type FragmentType } from "@/gql";
import {
  StatementModifier,
  StatementType,
  SymbolType,
  TypeTag,
  type InterpSymbol,
  type SimpleTypeNode,
  type TypeNodeCreateInput,
  type TypeNodeUpdateInput,
} from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { FileHeaderType, SimpleTypeNodeType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { computed, inject, type Ref } from "vue";

import { TYPETAG_KEYWORD } from "@/state/editor";
import { newTypeNodeId } from "@/state/operations/statement";
import { contextOf } from "@/state/runtime";
import { INTEGER_ZERO } from "@/utils/fractional";
import { syncProperty } from "@/utils/sync";

// not using Symbol here to improve hotreload experience (Symbol is not a constant)
export const STATEMENT_CONTEXT = "__statementContext__" as const;

export type StatementContext = {
  depth: number;
  xOffset: number;
  lineNumberBase: number;
  readonly: boolean;
  focused: boolean;
  editing: boolean;
  statement: FragmentType<typeof StatementContentType>;
  file: FragmentType<typeof FileHeaderType>;
  reference: InterpSymbol | { id: string; name: string } | null;
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
      .filter((n) => n.deletedAt == null)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
  );
  const records = computed(() =>
    statement.value.records.edges
      .map((n) => n.node)
      .filter((n) => n.deletedAt == null)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
  );

  const symbolSubtype: Ref<string | null> = computed(() => {
    if (
      (statement.value.type == StatementType.Definition ||
        (statement.value.type == StatementType.Blank && statement.value.symbolType == SymbolType.Type)) &&
      rootTypeTag.value == TypeTag.Enum
    ) {
      return "choice";
    }
    return null;
  });

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

  const ops = useOperations();

  async function morphToBlank() {
    await ops.statement.morph(
      null,
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: StatementType.Blank }
    );
  }

  async function morphToComment(text?: string) {
    const updateCode = ops.symbol.updateStatementCode(null, statement.value.id, statement.value.code ?? "", text);
    const morphType = ops.statement.morph(
      null,
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
    await ops.statement.morph(
      null,
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
    await ops.statement.morph(
      null,
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: StatementType.Reference, symbolType, name }
    );
  }

  async function setModifier(modifier: StatementModifier | null) {
    await ops.statement.modify(null, statement.value.id, statement.value.modifier ?? null, modifier);
  }

  async function setSymbolType(symbolType: SymbolType | null) {
    await ops.statement.morph(
      null,
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: statement.value.type, symbolType: symbolType ?? undefined }
    );
  }

  async function setSymbolTypeEnum() {
    // morphs to type symbol with an enum as head type node
    await ops.statement.morph(
      null,
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

  async function setReference(reference: { id: string; name: string } | null) {
    await ops.statement.setReference(
      null,
      statement.value.id,
      statement.value.reference?.id,
      context?.value.reference?.name ?? null,
      reference?.id ?? null,
      reference?.name ?? null
    );
  }

  // one-way syncs from current state to backend (:Singleplayer)

  // :EditableSyncDance
  // There is a bit of a delicate dance when syncing these properties since we want to
  // preserve the users local edits, but also want to sync from server/cache when not editing
  // since that includes multiplayer updates and redo/undo data while not editing.
  // That's why we only save the properties while the user is editing, otherwise we would have

  function syncName(content: Ref<string>, editing: Ref<boolean | undefined>) {
    syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.name ?? ""),
      write: () => ops.statement.rename(null, statement.value.id, statement.value.name ?? "", content.value),
    });
  }

  function syncCode(content: Ref<string>, editing: Ref<boolean | undefined>) {
    syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.code ?? ""),
      write: () => ops.symbol.updateStatementCode(null, statement.value.id, statement.value.code ?? "", content.value),
      debounceMs: 1000,
      debounceMaxWait: 5000,
    });
  }

  // :StatementCodeTextReuse
  const syncText = syncCode;

  function syncDescription(content: Ref<string>, editing: Ref<boolean | undefined>) {
    syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.description ?? ""),
      write: () =>
        ops.symbol.updateStatementDescription(
          null,
          statement.value.id,
          statement.value.description ?? "",
          content.value
        ),
    });
  }

  // one-way writes to backend (:Singleplayer)

  async function createTypeNode(typeNode: SimpleType) {
    await ops.symbol.createTypeNode(null, statement.value.id, { ...typeNode, statementId: statement.value.id });
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
      value: newTypeNode.value,
      reference: newTypeNode.reference,
      isArray: newTypeNode.isArray,
      isNullable: newTypeNode.isNullable,
      isOutput: newTypeNode.isOutput,
    };
    await ops.symbol.updateTypeNode(
      null,
      makeTypeNodeUpdate(oldTypeNode as SimpleTypeNode),
      makeTypeNodeUpdate(newTypeNode as SimpleTypeNode)
    );
  }

  async function deleteTypeNode(typeNode: { id: string }) {
    const oldTypeNode = typeNodes.value?.find((n) => n.id == typeNode.id);
    if (!oldTypeNode) {
      throw new Error("cannot delete type node that doesn't exist");
    }
    await ops.symbol.softDeleteTypeNode(null, statement.value.id, makeTypeNodeInput(statement.value.id, oldTypeNode));
  }

  return {
    // state
    statement,
    file,
    reference,
    depth: computed(() => context.value.depth),
    readonly: computed(() => context.value.readonly || statement.value.generated),
    focused: computed(() => context.value.focused),
    editing: computed(() => context.value.editing),
    xOffset: computed(() => context.value.xOffset),
    lineNumberBase: computed(() => context.value.lineNumberBase),
    typeRootTag: rootTypeTag,
    symbolSubtype,
    typeNodes,
    records,
    // actions
    actions,
    navigateUp,
    navigateDown,
    escape,
    morphToBlank,
    morphToComment,
    morphToDefinition,
    morphToReference,
    setModifier,
    setSymbolType,
    setSymbolTypeEnum,
    setReference,
    syncText,
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
  } else if (symbolType == SymbolType.Data) {
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
  } else if (symbolType == SymbolType.Data) {
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

export const PRIMITIVE_TYPES = [
  TypeTag.String,
  TypeTag.Boolean,
  TypeTag.Number,
  TypeTag.File,
  TypeTag.Embedding,
  TypeTag.Null,
];
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
