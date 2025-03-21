import { ReadNodeGraph } from "@/language/core/graph";
import { Transaction } from "@/language/runtime/transaction";
import { MembershipData, NodeType } from "@/proto/wire";

/** Create a Membership */
export function createMembership(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: { membership: Partial<MembershipData> },
): MembershipData {
  const siblings = graph.getChildren(options.membership.parentPtr!, NodeType.MEMBERSHIP);
  const membership = tx.create({ metatype: NodeType.MEMBERSHIP, ...options.membership });
  return membership;
}
