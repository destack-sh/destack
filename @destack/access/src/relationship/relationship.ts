import type { ObjectReference } from "../access/index.ts";
import type { Subject } from "../access/subject.ts";

/** An explicit, expiring relationship between an object and a subject. */
export interface Grant {
    /** The stable grant identifier. */
    readonly id: string;
    /** The protected object, qualified by package and scope. */
    readonly object: ObjectReference;
    /** The declared relationship granted to the subject. */
    readonly relation: string;
    /** The subject receiving access, or everyone for public access. */
    readonly subject: Subject | { readonly kind: "everyone" };
    /** The creation time in Unix milliseconds. */
    readonly createdAt: number;
    /** The exclusive expiry time in Unix milliseconds, or null for no expiry. */
    readonly expiresAt: number | null;
    /** The revocation time in Unix milliseconds, or null while unrevoked. */
    readonly revokedAt: number | null;
}

/** Determine whether a persisted grant remains active at a trusted request time. */
export function isActive(
    grant: Pick<Grant, "createdAt" | "revokedAt" | "expiresAt">,
    now: number,
): boolean {
    return (
        grant.createdAt <= now &&
        grant.revokedAt === null &&
        (grant.expiresAt === null || now < grant.expiresAt)
    );
}
