import { type Package, PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** How a meter folds a period's events into one usage. */
export const MeterAggregation = schema.enum(["sum", "count", "max", "last", "average"]);
/** How a meter folds the events of a period into one usage. */
export type MeterAggregation = schema.Infer<typeof MeterAggregation>;

/** A meter's package-local name, dotted like OpenTelemetry instrument names. */
export const MeterName = schema.string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$/u);

/** The identity of a meter across package renames and releases: its package and name. */
export const MeterReference = schema.object({
    /** The declaring package. */
    packageId: PackageId,
    /** The package-local name. */
    name: MeterName,
});
/** The identity of a meter across package renames and releases. */
export type MeterReference = schema.Infer<typeof MeterReference>;

/** The fields of a meter declaration. */
export const MeterMetadata = schema.object({
    /** The package-local name. */
    name: MeterName,
    /** What the meter counts. */
    description: schema.string().min(1),
    /** How the meter folds a period's events. */
    aggregation: MeterAggregation,
    /** The unit of an event's value, such as request or byte. */
    unit: schema.string().min(1),
});
/** The fields of a meter declaration. */
export type MeterDefinition = schema.Infer<typeof MeterMetadata>;

/** A meter declaration: what its events count and how a period's events fold into usage. */
export class Meter {
    /** The declaring package. */
    readonly package: Package;
    /** The name, description, aggregation and unit. */
    readonly definition: MeterDefinition;

    /** Take a checked declaration. */
    constructor(owner: Package, definition: MeterDefinition) {
        this.package = owner;
        this.definition = definition;
    }

    /** The identity its events and features refer to. */
    get reference(): MeterReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Serialise the meter as its reference. */
    toJSON(): MeterReference {
        return this.reference;
    }
}
