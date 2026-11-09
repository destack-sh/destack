import { Address, type Comparator } from "@destack/resource";
import { unreachablePlan } from "@destack/resource/error";
import type { JsonValue } from "@destack/schema";
import { type Meter, MeterDescription } from "../meter/meter.ts";

/** The fields usage counted under a meter depends on, its SKUs' attribute values included. */
const COUNTED = ["aggregation", "unit", "attributes"] as const;

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
    const problems = COUNTED.filter((field) => textOf(earlier[field]) !== textOf(later[field])).map(
        (field) => ({
            target,
            detail: `keep ${field} ${textOf(earlier[field])}, or declare a meter of ${field} ${textOf(later[field])}`,
        }),
    );
    if (problems.length > 0) {
        throw unreachablePlan(problems);
    }

    return { steps: [] };
};

/** List a meter's term: how it counts. */
export function meterVocabulary(declared: Meter): Record<string, JsonValue> {
    const { name, aggregation, unit, attributes } = declared.definition;

    return { [Address.join("meter", name)]: { aggregation, unit, attributes } };
}

/** Write a counted field as text: a string as it is, attributes as JSON. */
function textOf(value: string | Readonly<Record<string, readonly string[]>>): string {
    return typeof value === "string" ? value : JSON.stringify(value);
}
