import { supergraph } from "@/globals";
import { isSubjectNode } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { newChangeId, Transaction } from "@/language/core/transaction";
import { JoinableNodeData, MembershipData, MessageType, NodeType } from "@/proto/wire";
import { describeNode, isNode, toNodeRef } from "@/proto/wiring";

/** Create a Membership */
export function createMembership(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: { parent: JoinableNodeData; membership: Partial<MembershipData> },
): MembershipData {
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }

  const member = supergraph.get(options.membership.memberPtr!);
  if (!isSubjectNode(member)) {
    throw new Error(`non-subject node: ${describeNode(member ?? options.membership.memberPtr!)}`);
  }

  const membership = tx.create({
    metatype: NodeType.MEMBERSHIP,
    ...options.membership,
    parentPtr: toNodeRef(options.parent),
    memberPtr: toNodeRef(member),
  });

  // auto create join messages :BadAutoMessages
  //  (unfortunately, these have to be 'created' by the current User, not the system)
  if (isNode(options.parent, NodeType.THREAD)) {
    const message = tx.create({
      metatype: NodeType.MESSAGE,
      parentPtr: toNodeRef(options.parent),
      packagePtr: options.parent.packagePtr,
      type: MessageType.JOIN,
      threadPtr: toNodeRef(options.parent),
      nodesPtr: [toNodeRef(member)],
    });
  }

  return membership;
}
