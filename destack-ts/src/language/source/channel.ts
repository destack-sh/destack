import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { getOrderKey } from "@/language/core/order";
import { newChangeId, Transaction } from "@/language/core/transaction";
import { ChannelData, NodeReferenceData, NodeType, ObjectType, PackageData, PageData } from "@/proto/wire";
import { describeNode, isNode, toNodeRef } from "@/proto/wiring";
import { generateOrderKey } from "@/utils/fractional";

/** Create a Channel. */
export function createChannel(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    channel: Partial<ChannelData>;
    anchor: "before" | "after" | "inside";
    target: PackageData | PageData;
  },
): ChannelData {
  const { target, anchor } = options;
  const packagePtr = isNode(target, NodeType.PACKAGE) ? toNodeRef(target) : target.packagePtr;

  // position
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let siblings: ChannelData[];
  if (anchor == "inside") {
    parentPtr = toNodeRef(target);
    siblings = graph.getChildren(target, NodeType.CHANNEL);
    orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
  } else {
    if (isNode(target, NodeType.PACKAGE)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
    parentPtr = target.parentPtr!;
    siblings = graph.getChildren(target.parentPtr!, NodeType.CHANNEL);
    orderKey = getOrderKey({ position: anchor, reference: target, nodes: siblings });
  }

  // create
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const name = options.channel.name ?? generateNodeName({ metatype: ObjectType.CHANNEL, ...options.channel }, siblings);
  const channel = tx.create({
    metatype: NodeType.CHANNEL,
    parentPtr,
    packagePtr,
    ...options.channel,
    orderKey,
    name,
  });

  return channel;
}
