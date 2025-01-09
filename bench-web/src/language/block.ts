import { makeTypeInfo } from "@/language/field";
import { type ReadNodeGraph } from "@/language/graph";
import { makeNodeName, NodeIn } from "@/language/node";
import { getOrderKey } from "@/language/order";
import type { Transaction } from "@/language/transaction";
import {
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

/** Create a Block relative to another. */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    block: Partial<NodeIn<NodeType.BLOCK>> & Required<Pick<NodeIn<NodeType.BLOCK>, "type">>;
    anchor: "before" | "after" | "inside";
    target: BlockData | TypedNodeReferenceData<NodeType.BLOCK> | PackageData | TypedNodeReferenceData<NodeType.PACKAGE>;
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
    (options.block.subnode as VariableBlockData).valueType = makeTypeInfo({
      kind: TypeKind.STRUCT,
      benchType: BenchType.TEXT,
    });
  }

  // create
  const block = tx.create({
    metatype: NodeType.BLOCK,
    parentPtr,
    packagePtr,
    ...options.block,
    orderKey,
    name: makeNodeName(graph, { metatype: ObjectType.BLOCK, type: options.block.type, parentPtr }),
  });
  return block;
}

/** Gets the (primary) type represented by the Block. */
export function blockToType(block: BlockData): TypeData {
  // NOTE: technically there is more than one possible mapping from node to type identity
  //  (for instance Signal blocks could map to both Signal nodes based in that block or Values of that Signal type)
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
  const type = makeTypeInfo({ kind, benchType, baseFieldTypes });
  type.baseTypePtr = toNodeRef(block);
  return type;
}
