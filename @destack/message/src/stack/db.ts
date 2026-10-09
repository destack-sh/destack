import type { Table } from "@destack/db";
import { endpoint, message, messageAttempt } from "../object/index.ts";

/** The message kind's tables: its messages with their attempts, and the endpoints subscribing scopes' histories. */
export const messageTables: readonly Table[] = [
    ...message.tables,
    ...messageAttempt.tables,
    ...endpoint.tables,
];
