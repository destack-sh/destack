import { ReadNodeGraph } from "@/language/core/graph";
import { makeType } from "@/language/core/type";
import { Transaction } from "@/language/runtime/transaction";
import { NodeType, AgentData, TypeKind, BenchType, FieldType, TypeData } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";

/** Create a Agent. */
export function createAgent(tx: Transaction, graph: ReadNodeGraph, options: { agent: Partial<AgentData> }): AgentData {
  const siblings = graph.getChildren(options.agent.parentPtr!, NodeType.AGENT);
  const agent = tx.create({ metatype: NodeType.AGENT, ...options.agent });
  return agent;
}

/** Get the Type of a Agent. */
export function agentToType(
  agent: AgentData,
  of: "instance" | "value" = "instance",
  fieldTypes?: FieldType[],
): TypeData {
  if (of == "instance") {
    return makeType({ kind: TypeKind.BASED_NODE, benchType: BenchType.RUN, baseTypePtr: toNodeRef(agent) });
  } else {
    fieldTypes = fieldTypes ?? [];
    return makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: toNodeRef(agent),
      baseFieldTypes: fieldTypes,
      propertyFieldTypes: fieldTypes,
    });
  }
}
