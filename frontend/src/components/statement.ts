import { useFragment, type FragmentType } from "@/gql";
import {
  StatementModifier,
  StatementType,
  SymbolType,
  TypeTag,
  type StatementTypeNodeDataCreateInput,
  type TypeNode,
  type TypeNodeData,
} from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { FileHeaderType, StatementContentType, StatementHeaderType, TypeNodeDataType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useDebounceFn } from "@vueuse/shared";
import { computed, inject, watch, type Ref } from "vue";

import { newTypeNodeDataId } from "@/state/operations/statement";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";

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

  const typeNodes = computed(() => statement.value.typeNodes?.map((n) => useFragment(TypeNodeDataType, n)));
  const typeNodeRoot = computed(() => typeNodes.value?.find((n) => n.parentId == null));
  const typeNodesChildren = computed(() =>
    typeNodes.value
      ?.filter((n) => n.parentId == typeNodeRoot.value?.id)
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

  function tryDeleteAbove() {
    // should really be handled here :MissingStatementContext
    actions.apply("statement.deleteAboveCurrent");
  }

  function insertBelow() {
    actions.apply("statement.insertBelowCurrent");
  }

  function insertAbove() {
    actions.apply("statement.insertAboveCurrent");
  }

  // self mutations

  const operations = useOperations();

  async function morphToComment() {
    await operations.statement.morph(
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: StatementType.Comment }
    );
  }

  async function morphToDefinition(symbolType: SymbolType | null, name: string) {
    if (statement.value.type != StatementType.Blank || symbolType == null) {
      throw new Error("cannot morph from non-blank without symbol type: " + statement.value.id);
    }
    const defaults = getDefaultSymbolDefinition(symbolType);
    // set new type nodes to defaults if they aren't compatible
    // we may keep previous type nodes because they may be used to mark the specific type
    // e.g. for enums we create a regular type symbol but pre-morph its head type node to enum
    const oldTypeNodes = statement.value.typeNodes?.map((n) => mapToTypeNodeDataInput(statement.value.id, n));
    let newTypeNodes;
    if (typeNodeRoot.value?.tag != null && isTypeTagCompatible(typeNodeRoot.value.tag, symbolType)) {
      newTypeNodes = oldTypeNodes;
    } else {
      newTypeNodes = defaults.typeNodes?.map((n) => mapToTypeNodeDataInput(statement.value.id, n));
    }

    await operations.statement.morph(
      statement.value.id,
      {
        type: statement.value.type,
        symbolType: statement.value.symbolType ?? undefined,
        name: undefined,
        typeNodes: oldTypeNodes,
      },
      {
        type: StatementType.Definition,
        symbolType,
        name,
        language: defaults.language,
        typeNodes: newTypeNodes,
      }
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
    const oldTypeNodes = statement.value.typeNodes?.map((n) => mapToTypeNodeDataInput(statement.value.id, n));
    await operations.statement.morph(
      statement.value.id,
      {
        type: statement.value.type,
        symbolType: statement.value.symbolType ?? undefined,
        name: statement.value.name ?? undefined,
        typeNodes: oldTypeNodes,
      },
      {
        type: statement.value.type,
        symbolType: SymbolType.Type,
        name: statement.value.name ?? undefined,
        typeNodes: makeEnumTypeNodeData().map((n) => mapToTypeNodeDataInput(statement.value.id, n)),
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

  async function createTypeNode(typeNode: TypeNodeData) {
    await operations.statement.createTypeNode(statement.value.id, mapToTypeNodeDataInput(statement.value.id, typeNode));
  }

  async function updateTypeNode(typeNode: TypeNodeData) {
    const oldTypeNode = typeNodes.value?.find((n) => n.id == typeNode.id);
    if (!oldTypeNode) {
      throw new Error("cannot update type node that doesn't exist");
    }
    await operations.statement.updateTypeNode(
      statement.value.id,
      mapToTypeNodeDataInput(statement.value.id, oldTypeNode),
      mapToTypeNodeDataInput(statement.value.id, typeNode)
    );
  }

  async function deleteTypeNode(typeNode: TypeNodeData) {
    const oldTypeNode = typeNodes.value?.find((n) => n.id == typeNode.id);
    if (!oldTypeNode) {
      throw new Error("cannot delete type node that doesn't exist");
    }
    await operations.statement.deleteTypeNode(statement.value.id, mapToTypeNodeDataInput(statement.value.id, typeNode));
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
    typeNodes,
    typeNodeRoot,
    typeNodesChildren,
    // actions
    actions,
    navigateUp,
    navigateDown,
    escape,
    morphToComment,
    morphToDefinition,
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
    tryDeleteAbove,
    insertAbove,
    insertBelow,
  };
}

function mapToTypeNodeDataInput(id: string, typeNodeData: TypeNodeData): StatementTypeNodeDataCreateInput {
  return {
    id: id,
    nodeId: typeNodeData.id,
    tag: typeNodeData.tag,
    orderKey: typeNodeData.orderKey,
    parentId: typeNodeData.parentId ?? null,
    reference: typeNodeData.reference ?? null,
    description: typeNodeData.description ?? null,
    name: typeNodeData.name ?? null,
    value: typeNodeData.value ?? null,
  };
}

export function getDefaultSymbolDefinition(symbolType: SymbolType): { language?: string; typeNodes?: TypeNodeData[] } {
  if (symbolType == SymbolType.Code) {
    return {
      language: "python",
      typeNodes: makeFunctionTypeNodeData(),
    };
  } else if (symbolType == SymbolType.Task) {
    return {
      typeNodes: makeFunctionTypeNodeData(),
    };
  } else if (symbolType == SymbolType.Dataset) {
    return {
      language: "jsonl",
      typeNodes: [makeTypeNodeData({ name: "element", tag: TypeTag.Struct })],
    };
  } else if (symbolType == SymbolType.Type) {
    // default to struct
    return {
      typeNodes: [makeTypeNodeData({ tag: TypeTag.Struct })],
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

export function mapToTypeNode(typeNodes: TypeNodeData[], rootId?: string): TypeNode {
  let rootNodeData;
  if (rootId == null) {
    rootNodeData = typeNodes.find((n) => n.parentId == null);
  } else {
    rootNodeData = typeNodes.find((n) => n.id == rootId);
  }
  if (!rootNodeData) {
    throw new Error("type nodes do not contain root node");
  }

  const mappedNodes: Record<string, TypeNode> = {};
  function walkMap(typeNode: TypeNodeData) {
    if (typeNode.id in mappedNodes) {
      throw new Error("circular type nodes"); // just in case
    }
    const mapped = {
      name: typeNode.name ?? null,
      tag: typeNode.tag,
      description: typeNode.description ?? null,
      reference: typeNode.reference ?? null,
      children: null,
    } as TypeNode;
    mappedNodes[typeNode.id] = mapped;

    const children = typeNodes.filter((n) => n.parentId == typeNode.id);
    if (children) {
      mapped.children = children.map((c) => walkMap(c));
    }
    return mapped;
  }

  return walkMap(rootNodeData);
}

export function makeTypeNodeData(data: {
  name?: string;
  tag: TypeTag;
  parentId?: string;
  orderKey?: string;
  value?: any;
  reference?: string;
}) {
  const typeNodeData: TypeNodeData = {
    __typename: "TypeNodeData",
    id: newTypeNodeDataId(),
    name: data.name ?? null,
    tag: data.tag,
    parentId: data.parentId ?? null,
    orderKey: data.orderKey ?? INTEGER_ZERO,
    value: data.value ?? null,
    reference: data.reference ?? null,
  };
  return typeNodeData;
}

export function makeFunctionTypeNodeData(): TypeNodeData[] {
  const functionType: TypeNodeData = makeTypeNodeData({
    tag: TypeTag.Function,
  });
  const inputType: TypeNodeData = makeTypeNodeData({
    name: "input",
    tag: TypeTag.Struct,
    parentId: functionType.id,
  });
  const outputType: TypeNodeData = makeTypeNodeData({
    name: "output",
    tag: TypeTag.Null,
    parentId: functionType.id,
    orderKey: generateKeyBetween(inputType.orderKey, null),
  });
  return [functionType, inputType, outputType];
}

export function makeEnumTypeNodeData(memberType: TypeTag = TypeTag.String): TypeNodeData[] {
  const enumType: TypeNodeData = makeTypeNodeData({
    tag: TypeTag.Enum,
  });
  const headType = makeTypeNodeData({
    tag: memberType,
    parentId: enumType.id,
  });
  return [enumType, headType];
}

export const STRING_TYPE_NODE = mapToTypeNode([makeTypeNodeData({ tag: TypeTag.String })]);
