import { BENCH_BUILTIN_IDENTITY_PTR } from "@/language/core/builtin";
import { ReadNodeGraph } from "@/language/core/graph";
import { newChangeId, Transaction } from "@/language/runtime/transaction";
import { createMembership } from "@/language/source/membership";
import { NodeType, SubjectNodeData, ThreadData } from "@/proto/wire";
import { isNode, toNodeRef } from "@/proto/wiring";
import { user } from "@/system/user";

/** Create a Thread. */
export function createThread(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    thread: Partial<ThreadData>;
    members: Array<"bench" | "user" | SubjectNodeData>;
  },
): ThreadData {
  // create
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const thread = tx.create({ metatype: NodeType.THREAD, ...options.thread });

  // add members
  for (const member of options.members) {
    if (member == "bench") {
      const membership = createMembership(tx, graph, {
        membership: {
          parentPtr: toNodeRef(thread),
          packagePtr: thread.packagePtr,
          memberPtr: BENCH_BUILTIN_IDENTITY_PTR,
        },
      });
    } else if (member == "user") {
      if (user.value == null) throw new Error("no user");
      const membership = createMembership(tx, graph, {
        membership: {
          parentPtr: toNodeRef(thread),
          packagePtr: thread.packagePtr,
          memberPtr: toNodeRef(user.value),
        },
      });
    } else if (isNode(member)) {
      const membership = createMembership(tx, graph, {
        membership: {
          parentPtr: toNodeRef(thread),
          packagePtr: thread.packagePtr,
          memberPtr: toNodeRef(member),
        },
      });
    }
  }

  return thread;
}
