import { makeType } from "@/language/field";
import { type ReadNodeGraph } from "@/language/graph";
import { makeNodeName, NodeIn } from "@/language/node";
import { getOrderKey } from "@/language/order";
import { newChangeId, type Transaction } from "@/language/transaction";
import {
  ActionType,
  BenchType,
  BlockData,
  BlockType,
  FieldType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PackageData,
  TypeData,
  TypeKind,
  VariableBlockData,
} from "@/proto/wire";
import { describeNode, isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { generateOrderKey } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";

/** Create a Block (relative to another). */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    block: Partial<NodeIn<NodeType.BLOCK>> & Required<Pick<NodeIn<NodeType.BLOCK>, "type">>;
    anchor: "before" | "after" | "inside";
    target: BlockData | TypedNodeReferenceData<NodeType.BLOCK> | PackageData | TypedNodeReferenceData<NodeType.PACKAGE>;
    skipDefaultStuff?: boolean;
  },
): BlockData {
  const target = isNode(options.target) ? options.target : graph.getOrError(options.target);
  const packagePtr = isNode(target, NodeType.PACKAGE) ? toNodeRef(target) : target.packagePtr;

  // position in graph
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let siblings: BlockData[];
  if (options.anchor == "inside") {
    parentPtr = toNodeRef(target);
    siblings = graph.getChildren(target, NodeType.BLOCK);
    orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
  } else {
    if (isNode(target, NodeType.PACKAGE)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
    parentPtr = target.parentPtr!;
    siblings = graph.getChildren(target.parentPtr!, NodeType.BLOCK);
    orderKey = getOrderKey({ position: options.anchor, reference: target, nodes: siblings });
  }
  if (options.block.subnode == null) options.block.subnode = {};

  // add value type if not given
  if (options.block.type == BlockType.VARIABLE && (options.block.subnode as VariableBlockData).valueType == null) {
    (options.block.subnode as VariableBlockData).valueType = makeType({
      kind: TypeKind.STRUCT,
      benchType: BenchType.TEXT,
    });
  }

  // create
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const block = tx.create({
    metatype: NodeType.BLOCK,
    parentPtr,
    packagePtr,
    ...options.block,
    orderKey,
    name: makeNodeName(graph, { metatype: ObjectType.BLOCK, type: options.block.type, parentPtr }),
  });

  // create default stuff
  if (!options.skipDefaultStuff) {
    if (block.type == BlockType.FLOW) {
      // create start action
      tx.create({
        metatype: NodeType.ACTION,
        type: ActionType.START,
        parentPtr: toNodeRef(block),
        name: makeNodeName(graph, { metatype: ObjectType.ACTION, type: ActionType.START, parentPtr: toNodeRef(block) }),
        packagePtr,
      });
    }
  }

  return block;
}

/** Get the Type for a Block. */
export function blockToType(block: BlockData, of: "instance" | "value", fieldTypes?: FieldType[]): TypeData {
  if (of == "instance") {
    let kind: TypeKind;
    let benchType: BenchType | undefined;
    let baseFieldTypes: FieldType[] | undefined;
    if (block.type == BlockType.CHOICE) {
      benchType = BenchType.FIELD;
      kind = TypeKind.BASED_NODE;
      baseFieldTypes = [FieldType.OPTION];
    } else if (block.type == BlockType.MESSAGE) {
      benchType = BenchType.MESSAGE;
      kind = TypeKind.BASED_NODE;
      baseFieldTypes = [FieldType.MEMBER];
    } else if (block.type == BlockType.DATABASE) {
      benchType = BenchType.RECORD;
      kind = TypeKind.BASED_NODE;
      baseFieldTypes = [FieldType.MEMBER];
    } else {
      throw new Error(`unsupported block type: ${block.type}`);
    }
    const type = makeType({ kind, benchType, baseFieldTypes });
    type.baseTypePtr = toNodeRef(block);
    return type;
  } else if (of == "value") {
    fieldTypes = fieldTypes ?? [FieldType.MEMBER];
    return makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: toNodeRef(block),
      baseFieldTypes: fieldTypes,
      propertyFieldTypes: fieldTypes,
    });
  } else {
    assertNever(of);
  }
}
