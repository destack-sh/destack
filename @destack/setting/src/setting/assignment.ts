import { RecordProvenance } from "@destack/model/source";
import { defineSchema, identifier, schema } from "@destack/schema";
import { SettingReference } from "./setting.ts";
import { SettingTarget } from "./target.ts";

/** An explicit value at one exact setting target. */
export const SettingAssignment = defineSchema(
    schema.object({
        /** Stable identity for this assignment. */
        id: identifier("setting-assignment"),
        /** The configured declaration. */
        setting: SettingReference,
        /** The exact person or runtime selection. */
        target: SettingTarget,
        /** The complete value validated by the consuming declaration. */
        value: schema.json(),
        /** Opaque revision used for conditional edits. */
        revision: schema.uuid(),
        /** The declaration last applied to this record, or null for direct creation. */
        provenance: RecordProvenance.nullable(),
        /** UTC milliseconds when reconciliation stopped, or null while attached. */
        detachedAt: schema.number().int().nonnegative().nullable(),
        /** Creation time in UTC milliseconds. */
        createdAt: schema.number().int().nonnegative(),
        /** Last accepted change in UTC milliseconds. */
        updatedAt: schema.number().int().nonnegative(),
    }),
);
/** A persisted explicit choice. */
export type SettingAssignment = schema.Infer<typeof SettingAssignment>;
