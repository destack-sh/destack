import { ACTIVE_TASK_STATUSES, TERMINAL_TASK_STATUSES } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { Transaction } from "@/language/runtime/transaction";
import { TaskData, NodeType, ObjectType, TaskStatus } from "@/proto/wire";

/** Create a Task. */
export function createTask(tx: Transaction, graph: ReadNodeGraph, options: { task: Partial<TaskData> }): TaskData {
  const siblings = graph.getChildren(options.task.parentPtr!, NodeType.TASK);
  const name = options.task.name ?? generateNodeName({ metatype: ObjectType.TASK, ...options.task }, siblings);
  const task = tx.create({ metatype: NodeType.TASK, ...options.task, name });
  return task;
}

/** Whether the given task is active */
export function isTaskActive(task: TaskData): boolean {
  return ACTIVE_TASK_STATUSES.includes(task.status);
}

/** Whether the given task is terminal */
export function isTaskTerminal(task: TaskData): boolean {
  return TERMINAL_TASK_STATUSES.includes(task.status);
}

/** 'Toggle' the Task status (whatever that means in its current state). */
export function toggleTaskStatus(tx: Transaction, task: TaskData): boolean {
  if (isTaskActive(task)) {
    return false;
  } else if (isTaskTerminal(task)) {
    tx.update(task, { status: task.ownedByPtr != null ? TaskStatus.CREATED : TaskStatus.ASSIGNED });
    return true;
  } else {
    tx.update(task, { status: TaskStatus.COMPLETED });
    return true;
  }
}
