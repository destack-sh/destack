import type { Table } from "@destack/db";
import { endpoint, message, messageAttempt } from "../object/index.ts";

/** The message kind's tables: messages with their attempts, and endpoints of scope histories. */
export const messageTables: readonly Table[] = [
    ...message.tables,
    ...messageAttempt.tables,
    ...endpoint.tables,
];
