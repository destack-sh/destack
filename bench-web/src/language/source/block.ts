import { type ReadNodeGraph } from "@/language/core/graph";
import { NodeIn } from "@/language/core/node";
import { getOrderKey } from "@/language/core/order";
import { newChangeId, type Transaction } from "@/language/runtime/transaction";
import { BlockData, FieldType, NodeReferenceData, NodeType, PackageData, PageData, TypeData } from "@/proto/wire";
import { describeNode, isNode, toNodeRef } from "@/proto/wiring";
import { generateOrderKey } from "@/utils/fractional";

/** Create a Block. */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    block: Partial<NodeIn<NodeType.BLOCK>> & Required<Pick<NodeIn<NodeType.BLOCK>, "type">>;
    anchor: "before" | "after" | "inside";
    target: BlockData | PackageData | PageData;
  },
): BlockData {
  const { target, anchor } = options;
  const packagePtr = isNode(target, NodeType.PACKAGE) ? toNodeRef(target) : target.packagePtr;

  // position
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let siblings: BlockData[];
  if (anchor == "inside") {
    parentPtr = toNodeRef(target);
    siblings = graph.getChildren(target, NodeType.BLOCK);
    orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
  } else {
    if (isNode(target, NodeType.PACKAGE)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
    parentPtr = target.parentPtr!;
    siblings = graph.getChildren(target.parentPtr!, NodeType.BLOCK);
    orderKey = getOrderKey({ position: anchor, reference: target, nodes: siblings });
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
  });

  return block;
}

/** Get the Type for a Block. Unwraps to the type for the inner node (if any). */
export function blockToType(block: BlockData, of: "instance" | "value", fieldTypes?: FieldType[]): TypeData {
  throw new Error("nocheckin: redirect to other toTypes?");
}
