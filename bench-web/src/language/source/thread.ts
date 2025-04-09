import { supergraph } from "@/globals";
import { isSubjectNode } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { newChangeId, Transaction } from "@/language/core/transaction";
import { createMembership } from "@/language/source/membership";
import { NodeReferenceData, NodeType, SubjectNodeData, ThreadData, Timestamp } from "@/proto/wire";
import { describeNode, isNode, isNodeRef, toNodeRef } from "@/proto/wiring";
import { user } from "@/system/user";
import { assertNever } from "@/utils/functools";

/** Create a Thread. */
export function createThread(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    thread: Partial<ThreadData>;
    members: Array<"user" | NodeReferenceData | SubjectNodeData>;
  },
): ThreadData {
  // create
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const activeAt = options.thread.activeAt ?? Timestamp.now();
  const thread = tx.create({ metatype: NodeType.THREAD, ...options.thread, activeAt });

  // add members
  for (const member of options.members) {
    let memberNode: SubjectNodeData;
    if (member === "user") {
      if (user.value == null) {
        throw new Error("no current user");
      }
      memberNode = user.value;
    } else if (isNodeRef(member)) {
      const node = supergraph.get(member);
      if (!isSubjectNode(node)) {
        throw new Error(`non-subject node: ${describeNode(member)}`);
      }
      memberNode = node;
    } else if (isNode(member)) {
      memberNode = member;
    } else {
      assertNever(member);
    }

    // membership
    createMembership(tx, graph, {
      parent: thread,
      membership: {
        parentPtr: toNodeRef(thread),
        packagePtr: thread.packagePtr,
        memberPtr: toNodeRef(memberNode),
      },
    });
  }

  return thread;
}
