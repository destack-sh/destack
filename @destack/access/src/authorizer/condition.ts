import { jsonElements, sql, type SQL, type SQLWrapper, type Select } from "@destack/db";
import type { AccessContext } from "../context/context.ts";
import { type RelationshipColumnMap, type accessRelationship } from "../relationship/table.ts";
import type { TupleFailure } from "./decision.ts";

/** The request values a relationship's conditions compare against. */
export type ConditionContext = Pick<AccessContext, "now" | "linkSecrets">;

/** The request values a relationship's conditions compare, as SQL values. */
export interface ConditionValues {
    /** The request time. */
    readonly now: SQLWrapper;
    /** The digests of the request's link secrets, as a JSON list of one-element lists. */
    readonly linkSecrets: SQLWrapper;
}

/** A relationship's lifetime and link secret, decided in memory and in SQL alike. */
export class TupleCondition {
    /** The relationship's creation time. */
    readonly createdAt: number;
    /** The relationship's expiry, null when it never expires. */
    readonly expiresAt: number | null;
    /** The digest of the link secret a request must present, null for no share link. */
    readonly linkSecret: string | null;

    /** Read a relationship's lifetime and link secret from its row. */
    constructor(row: Select<typeof accessRelationship>) {
        this.createdAt = row.createdAt;
        this.expiresAt = row.expiresAt;
        this.linkSecret = row.linkSecret;
    }

    /** Read why the relationship fails a request, absent if it applies. */
    failure(context: ConditionContext): TupleFailure | undefined {
        // require a started, unexpired relationship
        const now = context.now;
        if (this.createdAt > now) {
            return "pending";
        } else if (this.expiresAt !== null && this.expiresAt <= now) {
            return "expired";
        }
        // require a share link's secret
        else if (this.linkSecret !== null && !context.linkSecrets.includes(this.linkSecret)) {
            return "linkSecret";
        }

        return undefined;
    }

    /** Read the next time the relationship may start or stop applying, absent if never. */
    until(context: ConditionContext): number | undefined {
        return TupleCondition.until(this, context);
    }

    /** Read the next moment a relationship's times start or stop applying. */
    static until(
        times: { readonly createdAt: number; readonly expiresAt: number | null },
        context: Pick<ConditionContext, "now">,
    ): number | undefined {
        const moments = [times.createdAt, times.expiresAt].filter(
            (moment): moment is number => moment !== null && moment > context.now,
        );

        return moments.length === 0 ? undefined : Math.min(...moments);
    }

    /** Require a relationship to apply to a request, as `failure` decides. */
    static where(relationship: RelationshipColumnMap, values: ConditionValues): SQL {
        // require a started, unexpired relationship, and a share link's secret
        const now = sql`CAST(${values.now} AS BIGINT)`;
        const conditions = [
            sql`${relationship.createdAt} <= ${now}`,
            sql`(${relationship.expiresAt} IS NULL OR ${relationship.expiresAt} > ${now})`,
            sql`(${relationship.linkSecret} IS NULL OR EXISTS (
                SELECT 1 FROM ${jsonElements(values.linkSecrets, "link_secret")}
                WHERE link_secret.value ->> 0 = ${relationship.linkSecret}
            ))`,
        ];

        return sql`(${sql.join(conditions, sql` AND `)})`;
    }

    /** Bind a request's values as the conditions compare against them. */
    static bindings(context: ConditionContext): Record<keyof ConditionValues, unknown> {
        return {
            now: context.now,
            linkSecrets: JSON.stringify(context.linkSecrets.map((digest) => [digest])),
        };
    }

    /** Embed a request's values in a statement. */
    static values(context: ConditionContext): ConditionValues {
        const bindings = TupleCondition.bindings(context);

        return TupleCondition.parameters((name) => sql`${bindings[name]}`);
    }

    /** List a request's values as `bindings` supplies them to a prepared statement on each run. */
    static parameters(value: (name: keyof ConditionValues) => SQLWrapper): ConditionValues {
        return { now: value("now"), linkSecrets: value("linkSecrets") };
    }
}
