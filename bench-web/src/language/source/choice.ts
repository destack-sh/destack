import { BenchType, NodeType, ObjectType, TypeData, TypeKind } from "@/proto/wire";
import { makeType } from "@/language/core/type";
import { ChoiceData } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { Transaction } from "@/language/runtime/transaction";

/** Create a Choice. */
export function createChoice(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: { choice: Partial<ChoiceData> },
): ChoiceData {
  const siblings = graph.getChildren(options.choice.parentPtr!, NodeType.CHOICE);
  const name = options.choice.name ?? generateNodeName({ metatype: ObjectType.CHOICE, ...options.choice }, siblings);
  const choice = tx.create({ metatype: NodeType.CHOICE, ...options.choice, name });
  return choice;
}

/** Get the Type of a Choice. */
export function choiceToType(choice: ChoiceData): TypeData {
  return makeType({
    kind: TypeKind.BASED_NODE,
    benchType: BenchType.FIELD,
    baseTypePtr: toNodeRef(choice),
  });
}
