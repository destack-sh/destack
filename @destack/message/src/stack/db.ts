import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { space } from "@destack/space/object";
import { message, messageAttempt } from "../object/index.ts";

/** The message service's database: its messages with their attempts and the journal, with copies of the spaces they live in. */
export const messageDatabase = defineDatabase({
    name: "message",
    tables: [...message.tables, ...messageAttempt.tables, journal],
    copies: [space.table],
});
