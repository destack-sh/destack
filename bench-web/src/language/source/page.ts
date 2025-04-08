import { ReadNodeGraph } from "@/language/core/graph";
import { getOrderKey } from "@/language/core/order";
import { emptyTextLine } from "@/language/core/text";
import { newChangeId, Transaction } from "@/language/core/transaction";
import { BlockType, NodeReferenceData, NodeType, PackageData, PageData } from "@/proto/wire";
import { describeNode, isNode, toNodeRef } from "@/proto/wiring";
import { generateOrderKey } from "@/utils/fractional";

/** Create a Page. */
export function createPage(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    page: Partial<PageData>;
    anchor: "before" | "after" | "inside";
    target: PackageData | PageData;
  },
): PageData {
  const { target, anchor } = options;
  const packagePtr = isNode(target, NodeType.PACKAGE) ? toNodeRef(target) : target.packagePtr;
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }

  // position
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let siblings: PageData[];
  if (anchor == "inside") {
    parentPtr = toNodeRef(target);
    siblings = graph.getChildren(target, NodeType.PAGE);
    orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
  } else {
    if (isNode(target, NodeType.PACKAGE)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
    parentPtr = target.parentPtr!;
    siblings = graph.getChildren(target.parentPtr!, NodeType.PAGE);
    orderKey = getOrderKey({ position: anchor, reference: target, nodes: siblings });
  }

  // create
  if (isNode(target, NodeType.PAGE) && options.page.definitionPtr == null) {
    throw new Error("cannot create inline Page without block");
  }
  const page = tx.create({
    metatype: NodeType.PAGE,
    parentPtr,
    packagePtr,
    ...options.page,
    orderKey,
  });

  // create default contents
  const text = tx.create({
    metatype: NodeType.BLOCK,
    parentPtr: toNodeRef(page),
    packagePtr,
    type: BlockType.PARAGRAPH,
    orderKey: generateOrderKey(null, null),
    line: emptyTextLine(),
  });

  return page;
}
