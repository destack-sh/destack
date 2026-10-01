import { jsonElements, sql, type SQL, type SQLWrapper } from "@destack/db";
import { Subject } from "@destack/sync";
import { ACCESS_PACKAGE_ID, anyone } from "../policy/principal.ts";
import type { AccessContext } from "../context/context.ts";
import type { RelationshipColumnMap, RelationshipRow } from "../relationship/table.ts";
import type { GrantFailure } from "./decision.ts";

/** The request facts a relationship's conditions compare against. */
export type ConditionContext = Pick<
    AccessContext,
    "now" | "request" | "session" | "capabilities" | "assurance"
>;

/** The request facts a relationship's conditions compare against, as SQL values, null where the request has none. */
export interface ConditionValues {
    /** The request time. */
    readonly now: SQLWrapper;
    /** The request's identifier. */
    readonly request: SQLWrapper;
    /** The request's session. */
    readonly session: SQLWrapper;
    /** The digests of the capabilities the request presented, as a JSON list of one-element lists. */
    readonly capabilities: SQLWrapper;
    /** The request's authentication assurance level. */
    readonly level: SQLWrapper;
    /** When the request's caller authenticated. */
    readonly authenticatedAt: SQLWrapper;
    /** The key of the principal a compiled delegate acts for. */
    readonly delegator: SQLWrapper;
}

/**
 * A relationship's conditions on the requests it applies to, decided in memory and in SQL alike.
 *
 * Each condition appears in `failure` and in `where` in the same order, and both evaluators decide it the same way.
 */
export class GrantCondition {
    /** The relationship's creation time. */
    readonly createdAt: number;
    /** The relationship's expiry, null when it never expires. */
    readonly expiresAt: number | null;
    /** The request the relationship applies to alone. */
    readonly requestId: string | null;
    /** The session the relationship applies within. */
    readonly sessionId: string | null;
    /** The digest of the capability a request must present. */
    readonly capability: string | null;
    /** The minimum authentication assurance level. */
    readonly assurance: number | null;
    /** The longest time since authentication, in milliseconds. */
    readonly maxAge: number | null;
    /** The key of the principal a delegate must act for. */
    readonly onBehalfOf: string | null;

    /** Read a relationship's conditions from its row. */
    constructor(row: RelationshipRow) {
        // take the times and conditions and read PostgreSQL's text integers as numbers
        this.createdAt = Number(row.createdAt);
        this.expiresAt = integerOrNull(row.expiresAt);
        this.requestId = row.requestId;
        this.sessionId = row.sessionId;
        this.capability = row.capability;
        this.assurance = integerOrNull(row.assurance);
        this.maxAge = integerOrNull(row.maxAge);
        this.onBehalfOf = row.onBehalfOf;
    }

    /** Read why the conditions fail a request for an authority acting for a delegator, absent when they hold. */
    failure(
        context: ConditionContext,
        delegator: Subject | undefined,
        subject: Subject | undefined,
    ): GrantFailure | undefined {
        // require the relationship to have started and not expired
        const now = context.now;
        if (this.createdAt > now) {
            return "pending";
        } else if (this.expiresAt !== null && this.expiresAt <= now) {
            return "expired";
        }

        // require its request, session and capability to match
        const bound = (value: string | null, held: string | undefined) =>
            value === null || (held !== undefined && value === held);
        if (!bound(this.requestId, context.request)) {
            return "request";
        } else if (!bound(this.sessionId, context.session)) {
            return "session";
        } else if (
            this.capability !== null &&
            !(context.capabilities ?? []).includes(this.capability)
        ) {
            return "capability";
        }

        // require the authentication it asks for
        const assurance = context.assurance;
        if (
            this.assurance !== null &&
            (assurance === undefined || this.assurance > assurance.level)
        ) {
            return "assurance";
        } else if (
            this.maxAge !== null &&
            (assurance === undefined || this.maxAge < now - assurance.authenticatedAt)
        ) {
            return "age";
        }

        // admit delegations only for a delegate acting for their principal, and besides them only grants to anyone
        const isLent =
            delegator === undefined
                ? this.onBehalfOf === null
                : this.onBehalfOf === Subject.key(delegator) ||
                  (this.onBehalfOf === null && subject !== undefined && anyone.is(subject));

        return isLent ? undefined : "delegation";
    }

    /** Read the next moment time alone changes whether the conditions hold, absent when it never does. */
    boundary(context: ConditionContext): number | undefined {
        return GrantCondition.boundary(this, context);
    }

