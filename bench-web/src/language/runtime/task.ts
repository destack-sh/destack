import { ACTIVE_PROCESS_STATUSES, TERMINAL_PROCESS_STATUSES } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { Transaction } from "@/language/core/transaction";
import { isProcessActive, isProcessTerminal } from "@/language/runtime/process";
import { NodeType, ProcessStatus, TaskData } from "@/proto/wire";

/** Create a Task. */
export function createTask(tx: Transaction, graph: ReadNodeGraph, options: { task: Partial<TaskData> }): TaskData {
  const siblings = graph.getChildren(options.task.parentPtr!, NodeType.TASK);
  const task = tx.create({ metatype: NodeType.TASK, ...options.task });
  return task;
}

/** 'Toggle' the Task status (whatever that means in its current state). */
export function toggleTaskStatus(tx: Transaction, task: TaskData) {
  if (isProcessActive(task) || isProcessTerminal(task)) {
    tx.update(task, { status: task.ownedByPtr != null ? ProcessStatus.CREATED : ProcessStatus.ASSIGNED });
  } else {
    tx.update(task, { status: ProcessStatus.COMPLETED });
  }
}
