import { Package } from "@destack/package";
import { defineSchema, type schema } from "@destack/schema";
import { type Meter, MeterMetadata } from "../meter/meter.ts";

/** A meter declaration as manifests describe it. */
export const MeterDescription = defineSchema(
    MeterMetadata.extend({
        /** The declaring package. */
        package: Package,
    }),
);
/** A meter declaration as manifests describe it. */
export type MeterDescription = schema.Infer<typeof MeterDescription>;

/** Describe a meter with its aggregation and unit. */
export function describeMeter(meter: Meter): MeterDescription {
    return { ...meter.definition, package: meter.package };
}
