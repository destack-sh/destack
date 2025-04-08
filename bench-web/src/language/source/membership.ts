import { supergraph } from "@/globals";
import { isNodeInstance, isSubjectNode } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { instanceNode } from "@/language/core/node";
import { newChangeId, Transaction } from "@/language/core/transaction";
import { JoinableNodeData, MembershipData, NodeType } from "@/proto/wire";
import { describeNode, isNode } from "@/proto/wiring";

/** Create a Membership */
export function createMembership(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: { parent: JoinableNodeData; membership: Partial<MembershipData> },
): MembershipData {
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }

  let member = supergraph.get(options.membership.memberPtr!);
  if (!isSubjectNode(member)) {
    throw new Error(`non-subject node: ${describeNode(member ?? options.membership.memberPtr!)}`);
  }

  // instance agents if not already instanced
  if (isNode(member, NodeType.AGENT) && !isNodeInstance(member)) {
    member = instanceNode(tx, graph, member, { parent: options.parent });
  }

  const siblings = graph.getChildren(options.membership.parentPtr!, NodeType.MEMBERSHIP);
  const membership = tx.create({ metatype: NodeType.MEMBERSHIP, ...options.membership });
  return membership;
}
