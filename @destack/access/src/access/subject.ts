import { defineSchema, schema } from "@destack/schema";
import type { Delegation, PermissionSelection } from "./policy.ts";

/** An authenticated identity or a verified group or bearer-token identity. */
export const Subject = defineSchema(
    schema.object({
        /** The authenticated identity category. */
        kind: schema.enum(["user", "service-account", "group", "share-token", "host"]),
        /** The authority responsible for the identity. */
        authority: schema.string().min(1),
        /** The immutable identifier assigned by that authority. */
        id: schema.string().min(1),
    }),
);
/** An authenticated identity or a verified group or bearer-token identity. */
export type Subject = schema.Infer<typeof Subject>;

/** A scalar attribute accepted by both memory and SQL evaluation. */
export const Attribute = schema.union([
    schema.string(),
    schema.number().finite(),
    schema.boolean(),
]);
/** A scalar attribute accepted by both memory and SQL evaluation. */
export type Attribute = schema.Infer<typeof Attribute>;

/** Verified identities and attributes supplied by the authoritative caller. */
export interface AccessContext {
    /** Credential permissions intersected with all current object and role grants. */
    readonly permissions?: readonly PermissionSelection[];
    /** Authenticated represented identity, required for delegated calls. */
    readonly subject?: Subject;
    /** Authenticated software actor, independently constrained by its own permissions. */
    readonly actor?: Subject;
    /** Current delegation records, verified by the host in its authorization read view. */
    readonly delegations?: readonly Delegation[];
    /** Current subject and verified group memberships; empty means anonymous. */
    readonly subjects: readonly Subject[];
    /** Trusted request time in UTC epoch milliseconds. */
    readonly now: number;
    /** Trusted request attributes referenced by declarations. */
    readonly attributes: Readonly<Record<string, Attribute>>;
}

/** Compare the complete authority-qualified identity. */
export function sameSubject(left: Subject, right: Subject): boolean {
    return left.kind === right.kind && left.authority === right.authority && left.id === right.id;
}
