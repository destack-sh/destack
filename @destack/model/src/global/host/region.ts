import { check, recordColumns, type Select, sql, table, text, unique } from "@destack/db";
import { RESIDENCIES } from "./residency.ts";

/** A Destack regional deployment with an explicit residency classification. */
export const region = table(
    "region",
    {
        ...recordColumns("region"),
        /** The stable code identifying this Destack regional deployment. */
        code: text("code").notNull(),
        /** The displayed region name. */
        name: text("name").notNull(),
        /** The storage and processing jurisdiction. */
        residency: text("residency", { enum: RESIDENCIES }).notNull(),
    },
    (region) => [
        unique("region_code").on(region.code),
        unique("region_residency_id").on(region.id, region.residency),
        check("region_residency", sql`${region.residency} IN ('eu', 'us')`),
    ],
);

/** A Destack regional deployment. */
export type Region = Select<typeof region>;
