import * as schema from "../validate/index.ts";
import { PlainDate } from "./plain.ts";

/** The schema of a range of dates. */
const dateRangeSchema = schema
    .object({
        /** The first date. */
        from: PlainDate,
        /** The last date, the first or after it. */
        to: PlainDate,
    })
    .meta({ id: "schema.dateRange", title: "Dates" });

/** A range of calendar dates, both ends included. */
export type DateRange = schema.Infer<typeof dateRangeSchema>;

/** A range of calendar dates, both ends included, as a date range picker gives it, from and to. */
export const DateRange = Object.assign(dateRangeSchema, {
    /** Make a range of two dates in either order, the earlier first. */
    of(first: PlainDate, second: PlainDate): DateRange {
        return PlainDate.compare(first, second) <= 0
            ? { from: first, to: second }
            : { from: second, to: first };
    },
});
