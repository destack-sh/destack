import type { Meter } from "../meter/meter.ts";
import type { MeterDescription } from "../meter/description.ts";

/** Describe a meter with its aggregation and unit. */
export function describeMeter(meter: Meter): MeterDescription {
    return { ...meter.definition, package: meter.package };
}
