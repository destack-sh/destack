import { defineTable, index, integer, primaryKey, text } from "@destack/db";
import { defineSchema, Instant, schema } from "@destack/schema";

/** How long a reservation waits for its write to commit, in milliseconds: a minute, above any write. */
export const RESERVATION_MILLISECONDS = 60_000;

/** A name an object claims in a unique index across databases. */
export const Claim = defineSchema(
    schema.object({
        /** The index: the object type's package, type and index name. */
        index: schema.string().min(1),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: schema.string(),
        /** The object claiming the name. */
        objectId: schema.string().min(1),
        /** The scope the object lives in. */
        scope: schema.string().min(1),
    }),
);
/** A name an object claims in a unique index across databases. */
export type Claim = schema.Infer<typeof Claim>;

/** The claims one object holds in its type's indexes after a write. */
export const ObjectClaims = defineSchema(
    schema.object({
        /** The indexes the object's type declares. */
        indexes: schema.array(schema.string().min(1)),
        /** The object. */
        objectId: schema.string().min(1),
        /** The names it claims. */
        claims: schema.array(Claim),
    }),
);
/** The claims one object holds in its type's indexes after a write. */
export type ObjectClaims = schema.Infer<typeof ObjectClaims>;

/** The expired reservations of some indexes and the next deadline. */
export const Expiry = defineSchema(
    schema.object({
        /** The expired reservations. */
        claims: schema.array(Claim),
        /** When the next reservation expires, in UTC epoch milliseconds. */
        next: Instant.optional(),
    }),
);
/** The expired reservations of some indexes and the next deadline. */
export type Expiry = schema.Infer<typeof Expiry>;

/** Every claimed name, reserved during its write and confirmed after commit. */
export const claimTable = defineTable(
    "claim",
    {
        /** The index: the object type's package, type and index name. */
        index: text("index").notNull(),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: text("key").notNull(),
        /** The object claiming the name. */
        objectId: text("object_id").notNull(),
        /** The scope the object lives in. */
        scope: text("scope").notNull(),
        /** Whether the object's write committed or may still fail. */
        state: text("state", { enum: ["reserved", "confirmed"] }).notNull(),
        /** The request whose write reserved the claim. */
        requestId: text("request_id").notNull(),
        /** The reservation deadline, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    {
        tier: "global",
        constraints: (claim) => [
            primaryKey({ columns: [claim.index, claim.key] }),
            index("claim_request").on(claim.requestId),
            index("claim_object").on(claim.index, claim.objectId),
            index("claim_expiry").on(claim.state, claim.expiresAt),
        ],
        log: {},
    },
);
