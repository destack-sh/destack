import { sql, type SQL, type SQLWrapper } from "@destack/db";
import { ACCESS_PACKAGE_ID, anyone } from "../policy/principal.ts";
import type { Subject } from "../policy/subject.ts";
import type { RelationshipColumnMap } from "../relationship/table.ts";
import type { Access } from "./access.ts";
import type { GrantFailure } from "./decision.ts";
import type { Grant } from "./grant.ts";

/** The columns naming a subject: its package, type, scope, identifier and, for a subject set, its relation. */
export interface SubjectColumns {
    /** The subject's package. */
    readonly packageId: SQLWrapper;
    /** The subject's type. */
    readonly type: SQLWrapper;
    /** The subject's scope, or `*`. */
    readonly scope: SQLWrapper;
    /** The subject's identifier, or `*`. */
    readonly id: SQLWrapper;
    /** The subject set's relation, absent for a plain subject. */
    readonly relation?: SQLWrapper;
}

/**
 * One authority of a caller: the represented subject, or a delegate with the principal it acts for.
 *
 * It matches subjects in memory and in SQL alike, each rule once in `isMember` and once in `match`.
 */
export class Authority {
    /** The authority's identities and every subject set they belong to. */
    readonly subjects: readonly Subject[];
    /** The principal a delegate acts for, absent for the represented subject. */
    readonly delegator: Subject | undefined;

    /** Name an authority's subjects and, for a delegate, the principal it acts for. */
    constructor(subjects: readonly Subject[], delegator?: Subject) {
        this.subjects = subjects;
        this.delegator = delegator;
    }

    /** Decide whether a subject names the authority: a subject set it belongs to, one of its identities through wildcards, or anyone. */
    isMember(subject: Subject): boolean {
        // match a subject set exactly
        if (subject.relation !== undefined) {
            return this.subjects.some(
                (entry) =>
                    entry.relation === subject.relation &&
                    entry.packageId === subject.packageId &&
                    entry.type === subject.type &&
                    entry.scope === subject.scope &&
                    entry.id === subject.id,
            );
        }

        // match anyone, or an identity through wildcards
        return (
            anyone.is(subject) ||
            this.subjects.some(
                (entry) =>
                    entry.relation === undefined &&
                    entry.packageId === subject.packageId &&
                    entry.type === subject.type &&
                    (subject.scope === entry.scope || subject.scope === "*") &&
                    (subject.id === entry.id || subject.id === "*"),
            )
        );
    }

    /** Match a subject's columns against the authority, as `isMember` decides in memory. */
    match(subject: SubjectColumns): SQL {
        // match a subject set exactly
        if (subject.relation !== undefined) {
            const matches = this.subjects
                .filter((entry) => entry.relation !== undefined)
                .map(
                    (entry) => sql`(
                        ${subject.packageId} = ${entry.packageId}
                        AND ${subject.type} = ${entry.type}
                        AND ${subject.scope} = ${entry.scope}
                        AND ${subject.id} = ${entry.id}
                        AND ${subject.relation} = ${entry.relation}
                    )`,
                );

            return matches.length === 0 ? sql`false` : sql`(${sql.join(matches, sql` OR `)})`;
        }

        // match anyone, or an identity through wildcards
        const matches = this.subjects
            .filter((entry) => entry.relation === undefined)
            .map(
                (entry) => sql`(
                    ${subject.packageId} = ${entry.packageId}
                    AND ${subject.type} = ${entry.type}
                    AND (${subject.scope} = ${entry.scope} OR ${subject.scope} = '*')
                    AND (${subject.id} = ${entry.id} OR ${subject.id} = '*')
                )`,
            );
        matches.push(
            sql`(${subject.packageId} = ${ACCESS_PACKAGE_ID} AND ${subject.type} = ${anyone.name})`,
        );

        return sql`(${sql.join(matches, sql` OR `)})`;
    }

    /** Match a relationship's subject, a plain subject or a subject set, against the authority. */
    member(relationship: RelationshipColumnMap): SQL {
        const subject = {
            packageId: relationship.subjectPackageId,
            type: relationship.subjectType,
            scope: relationship.subjectScope,
            id: relationship.subjectId,
        };

        return sql`(
            (${relationship.subjectRelation} IS NULL AND ${this.match(subject)})
            OR ${this.match({ ...subject, relation: relationship.subjectRelation })}
        )`;
    }

    /** Read why one grant fails to admit the authority, absent when it admits it. */
    failure(grant: Grant, access: Access): GrantFailure | undefined {
        // require each followed arrow to hold without delegation
        const context = access.context;
        if (
            grant.arrows.some((arrow) => arrow.failure(context, undefined, undefined) !== undefined)
        ) {
            return "arrow";
        }
        // lend nothing a field holds to delegates, and match its subject
        else if (grant.condition === undefined) {
            return this.delegator !== undefined
                ? "field"
                : this.isMember(grant.subject)
                  ? undefined
                  : "subject";
        }
        // require a bound role to grant the permission, or everything for ownership
        else if (
            grant.role &&
            !(
                grant.role.permission === undefined
                    ? access.universal()
                    : access.granting(grant.role.permission)
            ).includes(grant.role.id)
        ) {
            return "role";
        }

        // require the relationship to hold and the subject to match
        return (
            grant.condition.failure(context, this.delegator, grant.subject) ??
            (this.isMember(grant.subject) ? undefined : "subject")
        );
    }
}
