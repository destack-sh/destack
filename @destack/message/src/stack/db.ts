import type { Table } from "@destack/db";
import { endpoint, mail, message, messageAttempt } from "../object/index.ts";

/** The message tables: messages with their attempts, endpoints and mail. */
export const messageTables: readonly Table[] = [
    ...message.tables,
    ...messageAttempt.tables,
    ...endpoint.tables,
    ...mail.tables,
];
