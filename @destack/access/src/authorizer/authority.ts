import { jsonElements, sql, type SQL, type SQLWrapper } from "@destack/db";
import type { Subject } from "@destack/sync";
import { ACCESS_PACKAGE_ID, anyone } from "../declare/principal.ts";
import type { RelationshipColumnMap } from "../relationship/table.ts";
import type { Access } from "./access.ts";
import type { GrantFailure } from "./decision.ts";
import type { Grant } from "./grant.ts";

/** The name of the list element a match reads an authority's subject from. */
const SUBJECT = "access_subject";

/** The columns of a subject: its package, type, scope, identifier and set relation. */
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
 * It matches subjects in memory with `isMember` and in SQL with `match`, with the same rules.
 */
export class Authority {
    /** The authority's identities and every subject set they belong to. */
    readonly subjects: readonly Subject[];
    /** The delegate and the principal it acts for, absent for the represented subject. */
    readonly delegation: { readonly delegate: Subject; readonly delegator: Subject } | undefined;
    /** The authority's plain identities as one JSON list, each `[package, type, scope, id]`. */
    readonly #identities: string;
    /** The authority's subject sets as one JSON list, each `[package, type, scope, id, relation]`. */
    readonly #sets: string;

    /** Name an authority's subjects and, for a delegate, the principal it acts for. */
    constructor(
        subjects: readonly Subject[],
        delegation?: { readonly delegate: Subject; readonly delegator: Subject },
    ) {
        // keep the subjects, and list them once for SQL
        this.subjects = subjects;
        this.delegation = delegation;
        this.#identities = JSON.stringify(
            subjects
                .filter((entry) => entry.relation === undefined)
                .map((entry) => [entry.packageId, entry.type, entry.scope, entry.id]),
        );
        this.#sets = JSON.stringify(
            subjects.flatMap((entry) =>
                entry.relation === undefined
                    ? []
                    : [[entry.packageId, entry.type, entry.scope, entry.id, entry.relation]],
            ),
        );
    }

    /** The principal a delegate acts for, absent for the represented subject. */
    get delegator(): Subject | undefined {
        return this.delegation?.delegator;
    }

    /** Decide whether a subject matches the authority: through a subject set, a wildcard identity, or anyone. */
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

    /** Match a subject's columns against the authority, as `isMember` decides in memory, its subjects bound as one list whatever their number. */
    match(subject: SubjectColumns): SQL {
        // match a subject set exactly
        const entry = sql.identifier(SUBJECT);
        const element = (index: number) => sql`${entry}.value ->> ${sql.raw(String(index))}`;
        if (subject.relation !== undefined) {
            return sql`EXISTS (
                SELECT 1 FROM ${jsonElements(sql`${this.#sets}`, SUBJECT)}
                WHERE ${subject.packageId} = ${element(0)}
                    AND ${subject.type} = ${element(1)}
                    AND ${subject.scope} = ${element(2)}
                    AND ${subject.id} = ${element(3)}
                    AND ${subject.relation} = ${element(4)}
            )`;
        }

        // match anyone, or an identity through wildcards
        return sql`(
            EXISTS (
                SELECT 1 FROM ${jsonElements(sql`${this.#identities}`, SUBJECT)}
                WHERE ${subject.packageId} = ${element(0)}
                    AND ${subject.type} = ${element(1)}
                    AND (${subject.scope} = ${element(2)} OR ${subject.scope} = '*')
                    AND (${subject.id} = ${element(3)} OR ${subject.id} = '*')
            )
            OR (${subject.packageId} = ${ACCESS_PACKAGE_ID} AND ${subject.type} = ${anyone.name})
        )`;
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
        // require each followed arrow to pass without delegation
        const context = access.context;
        if (
            grant.arrows.some((arrow) => arrow.failure(context, undefined, undefined) !== undefined)
        ) {
            return "arrow";
        }
        // lend nothing in a field to delegates, and match its subject
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

        // require the relationship to pass and the subject to match
        return (
            grant.condition.failure(context, this.delegator, grant.subject) ??
            (this.isMember(grant.subject) ? undefined : "subject")
        );
    }
}
