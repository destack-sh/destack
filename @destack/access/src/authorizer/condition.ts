import { jsonElements, sql, type SQL, type SQLWrapper } from "@destack/db";
import { Subject } from "@destack/sync";
import { ACCESS_PACKAGE_ID, anyone } from "../declare/principal.ts";
import type { AccessContext } from "../context/context.ts";
import type { RelationshipColumnMap, RelationshipRow } from "../relationship/table.ts";
import type { GrantFailure } from "./decision.ts";

/** The request values a relationship's conditions compare against. */
export type ConditionContext = Pick<
    AccessContext,
    "now" | "request" | "session" | "linkSecrets" | "assurance"
>;

/** The request values a relationship's conditions compare against, as SQL values, null where the request has none. */
export interface ConditionValues {
    /** The request time. */
    readonly now: SQLWrapper;
    /** The request's identifier. */
    readonly request: SQLWrapper;
    /** The request's session. */
    readonly session: SQLWrapper;
    /** The digests of the link secrets the request presented, as a JSON list of one-element lists. */
    readonly linkSecrets: SQLWrapper;
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
    /** The digest of the link secret a request must present. */
    readonly linkSecret: string | null;
    /** The minimum authentication assurance level. */
    readonly assurance: number | null;
    /** The longest time since authentication, in milliseconds. */
    readonly maxAge: number | null;
    /** The key of the principal a delegate must act for. */
    readonly onBehalfOf: string | null;

    /** Read a relationship's conditions from its row. */
    constructor(row: RelationshipRow) {
        // take the times and conditions
        this.createdAt = row.createdAt;
        this.expiresAt = row.expiresAt;
        this.requestId = row.requestId;
        this.sessionId = row.sessionId;
        this.linkSecret = row.linkSecret;
        this.assurance = row.assurance;
        this.maxAge = row.maxAge;
        this.onBehalfOf = row.onBehalfOf;
    }

    /** Read why the conditions fail a request for an authority acting for a delegator, absent when they pass. */
    failure(
        context: ConditionContext,
        delegator: Subject | undefined,
        subject: Subject | undefined,
    ): GrantFailure | undefined {
        // require a started, unexpired relationship
        const now = context.now;
        if (this.createdAt > now) {
            return "pending";
        } else if (this.expiresAt !== null && this.expiresAt <= now) {
            return "expired";
        }

        // require its request, session and link secret to match
        if (!isBound(this.requestId, context.request)) {
            return "request";
        } else if (!isBound(this.sessionId, context.session)) {
            return "session";
        } else if (
            this.linkSecret !== null &&
            !(context.linkSecrets ?? []).includes(this.linkSecret)
        ) {
            return "linkSecret";
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

    /** Read the next moment time alone changes whether the conditions pass, absent when it never does. */
    until(context: ConditionContext): number | undefined {
        return GrantCondition.until(this, context);
    }

    /** Read the next moment a relationship's times start or stop applying. */
    static until(
        times: {
            readonly createdAt: number;
            readonly expiresAt: number | null;
            readonly maxAge: number | null;
        },
        context: ConditionContext,
    ): number | undefined {
        // take the upcoming start, expiry and authentication age limit
        const now = context.now;
        const authenticatedAt = context.assurance?.authenticatedAt;
        const maxAge = times.maxAge;
        const moments = [
            times.createdAt,
            times.expiresAt,
            maxAge === null || authenticatedAt === undefined ? null : authenticatedAt + maxAge,
        ].filter((moment): moment is number => moment !== null && moment > now);

        return moments.length === 0 ? undefined : Math.min(...moments);
    }

    /** Require a relationship's conditions to pass for a request, as `failure` decides in memory. */
    static where(relationship: RelationshipColumnMap, values: ConditionValues): SQL {
        // type the request time for engines that type parameters
        const now = castInteger(values.now);

        // require a started, unexpired relationship with a matching request, session and link secret
        const conditions = [
            sql`${relationship.createdAt} <= ${now}`,
            sql`(${relationship.expiresAt} IS NULL OR ${relationship.expiresAt} > ${now})`,
            sql`(${relationship.requestId} IS NULL OR ${relationship.requestId} = ${castText(values.request)})`,
            sql`(${relationship.sessionId} IS NULL OR ${relationship.sessionId} = ${castText(values.session)})`,
            sql`(${relationship.linkSecret} IS NULL OR EXISTS (
                SELECT 1 FROM ${jsonElements(values.linkSecrets, "link_secret")}
                WHERE link_secret.value ->> 0 = ${relationship.linkSecret}
            ))`,
        ];

        // require the authentication it asks for
        conditions.push(
            sql`(${relationship.assurance} IS NULL OR ${relationship.assurance} <= ${castInteger(values.level)})`,
            sql`(${relationship.maxAge} IS NULL OR ${relationship.maxAge} >= ${now} - ${castInteger(values.authenticatedAt)})`,
        );

        // admit delegations only for a delegate acting for their principal, and besides them only grants to anyone
        conditions.push(
            sql`(
                ${relationship.onBehalfOf} = ${castText(values.delegator)}
                OR (
                    ${relationship.onBehalfOf} IS NULL
                    AND (
                        ${castText(values.delegator)} IS NULL
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

    /** Bind a request's values as its conditions compare against them. */
    static bindings(
        context: ConditionContext,
        delegator?: Subject,
    ): Record<keyof ConditionValues, unknown> {
        return {
            now: context.now,
            request: context.request ?? null,
            session: context.session ?? null,
            linkSecrets: JSON.stringify((context.linkSecrets ?? []).map((digest) => [digest])),
            level: context.assurance?.level ?? null,
            authenticatedAt: context.assurance?.authenticatedAt ?? null,
            delegator: delegator === undefined ? null : Subject.key(delegator),
        };
    }

    /** Embed a request's values in a statement. */
    static values(context: ConditionContext, delegator?: Subject): ConditionValues {
        const bindings = GrantCondition.bindings(context, delegator);

        return GrantCondition.parameters((name) => sql`${bindings[name]}`);
    }

    /** List a request's values as `bindings` supplies them to a prepared statement on each run. */
    static parameters(value: (name: keyof ConditionValues) => SQLWrapper): ConditionValues {
        return {
            now: value("now"),
            request: value("request"),
            session: value("session"),
            linkSecrets: value("linkSecrets"),
            level: value("level"),
            authenticatedAt: value("authenticatedAt"),
            delegator: value("delegator"),
        };
    }
}

/** Determine whether a request matches a relationship's binding, which a null binding always does. */
function isBound(value: string | null, actual: string | undefined): boolean {
    return value === null || (actual !== undefined && value === actual);
}

/** Type a nullable SQL value as text for engines that type parameters. */
function castText(value: SQLWrapper): SQL {
    return sql`CAST(${value} AS TEXT)`;
}

/** Type a nullable SQL value as a big integer for engines that type parameters. */
function castInteger(value: SQLWrapper): SQL {
    return sql`CAST(${value} AS BIGINT)`;
}
