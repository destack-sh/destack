import { jsonElements, sql, type SQL, type SQLWrapper } from "@destack/db";
import { AccessContext, Credential } from "../context/context.ts";
import type { RelationshipColumnMap, RelationshipRow } from "../relationship/table.ts";
import type { TupleFailure } from "./decision.ts";

/** The request values a relationship's conditions compare against. */
export type ConditionContext = Pick<
    AccessContext,
    "subject" | "now" | "request" | "credential" | "linkSecrets"
>;

/** The request values a relationship's conditions compare, as SQL values, null if absent. */
export interface ConditionValues {
    /** The request time. */
    readonly now: SQLWrapper;
    /** The request's identifier. */
    readonly request: SQLWrapper;
    /** The key of the credential the request presents. */
    readonly credential: SQLWrapper;
    /** The digests of the request's link secrets, as a JSON list of one-element lists. */
    readonly linkSecrets: SQLWrapper;
    /** The request's authentication assurance level. */
    readonly level: SQLWrapper;
    /** When the request's caller authenticated. */
    readonly authenticatedAt: SQLWrapper;
}

/** A relationship's conditions on its requests, decided in memory and in SQL alike. */
export class TupleCondition {
    /** The relationship's creation time. */
    readonly createdAt: number;
    /** The relationship's expiry, null when it never expires. */
    readonly expiresAt: number | null;
    /** The only request the relationship applies to. */
    readonly requestId: string | null;
    /** The key of the credential a call presents for the relationship to apply. */
    readonly credential: string | null;
    /** The digest of the link secret a request must present. */
    readonly linkSecret: string | null;
    /** The minimum authentication assurance level. */
    readonly assurance: number | null;
    /** The longest time since authentication, in milliseconds. */
    readonly maxAge: number | null;

    /** Read a relationship's conditions from its row. */
    constructor(row: RelationshipRow) {
        // take the times and conditions
        this.createdAt = row.createdAt;
        this.expiresAt = row.expiresAt;
        this.requestId = row.requestId;
        this.credential = row.credential;
        this.linkSecret = row.linkSecret;
        this.assurance = row.assurance;
        this.maxAge = row.maxAge;
    }

    /** Read why the conditions fail a request, absent if they pass. */
    failure(context: ConditionContext): TupleFailure | undefined {
        // require a started, unexpired relationship
        const now = context.now;
        if (this.createdAt > now) {
            return "pending";
        } else if (this.expiresAt !== null && this.expiresAt <= now) {
            return "expired";
        }

        // require its request, credential and link secret to match
        if (!TupleCondition.#isBound(this.requestId, context.request)) {
            return "request";
        } else if (!TupleCondition.#isBound(this.credential, credentialKey(context))) {
            return "credential";
        } else if (this.linkSecret !== null && !context.linkSecrets.includes(this.linkSecret)) {
            return "linkSecret";
        }

        // require the authentication it asks for
        const assurance = AccessContext.assurance(context);
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

        return undefined;
    }

    /** Read the next time the conditions may start or stop passing, absent if never. */
    until(context: ConditionContext): number | undefined {
        return TupleCondition.until(this, context);
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
        const authenticatedAt = AccessContext.assurance(context)?.authenticatedAt;
        const maxAge = times.maxAge;
        const moments = [
            times.createdAt,
            times.expiresAt,
            maxAge === null || authenticatedAt === undefined ? null : authenticatedAt + maxAge,
        ].filter((moment): moment is number => moment !== null && moment > now);

        return moments.length === 0 ? undefined : Math.min(...moments);
    }

    /** Require a relationship's conditions to pass for a request, as `failure` decides. */
    static where(relationship: RelationshipColumnMap, values: ConditionValues): SQL {
        // type the request time for engines that type parameters
        const now = TupleCondition.#castInteger(values.now);

        // require a started, unexpired relationship with a matching request, credential and link secret
        const conditions = [
            sql`${relationship.createdAt} <= ${now}`,
            sql`(${relationship.expiresAt} IS NULL OR ${relationship.expiresAt} > ${now})`,
            sql`(${relationship.requestId} IS NULL OR ${relationship.requestId} = ${TupleCondition.#castText(values.request)})`,
            sql`(${relationship.credential} IS NULL OR ${relationship.credential} = ${TupleCondition.#castText(values.credential)})`,
            sql`(${relationship.linkSecret} IS NULL OR EXISTS (
                SELECT 1 FROM ${jsonElements(values.linkSecrets, "link_secret")}
                WHERE link_secret.value ->> 0 = ${relationship.linkSecret}
            ))`,
        ];

        // require the authentication it asks for
        conditions.push(
            sql`(${relationship.assurance} IS NULL OR ${relationship.assurance} <= ${TupleCondition.#castInteger(values.level)})`,
            sql`(${relationship.maxAge} IS NULL OR ${relationship.maxAge} >= ${now} - ${TupleCondition.#castInteger(values.authenticatedAt)})`,
        );

        return sql`(${sql.join(conditions, sql` AND `)})`;
    }

    /** Bind a request's values as its conditions compare against them. */
    static bindings(context: ConditionContext): Record<keyof ConditionValues, unknown> {
        return {
            now: context.now,
            request: context.request ?? null,
            credential: credentialKey(context) ?? null,
            linkSecrets: JSON.stringify(context.linkSecrets.map((digest) => [digest])),
            level: AccessContext.assurance(context)?.level ?? null,
            authenticatedAt: AccessContext.assurance(context)?.authenticatedAt ?? null,
        };
    }

    /** Embed a request's values in a statement. */
    static values(context: ConditionContext): ConditionValues {
        const bindings = TupleCondition.bindings(context);

        return TupleCondition.parameters((name) => sql`${bindings[name]}`);
    }

    /** List a request's values as `bindings` supplies them to a prepared statement on each run. */
    static parameters(value: (name: keyof ConditionValues) => SQLWrapper): ConditionValues {
        return {
            now: value("now"),
            request: value("request"),
            credential: value("credential"),
            linkSecrets: value("linkSecrets"),
            level: value("level"),
            authenticatedAt: value("authenticatedAt"),
        };
    }

    /** Determine whether a request matches a relationship's binding, always for a null binding. */
    static #isBound(value: string | null, actual: string | undefined): boolean {
        return value === null || (actual !== undefined && value === actual);
    }

    /** Type a nullable SQL value as text for engines that type parameters. */
    static #castText(value: SQLWrapper): SQL {
        return sql`CAST(${value} AS TEXT)`;
    }

    /** Type a nullable SQL value as a big integer for engines that type parameters. */
    static #castInteger(value: SQLWrapper): SQL {
        return sql`CAST(${value} AS BIGINT)`;
    }
}

/** Key the credential a request presented, absent for a principal running its own code. */
function credentialKey(context: Pick<ConditionContext, "credential">): string | undefined {
    return context.credential === undefined ? undefined : Credential.key(context.credential);
}
