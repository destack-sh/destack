import { RecordProvenance } from "@destack/model/source";
import { defineSchema, identifier, schema } from "@destack/schema";
import { SettingPolicyDefinition } from "../declare/policy.ts";

/** A default or required value within an authority's verified scope. */
export const SettingPolicy = defineSchema(
    SettingPolicyDefinition.extend({
        /** Stable policy identity. */
        id: identifier("setting-policy"),
        /** Opaque revision used for conditional edits. */
        revision: schema.uuid(),
        /** The declaration last applied to this record. */
        provenance: RecordProvenance.nullable(),
        /** UTC milliseconds when reconciliation stopped. */
        detachedAt: schema.number().int().nonnegative().nullable(),
        /** Creation time in UTC milliseconds. */
        createdAt: schema.number().int().nonnegative(),
        /** Last accepted change in UTC milliseconds. */
        updatedAt: schema.number().int().nonnegative(),
    }),
);
/** A persisted recommendation or requirement. */
export type SettingPolicy = schema.Infer<typeof SettingPolicy>;
