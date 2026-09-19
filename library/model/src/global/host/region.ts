import { check, recordColumns, type Select, sql, table, text, unique } from "@destack/db";
import { RESIDENCIES } from "./residency.ts";

/** A provider location with an explicit residency classification. */
export const region = table("region", {
    ...recordColumns("region"),
    /** The provider operating this location. */
    providerCode: text("provider").notNull(),
    /** The location code accepted by the provider API. */
    code: text("code").notNull(),
    /** The displayed location name. */
    name: text("name").notNull(),
    /** The storage and processing jurisdiction. */
    residency: text("residency", { enum: RESIDENCIES }).notNull(),
}, (region) => [
    unique("region_provider_code").on(region.providerCode, region.code),
    unique("region_provider_id").on(region.providerCode, region.id),
    unique("region_residency_id").on(region.id, region.residency),
    check("region_residency", sql`${region.residency} IN ('eu', 'us')`),
]);

/** A provider location. */
export type Region = Select<typeof region>;
