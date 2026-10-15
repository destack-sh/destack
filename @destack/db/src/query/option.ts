import { schema } from "@destack/schema";
import { Condition } from "./condition.ts";
import { OrderBy } from "./order.ts";
import { Expression } from "../expression/expression.ts";
import type { Extras } from "./namespace.ts";
import type { Row } from "../table/row.ts";

/** Which rows of one table a read selects, in what order, and how many. */
export interface QueryOptions<Read = Row> {
    /** The condition the rows meet. */
    readonly where?: Condition<Read>;
    /** How the rows sort, completed by the primary key. */
    readonly orderBy?: OrderBy<Read>;
    /** The most rows selected, per selected row for an included relation. */
    readonly limit?: number;
}

/** Drizzle's relational read options in open form: rows, kept columns, extras and includes. */
export interface OpenFindOptions extends QueryOptions {
    /** The columns each result keeps: only those set true, or all but those set false. */
    readonly columns?: Readonly<Record<string, boolean>>;
    /** The values computed from each row, read like fields, by name. */
    readonly extras?: Extras;
    /** The related rows each selected row includes, by relation: every row, or a selection. */
    readonly with?: Readonly<Record<string, true | OpenFindOptions>>;
}

/** The schema of relational read options in open form, checking typed options at a read. */
export const OpenFindOptions: schema.Schema<OpenFindOptions> = schema.lazy(() =>
    schema.object({
        where: Condition.schema.exactOptional(),
        orderBy: OrderBy.schema.exactOptional(),
        limit: schema.number().int().min(1).exactOptional(),
        columns: schema.record(schema.string(), schema.boolean()).exactOptional(),
        extras: schema.record(schema.string(), Expression.schema).exactOptional(),
        with: schema
            .record(schema.string(), schema.union([schema.literal(true), OpenFindOptions]))
            .exactOptional(),
    }),
);
