import type { Table } from "@destack/db";
import { endpoint, message, messageAttempt, task } from "../object/index.ts";

/** The message tables: messages with their attempts, endpoints and tasks. */
export const messageTables: readonly Table[] = [
    ...message.tables,
    ...messageAttempt.tables,
    ...endpoint.tables,
    ...Object.values(task.objects).flatMap((object) => object.tables),
];
