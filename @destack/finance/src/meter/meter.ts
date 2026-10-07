import type { Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { CatalogName, CatalogReference } from "../catalog/reference.ts";

/** How a meter folds a period's events into one usage. */
export const MeterAggregation = defineSchema(
    schema.enum(["sum", "count", "max", "last", "average"]),
);
/** How a meter folds the events of a period into one usage. */
export type MeterAggregation = schema.Infer<typeof MeterAggregation>;

/** The fields of a meter declaration. */
export const MeterMetadata = defineSchema(
    schema.object({
        /** The package-local name. */
        name: CatalogName,
        /** What the meter counts. */
        description: schema.string().min(1),
        /** How the meter folds a period's events. */
        aggregation: MeterAggregation,
        /** The unit of an event's value, such as request or byte. */
        unit: schema.string().min(1),
    }),
);
/** The fields of a meter declaration. */
export type MeterDefinition = schema.Infer<typeof MeterMetadata>;

/** A meter declaration: what its events count and how a period's events fold into usage. */
export class Meter {
    /** The declaring package. */
    readonly package: Package;
    /** The name, description, aggregation and unit. */
    readonly definition: MeterDefinition;

    /** Hold a checked declaration. */
    constructor(owner: Package, definition: MeterDefinition) {
        this.package = owner;
        this.definition = definition;
    }

    /** The identity its events and features refer to. */
    get reference(): CatalogReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Serialise the meter as its reference. */
    toJSON(): CatalogReference {
        return this.reference;
    }
}
