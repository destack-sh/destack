import { type DatabaseConnection, defineTable, index, integer, lte, text } from "@destack/db";
import { AccessError } from "../error/index.ts";

/** The single-use proofs a store spent, each until it expires, after JWT's `jti`. */
export const nonce = defineTable(
    "nonce",
    {
        /** The proof's digest. */
        id: text("id").primaryKey(),
        /** When the proof expires and its digest may leave, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    { constraints: (used) => [index("nonce_expiry").on(used.expiresAt)] },
);

/** A single-use proof's digest, spent once until it expires. */
export class Nonce {
    /** The proof's digest. */
    readonly id: string;
    /** When the proof expires, in UTC epoch milliseconds. */
    readonly expiresAt: number;

    /** Describe a proof's digest and its expiry. */
    constructor(id: string, expiresAt: number) {
        this.id = id;
        this.expiresAt = expiresAt;
    }

    /** Spend the digest once, refusing one spent before that has not expired. */
    async spend(database: DatabaseConnection, now: number): Promise<void> {
        // forget the expired digests, then record this one unless it is spent
        await database.delete(nonce).where(lte(nonce.expiresAt, now));
        const [spent] = await database
            .insert(nonce)
            .values({ id: this.id, expiresAt: this.expiresAt })
            .onConflictDoNothing()
            .returning({ id: nonce.id });
        if (spent === undefined) {
            throw AccessError.of("ASSERTION_REUSED", "assertion was already used");
        }
    }
}
