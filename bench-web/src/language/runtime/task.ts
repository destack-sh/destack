import { ReadNodeGraph } from "@/language/core/graph";
import { generateNodeName } from "@/language/core/node";
import { Transaction } from "@/language/runtime/transaction";
import { TaskData, NodeType, ObjectType } from "@/proto/wire";

/** Create a Task. */
export function createTask(tx: Transaction, graph: ReadNodeGraph, options: { task: Partial<TaskData> }): TaskData {
  const siblings = graph.getChildren(options.task.parentPtr!, NodeType.TASK);
  const name = options.task.name ?? generateNodeName({ metatype: ObjectType.TASK, ...options.task }, siblings);
  const task = tx.create({ metatype: NodeType.TASK, ...options.task, name });
  return task;
}
