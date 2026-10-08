import { check, defineTable, index, integer, sql, text, uniqueIndex } from "@destack/db";
import { defineSchema, Instant, schema } from "@destack/schema";

/** How long a reservation waits for its write to commit, in milliseconds: a minute, above any write. */
export const RESERVATION_MILLISECONDS = 60_000;

/** The states of a claim: reserved by a write that may still fail, resolving to its object, or holding the name for an object in the trash. */
const CLAIM_STATES = ["pending", "active", "held"] as const;

/** The state an object's committed names take: active, or held while the object sits in the trash. */
export const ObjectClaimState = defineSchema(schema.enum(["active", "held"]));
/** The state an object's committed names take. */
export type ObjectClaimState = schema.Infer<typeof ObjectClaimState>;

/** A name an object claims in a unique index across databases. */
export const Claim = defineSchema(
    schema.object({
        /** The index: the object type's package, type and index name, or a namespace types share. */
        index: schema.string().min(1),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: schema.string(),
        /** The package of the claiming object's type. */
        packageId: schema.string().min(1),
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
        /** The state its names take: active, or held while the object sits in the trash. */
        state: ObjectClaimState,
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
        next: Instant.exactOptional(),
    }),
);
/** The expired reservations of some indexes and the next deadline. */
export type Expiry = schema.Infer<typeof Expiry>;

/** Every claimed name: pending during its write, held while its object sits in the trash, active otherwise. */
export const claimTable = defineTable(
    "claim",
    {
        /** The claim's identifier. */
        id: text("id").notNull(),
        /** The index: the object type's package, type and index name, or a namespace types share. */
        index: text("index").primaryKey(),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: text("key").primaryKey(),
        /** The package of the claiming object's type. */
        packageId: text("package_id").notNull(),
        /** The object claiming the name. */
        objectId: text("object_id").notNull(),
        /** The scope the object lives in. */
        scope: text("scope").notNull(),
        /** Whether the claim is pending, active or held. */
        state: text("state", { enum: CLAIM_STATES }).notNull(),
        /** The request whose write claimed the name. */
        requestId: text("request_id").notNull(),
        /** The deadline of a pending claim's write, in UTC epoch milliseconds, null once the write committed. */
        expiresAt: integer("expires_at"),
    },
    {
        constraints: (claim) => [
            uniqueIndex("claim_id").on(claim.id),
            index("claim_request").on(claim.requestId),
            index("claim_object").on(claim.index, claim.objectId),
            index("claim_expiry").on(claim.expiresAt),
            check(
                "claim_expiry",
                sql`(${claim.state} = 'pending') = (${claim.expiresAt} IS NOT NULL)`,
            ),
        ],
        log: {},
    },
);
