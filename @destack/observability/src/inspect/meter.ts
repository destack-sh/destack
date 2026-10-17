import { Address, type Comparator } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import type { JsonValue } from "@destack/schema";
import type { Meter } from "../meter/meter.ts";
import { MeterDescription } from "../meter/description.ts";

/** The fields usage counted under a meter depends on. */
const COUNTED = ["aggregation", "unit"] as const;

/** Describe a meter with its aggregation and unit. */
export function describeMeter(meter: Meter): MeterDescription {
    return { ...meter.definition, package: meter.package };
}

/** Plan a meter's change between releases: keep how it counts, as counted usage follows it. */
export const compareMeter: Comparator = (before, after) => {
    // refuse a change of aggregation or unit
    const earlier = MeterDescription.parse(before.description);
    const later = MeterDescription.parse(after.description);
    const target = Address.join("meter", later.name);
    const problems = COUNTED.filter((field) => earlier[field] !== later[field]).map((field) => ({
        target,
        detail: `keep ${field} ${earlier[field]}, or declare a meter of ${field} ${later[field]}`,
    }));
    if (problems.length > 0) {
        throw new PlanError(problems);
    }

    return { steps: [] };
};

/** List a meter's term: how it counts. */
export function meterVocabulary(declared: Meter): Record<string, JsonValue> {
    const { name, aggregation, unit } = declared.definition;

    return { [Address.join("meter", name)]: { aggregation, unit } };
}
