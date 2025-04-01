import { ReadNodeGraph } from "@/language/core/graph";
import { Transaction } from "@/language/runtime/transaction";
import { NodeType, AgentData } from "@/proto/wire";

/** Create a Agent. */
export function createAgent(tx: Transaction, graph: ReadNodeGraph, options: { agent: Partial<AgentData> }): AgentData {
  const siblings = graph.getChildren(options.agent.parentPtr!, NodeType.AGENT);
  const agent = tx.create({ metatype: NodeType.AGENT, ...options.agent });
  return agent;
}
