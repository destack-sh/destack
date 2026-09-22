import type { AccessExpression } from "./expression.ts";
import type { PermissionReference } from "./object.ts";
import type { AccessContext, Subject } from "./subject.ts";
import { sameSubject } from "./subject.ts";
import { AccessError } from "../error/index.ts";
import type { PackageId } from "@destack/package/package";

/** A mandatory condition or prohibition applied after the object's permission rule. */
export interface AccessPolicy {
    /** Stable policy name shown by inspection. */
    readonly name: string;
    /** The package that declares the governed object type. */
    readonly packageId: PackageId;
    /** The declaration-local object type name. */
    readonly type: string;
    /** The permissions governed by this policy. */
    readonly permissions: readonly string[];
    /** Restrict one scope, or apply to every scope for this object type. */
    readonly scope?: string;
    /** Restrictions must match; prohibitions must not match. */
    readonly effect: "restrict" | "forbid";
    /** Typed condition evaluated over the target object and trusted context. */
    readonly condition: AccessExpression;
}

/** A permission restricted to one scope and optionally one object. */
export interface PermissionSelection extends PermissionReference {
    /** The authority scope in which this permission applies. */
    readonly scope: string;
    /** An optional stable application record identity. */
    readonly objectId?: string;
}

/** Current delegation state loaded and verified by the authenticating authority. */
export interface Delegation {
    /** The stable delegation identity. */
    readonly id: string;
    /** The represented identity granting authority. */
    readonly subject: Subject;
    /** The software identity receiving authority. */
    readonly actor: Subject;
    /** The complete permission restriction for this step. */
    readonly permissions: readonly PermissionSelection[];
    /** The creation time in Unix milliseconds. */
    readonly createdAt: number;
    /** The exclusive expiry time in Unix milliseconds. */
    readonly expiresAt: number;
    /** The revocation time in Unix milliseconds, or null while active. */
    readonly revokedAt: number | null;
}

/** Match a mandatory policy to one target permission. */
export function applies(
    policy: AccessPolicy,
    permission: PermissionReference,
    scope: string,
): boolean {
    return (
        policy.packageId === permission.packageId &&
        policy.type === permission.type &&
        policy.permissions.includes(permission.name) &&
        (policy.scope === undefined || policy.scope === scope)
    );
}

/** Match credential restrictions before checking exact objects or compiling filtered queries. */
export function permitsCredential(
    permission: PermissionReference,
    object: { scope: string; id?: string },
    context: AccessContext,
): boolean {
    return (
        context.permissions === undefined ||
        context.permissions.some(
            (entry) =>
                entry.packageId === permission.packageId &&
                entry.type === permission.type &&
                entry.name === permission.name &&
                entry.scope === object.scope &&
                (entry.objectId === undefined ||
                    object.id === undefined ||
                    entry.objectId === object.id),
        )
    );
}

/** Verify each delegation step and require every step to include the requested operation. */
export function permitsDelegation(
    permission: PermissionReference,
    object: { scope: string; id?: string },
    context: AccessContext,
): boolean {
    // allow direct calls only when no delegation chain was supplied
    if (!context.actor) {
        if (context.delegations?.length) {
            throw new AccessError("INVALID_CONTEXT", "delegations require an authenticated actor");
        }

        return true;
    }

    // require the represented identity among the authenticated subjects
    if (
        !context.subject ||
        !context.subjects.some((subject) => sameSubject(subject, context.subject!))
    ) {
        throw new AccessError(
            "INVALID_CONTEXT",
            "delegation requires an authenticated represented subject",
        );
    }

    // every step restricts the previous authority; credentials cannot skip a revoked step
    const chain = context.delegations ?? [];
    if (chain.length === 0) {
        return false;
    }

    // follow each actor transition once and intersect its permission selection
    let previous = context.subject;
    const ids = new Set<string>();
    for (const delegation of chain) {
        if (ids.has(delegation.id) || !sameSubject(previous, delegation.subject)) {
            throw new AccessError("INVALID_CONTEXT", "delegation chain is inconsistent");
        }
        ids.add(delegation.id);

        // reject revoked, expired or not yet active delegation records
        if (
            !Number.isFinite(delegation.createdAt) ||
            !Number.isFinite(delegation.expiresAt) ||
            delegation.revokedAt !== null ||
            delegation.createdAt > context.now ||
            context.now >= delegation.expiresAt
        ) {
            return false;
        }

        // require this step to include the requested permission and object
        const selected = delegation.permissions.some(
            (entry) =>
                entry.packageId === permission.packageId &&
                entry.type === permission.type &&
                entry.name === permission.name &&
                entry.scope === object.scope &&
                (object.id === undefined ||
                    entry.objectId === undefined ||
                    entry.objectId === object.id),
        );
        if (!selected) {
            return false;
        }

        previous = delegation.actor;
    }

    return sameSubject(previous, context.actor);
}
