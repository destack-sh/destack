import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { Transaction } from "@/language/runtime/transaction";
import { ImplementationData, NodeType, ObjectType } from "@/proto/wire";

/** Create an Implementation. */
export function createImplementation(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    implementation: Partial<ImplementationData>;
  },
): ImplementationData {
  const siblings = graph.getChildren(options.implementation.parentPtr!, NodeType.IMPLEMENTATION);
  const name =
    options.implementation.name ??
    generateNodeName({ metatype: ObjectType.IMPLEMENTATION, ...options.implementation }, siblings);
  const implementation = tx.create({
    metatype: NodeType.IMPLEMENTATION,
    ...options.implementation,
    name,
  });
  return implementation;
}
