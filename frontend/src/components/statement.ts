import { useFragment, type FragmentType } from "@/gql";
import {
  StatementModifier,
  StatementType,
  SymbolType,
  TypeHint,
  TypeTag,
  type DatasetRecord,
  type InterpSymbol,
  type SimpleTypeNode,
  type TypeNodeCreateInput,
  type TypeNodeUpdateInput,
} from "@/gql/graphql";
import { useActions } from "@/state/actions";
import type { StatementHeader } from "@/state/editor";
import { FileHeaderType, SimpleTypeNodeType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { closeTransaction, openTransaction, useOperations } from "@/state/operations";
import { newDatasetRecordId, newTypeNodeId, newTypeNodeKey } from "@/state/operations/statement";
import { TypeFlag } from "@/state/runtime";
import { INTEGER_ZERO } from "@/utils/fractional";
import { syncProperty } from "@/utils/sync";
import { computed, inject, type Ref } from "vue";

// not using Symbol here to improve hotreload experience (Symbol is not a constant)
export const STATEMENT_CONTEXT = "__statementContext__" as const;

export type StatementContext = {
  depth: Ref<number>;
  xOffset: Ref<number>;
  lineNumberBase: Ref<number>;
  readonly: Ref<boolean>;
  focused: Ref<boolean>;
  editing: Ref<boolean>;
  statement: Ref<FragmentType<typeof StatementContentType>>;
  file: Ref<FragmentType<typeof FileHeaderType>>;
  reference: Ref<InterpSymbol | { id: string; name: string } | null>;
  destroyed: Ref<boolean>;
};

export type StatementAction = {
  label: string;
  icon: any;
  action: (statement: StatementHeader) => void;
  active?: boolean;
  disabled?: boolean;
};

export type RecordAction = {
  label: string;
  icon: any;
  action: (record: DatasetRecord) => void;
  active?: boolean;
  disabled?: boolean;
};

export type TypeAction = {
  label: string;
  icon: any;
  keepOpen?: boolean;
  action: (type: SimpleType) => void;
};

export function useStatementContext() {
  const context = inject<StatementContext>(STATEMENT_CONTEXT);
  if (context == null) {
    throw new Error("StatementContext is not available.");
  }

  // state

  const statement = computed(() => useFragment(StatementContentType, context.statement.value));
  const file = computed(() => useFragment(FileHeaderType, context.file.value));
  const reference = computed(() => useFragment(StatementHeaderType, context.reference.value));

  const rootTypeTag = computed(() => statement.value.rootTypeTag);
  const typeNodes = computed(() =>
    statement.value.typeNodes
      ?.map((n) => useFragment(SimpleTypeNodeType, n))
      .filter((n) => n.deletedAt == null)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
  );
  const typeNodesByName = computed(() => {
    const typeNodesByName: Record<string, FragmentType<typeof SimpleTypeNodeType>> = {};
    for (const typeNode of typeNodes.value) {
      if (typeNode.name != null) {
        typeNodesByName[typeNode.name] = typeNode;
      }
    }
    return typeNodesByName;
  });

  const symbolSubtype: Ref<string | null> = computed(() => {
    if (statement.value.symbolType == SymbolType.Type) {
      if (statement.value.rootTypeTag == TypeTag.Enum) {
        return "choice";
      } else {
        return "struct";
      }
    } else if (statement.value.symbolType == SymbolType.Data) {
      if ((statement.value.rootTypeFlags ?? 0) & TypeFlag.IsArray) {
        return "table";
      } else {
        return "value";
      }
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
    // (this can actually be fixed now with navigation context)
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

  async function morphToDefinition(config: {
    symbolType: SymbolType | null;
    name?: string;
    rootTypeFlags?: number;
    rootTypeTag?: TypeTag;
  }) {
    if (statement.value.type != StatementType.Blank || config.symbolType == null) {
      throw new Error("cannot morph from non-blank without symbol type: " + statement.value.id);
    }
    const defaults = getDefaultSymbolDefinition(config.symbolType);
    let newTypeTag;
    if (config.rootTypeTag) {
      newTypeTag = config.rootTypeTag;
    } else if (rootTypeTag.value == null) {
      newTypeTag = defaults.rootTypeTag;
    } else {
      newTypeTag = isTypeTagCompatible(rootTypeTag.value, config.symbolType) ? rootTypeTag.value : defaults.rootTypeTag;
    }

    const tx = openTransaction({ name: "morph init", blockPartialUndo: true, collapseUndoToFirst: true });
    await ops.statement.morph(
      tx,
      statement.value.id,
      {
        type: statement.value.type,
        symbolType: statement.value.symbolType ?? undefined,
        name: undefined,
        lang: statement.value.lang ?? undefined,
        rootTypeTag: statement.value.rootTypeTag ?? undefined,
        rootTypeFlags: statement.value.rootTypeFlags ?? undefined,
      },
      {
        type: StatementType.Definition,
        symbolType: config?.symbolType,
        name: config.name,
        lang: config.rootTypeTag ?? defaults.language,
        rootTypeTag: newTypeTag,
        rootTypeFlags: config.rootTypeFlags,
      }
    );
    // create default record for value (must exist)
    if (config.symbolType == SymbolType.Data) {
      await ops.symbol.createRecord(tx, newDatasetRecordId(), statement.value.id, INTEGER_ZERO, {});
    }
    closeTransaction(tx);
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
      context?.reference.value?.name ?? null,
      reference?.id ?? null,
      reference?.name ?? null
    );
  }

  // :EditableSyncDance
  // There is a bit of a delicate dance when syncing these properties since we want to
  // preserve the users local edits, but also want to sync from server/cache when not editing
  // since that includes multiplayer updates and redo/undo data while not editing.
  // That's why we only save the properties while the user is editing, otherwise we would have

  // We can only save if the statement wasn't deleted, and often the statement component owning the
  // statement reference is destroyed before the deletedAt is set, so we also treat the context destroy as delete.
  const isDeleted = computed(() => statement.value.deletedAt != null || context.destroyed.value);

  function syncName(content: Ref<string>, editing: Ref<boolean | undefined>) {
    return syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.name ?? ""),
      write: () => ops.statement.rename(null, statement.value.id, statement.value.name ?? "", content.value),
      enabled: computed(() => !isDeleted.value),
    });
  }

  function syncCode(content: Ref<string>, editing: Ref<boolean | undefined>) {
    return syncProperty({
      value: content,
      editing,
      read: () => (content.value = statement.value?.code ?? ""),
      write: () => {
        ops.symbol.updateStatementCode(null, statement.value.id, statement.value.code ?? "", content.value);
      },
      debounceMs: 1000,
      debounceMaxWait: 3000,
      enabled: computed(() => !isDeleted.value),
    });
  }

  // :StatementCodeTextReuse
  const syncText = syncCode;

  function syncDescription(content: Ref<string>, editing: Ref<boolean | undefined>) {
    return syncProperty({
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
      enabled: computed(() => !isDeleted.value),
    });
  }

  // type node helpers

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
      hint: newTypeNode.hint ?? null,
      name: newTypeNode.name ?? oldTypeNode.name,
      description: newTypeNode.description ?? oldTypeNode.description,
      value: newTypeNode.value,
      reference: newTypeNode.reference,
      flags: newTypeNode.flags,
    };
    await ops.symbol.updateTypeNode(
      null,
      makeTypeNodeUpdate(oldTypeNode as SimpleTypeNode),
      makeTypeNodeUpdate(newTypeNode as SimpleTypeNode)
    );
  }

  async function moveTypeNode(typeNode: SimpleType, orderKey: string) {
    const oldTypeNode = typeNodes.value?.find((n) => n.id == typeNode.id);
    if (!oldTypeNode) {
      throw new Error("cannot move type node that doesn't exist");
    }
    await ops.symbol.moveTypeNode(null, typeNode.id, oldTypeNode.orderKey, orderKey);
  }

  async function deleteTypeNode(typeNode: { id: string }) {
    const oldTypeNode = typeNodes.value?.find((n) => n.id == typeNode.id);
    if (!oldTypeNode) {
      throw new Error("cannot delete type node that doesn't exist");
    }
    await ops.symbol.softDeleteTypeNode(null, statement.value.id, makeTypeNodeInput(statement.value.id, oldTypeNode));
  }

  // basic inline actions
  return {
    // state
    statement,
    file,
    reference,
    depth: context.depth,
    readonly: context.readonly,
    focused: context.focused,
    editing: context.editing,
    xOffset: context.xOffset,
    typeRootTag: rootTypeTag,
    symbolSubtype,
    typeNodes,
    typeNodesByName,
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
    moveTypeNode,
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
    hint: typeNode.hint ?? null,
    key: newTypeNodeKey(),
    orderKey: typeNode.orderKey,
    referenceId: typeNode.reference?.id ?? null,
    description: typeNode.description ?? null,
    name: typeNode.name ?? null,
    value: typeNode.value ?? null,
    flags: typeNode.flags,
  };
}

