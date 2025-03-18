import { ReadNodeGraph } from "@/language/core/graph";
import { newChangeId, Transaction } from "@/language/runtime/transaction";
import { NodeType, ThreadData } from "@/proto/wire";

/** Create a Thread. */
export function createThread(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    thread: Partial<ThreadData>;
  },
): ThreadData {
  // create
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const thread = tx.create({ metatype: NodeType.THREAD, ...options.thread });
  return thread;
}
