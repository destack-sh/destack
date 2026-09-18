import { identifier, index, recordColumns, type Select, table, text, unique } from "@destack/db";
import { user } from "./user.ts";

/** Identity records. */
export const identity = table("identity", {
    ...recordColumns("identity"),
    /** The authenticated Destack user. */
    userId: identifier("user_id", "user").notNull().references(() => user.id, {
        onDelete: "cascade",
    }),
    /** The identity provider issuer. */
    issuer: text("issuer").notNull(),
    /** The stable subject assigned by the issuer. */
    subject: text("subject").notNull(),
}, (identity) => [
    unique("identity_issuer_subject").on(identity.issuer, identity.subject),
    index("identity_user").on(identity.userId),
]);

/** A persisted identity record. */
export type Identity = Select<typeof identity>;
