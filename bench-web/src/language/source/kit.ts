import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { Transaction } from "@/language/runtime/transaction";
import { KitData, NodeType, ObjectType } from "@/proto/wire";

/** Create an Kit. */
export function createKit(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    kit: Partial<KitData>;
  },
): KitData {
  const siblings = graph.getChildren(options.kit.parentPtr!, NodeType.KIT);
  const name =
    options.kit.name ?? generateNodeName({ metatype: ObjectType.KIT, ...options.kit }, siblings);
  const kit = tx.create({
    metatype: NodeType.KIT,
    ...options.kit,
    name,
  });
  return kit;
}
