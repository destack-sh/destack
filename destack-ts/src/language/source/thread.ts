import { supergraph } from "@/globals";
import { DESTACK_DESTACK_AGENT_PTR } from "@/language/core/builtin";
import { isSubjectNode } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { newChangeId, Transaction } from "@/language/core/transaction";
import { createMembership } from "@/language/source/membership";
import { NodeReferenceData, NodeType, SubjectNodeData, ThreadData, Timestamp } from "@/proto/wire";
import { describeNode, isNode, isNodeRef, toNodeRef } from "@/proto/wiring";
import { pkg } from "@/system/space";
import { user } from "@/system/user";
import { assertNever } from "@/utils/functools";
import ProcessStatus from "@/views/internal/ProcessStatus.vue";

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
  const now = Timestamp.now();
  const startedAt = options.thread.startedAt ?? now;
  const activeAt = options.thread.activeAt ?? now;
  const status = options.thread.status ?? ProcessStatus.IDLE;
  const thread = tx.create({ metatype: NodeType.THREAD, ...options.thread, startedAt, activeAt, status });

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

/** Create the default Thread for this Space */
export function createDefaultThread(
  tx: Transaction,
  graph: ReadNodeGraph,
  options?: {
    thread?: Partial<ThreadData>;
  },
): ThreadData {
  if (pkg.value == null) {
    throw new Error("no package");
  }
  const thread = createThread(tx, graph, {
    thread: {
      parentPtr: toNodeRef(pkg.value!),
      packagePtr: toNodeRef(pkg.value!),
      ...options?.thread,
    },
    members: [DESTACK_DESTACK_AGENT_PTR, "user"],
  });
  return thread;
}
