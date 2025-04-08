import { ACTIVE_PROCESS_STATUSES, TERMINAL_PROCESS_STATUSES } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { Transaction } from "@/language/core/transaction";
import { NodeType, ProcessStatus, TaskData } from "@/proto/wire";

/** Create a Task. */
export function createTask(tx: Transaction, graph: ReadNodeGraph, options: { task: Partial<TaskData> }): TaskData {
  const siblings = graph.getChildren(options.task.parentPtr!, NodeType.TASK);
  const task = tx.create({ metatype: NodeType.TASK, ...options.task });
  return task;
}

/** Whether the given task is active */
export function isTaskActive(task: TaskData): boolean {
  return ACTIVE_PROCESS_STATUSES.includes(task.status);
}

/** Whether the given task is terminal */
export function isTaskTerminal(task: TaskData): boolean {
  return TERMINAL_PROCESS_STATUSES.includes(task.status);
}

/** 'Toggle' the Task status (whatever that means in its current state). */
export function toggleTaskStatus(tx: Transaction, task: TaskData): boolean {
  if (isTaskActive(task)) {
    return false;
  } else if (isTaskTerminal(task)) {
    tx.update(task, { status: task.ownedByPtr != null ? ProcessStatus.CREATED : ProcessStatus.ASSIGNED });
    return true;
  } else {
    tx.update(task, { status: ProcessStatus.COMPLETED });
    return true;
  }
}
