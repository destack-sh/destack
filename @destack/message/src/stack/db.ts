import type { Table } from "@destack/db";
import { endpoint, mail, message, messageAttempt, task } from "../object/index.ts";

/** The message tables: messages with their attempts, endpoints, mail and tasks. */
export const messageTables: readonly Table[] = [
    ...message.tables,
    ...messageAttempt.tables,
    ...endpoint.tables,
    ...mail.tables,
    ...Object.values(task.objects).flatMap((object) => object.tables),
];