function makeTypeNodeUpdate(typeNode: SimpleType): TypeNodeUpdateInput {
  return {
    id: typeNode.id,
    tag: typeNode.tag,
    hint: typeNode.hint ?? null,
    referenceId: typeNode.reference?.id ?? null,
    description: typeNode.description ?? null,
    name: typeNode.name ?? null,
    value: typeNode.value ?? null,
    flags: typeNode.flags,
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
  hint?: TypeHint | null;
  key?: string;
  orderKey?: string;
  value?: any;
  reference?: { id: string; name?: string };
  flags?: number;
}): SimpleType {
  const typeNodeData: SimpleType = {
    id: newTypeNodeId(),
    name: data.name ?? null,
    tag: data.tag,
    hint: data.hint ?? null,
    key: data.key ?? newTypeNodeKey(),
    orderKey: data.orderKey ?? INTEGER_ZERO,
    value: data.value ?? null,
    reference: data.reference,
    flags: data.flags ?? 0,
  };
  return typeNodeData;
}

export type SimpleType = Omit<SimpleTypeNode, "statement" | "createdAt" | "updatedAt" | "__typename">;

export const STRING_TYPE_NODE = makeTypeNode({ tag: TypeTag.String });
export const NAME_TYPE_NODE = makeTypeNode({ tag: TypeTag.String, hint: TypeHint.Name });
export const ANY_TYPE_NODE = makeTypeNode({ tag: TypeTag.Any });

export function getEnumColor(type: { key: string }) {
  /* Generate a strong color for the type */
  // Convert the client ID to a numerical seed
  const seed = type.key.split("").reduce((acc, char) => {
    return acc * 31 + char.charCodeAt(0);
  }, 0);

  // Generate a random pastel color based on the seed
  const hue = seed % 360;
  const saturation = 70 + (seed % 25); // Range: 70-95
  const lightness = 70;

  return `hsl(${hue}, ${saturation}%, ${lightness}%)`;
}
