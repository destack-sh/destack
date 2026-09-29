import { principal } from "@destack/access";
import { unique, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";

/** The supported storage and processing jurisdictions. */
export const RESIDENCIES = ["eu", "us"] as const;

/** The permitted jurisdiction for storage and processing. */
export const Residency = defineSchema(schema.enum(RESIDENCIES));
/** The permitted jurisdiction for storage and processing. */
export type Residency = schema.Infer<typeof Residency>;

/** A Destack regional deployment. */
export const region = defineObject({
    name: "region",
    tier: "global",
    plural: "regions",
    scope: "universe",
    represents: principal.region,
    fields: {
        /** The region's short code, such as eu-central. */
        code: field.string(),
        /** The display name. */
        name: field.string(),
        /** The jurisdiction the region keeps data in. */
        residency: field.enum(RESIDENCIES),
    },
    permissions: ["serve"],
    constraints: (region) => [
        unique("region_code").on(region.code),
        unique("region_residency_id").on(region.id, region.residency),
    ],
});
/** A Destack regional deployment. */
export type Region = Select<typeof region.table>;
