import { PackageId, type Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";

/** A package-local meter name, kept across releases. */
export const MeterName = defineSchema(
    schema.string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$(?![\s\S])/u),
);

/** How a meter folds the events of a period into one usage. */
export const MeterAggregation = defineSchema(schema.enum(["sum", "count", "max", "last"]));
/** How a meter folds the events of a period into one usage. */
export type MeterAggregation = schema.Infer<typeof MeterAggregation>;

/** The identity of a meter across package renames and releases. */
export const MeterReference = Object.assign(
    defineSchema(
        schema.object({
            /** The package declaring the meter. */
            packageId: PackageId,
            /** The package-local name. */
            name: MeterName,
        }),
    ),
    {
        /** Key a meter by its package and name. */
        key(reference: MeterReference): string {
            return `${reference.packageId}/${reference.name}`;
        },
    },
);
/** The identity of a meter. */
export type MeterReference = schema.Infer<typeof MeterReference>;

/** The fields of a meter declaration. */
export const MeterMetadata = defineSchema(
    schema.object({
        /** The package-local name. */
        name: MeterName,
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
