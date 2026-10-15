import type { Condition } from "./condition.ts";
import type { OrderBy } from "./order.ts";
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
