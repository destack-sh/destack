import {
    check,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { account } from "../account/account.ts";

/** Domain records. */
export const domain = table("domain", {
    ...recordColumns("domain"),
    /** The account administering this hostname. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "restrict",
    }),
    /** The canonical lowercase DNS hostname. */
    hostname: text("hostname").notNull().unique(),
    /** Successful domain ownership verification time. */
    verifiedAt: integer("verified_at"),
}, (domain) => [
    check(
        "domain_hostname",
        sql`length(${domain.hostname}) BETWEEN 1 AND 253 AND ${domain.hostname} = lower(${domain.hostname})`,
    ),
]);

/** A persisted domain record. */
export type Domain = Select<typeof domain>;
