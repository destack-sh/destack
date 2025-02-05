import { type ReadNodeGraph } from "@/language/graph";
import { makeNodeName, NodeIn } from "@/language/node";
import { getOrderKey } from "@/language/order";
import { newChangeId, type Transaction } from "@/language/transaction";
import {
  ActionType,
  BlockData,
  BlockType,
  FieldType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PackageData,
  TypeData
} from "@/proto/wire";
import { describeNode, isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { generateOrderKey } from "@/utils/fractional";

/** Create a Block (relative to another). */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    block: Partial<NodeIn<NodeType.BLOCK>> & Required<Pick<NodeIn<NodeType.BLOCK>, "type">>;
    anchor: "before" | "after" | "inside";
    target: BlockData | TypedNodeReferenceData<NodeType.BLOCK> | PackageData | TypedNodeReferenceData<NodeType.PACKAGE>;
    skipDefaultChildren?: boolean;
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

  // create default children
  if (!options.skipDefaultChildren) {
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
  throw new Error("nocheckin: redirect to other toTypes?");
}