    /** Read the next moment a relationship's times start or stop holding. */
    static boundary(
        times: {
            readonly createdAt: number | string;
            readonly expiresAt: number | string | null;
            readonly maxAge: number | string | null;
        },
        context: ConditionContext,
    ): number | undefined {
        // take the upcoming start, expiry and authentication age limit
        const now = context.now;
        const authenticatedAt = context.assurance?.authenticatedAt;
        const maxAge = integerOrNull(times.maxAge);
        const moments = [
            Number(times.createdAt),
            integerOrNull(times.expiresAt),
            maxAge === null || authenticatedAt === undefined ? null : authenticatedAt + maxAge,
        ].filter((moment): moment is number => moment !== null && moment > now);

        return moments.length === 0 ? undefined : Math.min(...moments);
    }

    /** Require a relationship's conditions to hold for a request, as `failure` decides in memory. */
    static where(relationship: RelationshipColumnMap, values: ConditionValues): SQL {
        // type each nullable value for engines that type parameters
        const now = sql`CAST(${values.now} AS BIGINT)`;
        const text = (value: SQLWrapper) => sql`CAST(${value} AS TEXT)`;
        const integer = (value: SQLWrapper) => sql`CAST(${value} AS BIGINT)`;

        // require the relationship to have started and not expired, and its request, session and capability to match
        const conditions = [
            sql`${relationship.createdAt} <= ${now}`,
            sql`(${relationship.expiresAt} IS NULL OR ${relationship.expiresAt} > ${now})`,
            sql`(${relationship.requestId} IS NULL OR ${relationship.requestId} = ${text(values.request)})`,
            sql`(${relationship.sessionId} IS NULL OR ${relationship.sessionId} = ${text(values.session)})`,
            sql`(${relationship.capability} IS NULL OR EXISTS (
                SELECT 1 FROM ${jsonElements(values.capabilities, "capability")}
                WHERE capability.value ->> 0 = ${relationship.capability}
            ))`,
        ];

        // require the authentication it asks for
        conditions.push(
            sql`(${relationship.assurance} IS NULL OR ${relationship.assurance} <= ${integer(values.level)})`,
            sql`(${relationship.maxAge} IS NULL OR ${relationship.maxAge} >= ${now} - ${integer(values.authenticatedAt)})`,
        );

        // admit delegations only for a delegate acting for their principal, and besides them only grants to anyone
        conditions.push(
            sql`(
                ${relationship.onBehalfOf} = ${text(values.delegator)}
                OR (
                    ${relationship.onBehalfOf} IS NULL
                    AND (
                        ${text(values.delegator)} IS NULL
                        OR (
                            ${relationship.subjectPackageId} = ${ACCESS_PACKAGE_ID}
                            AND ${relationship.subjectType} = ${anyone.name}
                        )
                    )
                )
            )`,
        );

        return sql`(${sql.join(conditions, sql` AND `)})`;
    }

    /** Bind a request's facts as the values its conditions compare against. */
    static bindings(
        context: ConditionContext,
        delegator?: Subject,
    ): Record<keyof ConditionValues, unknown> {
        return {
            now: context.now,
            request: context.request ?? null,
            session: context.session ?? null,
            capabilities: JSON.stringify((context.capabilities ?? []).map((digest) => [digest])),
            level: context.assurance?.level ?? null,
            authenticatedAt: context.assurance?.authenticatedAt ?? null,
            delegator: delegator === undefined ? null : Subject.key(delegator),
        };
    }

    /** Embed a request's facts in a statement. */
    static values(context: ConditionContext, delegator?: Subject): ConditionValues {
        const bindings = GrantCondition.bindings(context, delegator);

        return Object.fromEntries(
            Object.entries(bindings).map(([name, bound]) => [name, sql`${bound}`]),
        ) as unknown as ConditionValues;
    }

    /** List a request's facts as the values `bindings` supplies to a prepared statement on each run. */
    static parameters(value: (name: string) => SQLWrapper): ConditionValues {
        return {
            now: value("now"),
            request: value("request"),
            session: value("session"),
            capabilities: value("capabilities"),
            level: value("level"),
            authenticatedAt: value("authenticatedAt"),
            delegator: value("delegator"),
        };
    }
}

/** Read an integer a driver returns as a number or, for PostgreSQL's big integers, as text. */
function integerOrNull(value: number | string | null): number | null {
    return value === null ? null : Number(value);
}
