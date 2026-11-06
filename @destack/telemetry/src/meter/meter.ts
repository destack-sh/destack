import { DeclarationReference } from "@destack/package/declare";
import type {} from "@destack/package/import-meta";
import type { BuildReader } from "@destack/package/manifest";
import { DeclarationName, defineDeclarationKind, Package } from "@destack/package";
import { schema } from "@destack/schema";

/** How a meter folds a period's events into one usage. */
export const MeterAggregation = schema.enum(["sum", "count", "max", "last", "average"]);
/** How a meter folds the events of a period into one usage. */
export type MeterAggregation = schema.Infer<typeof MeterAggregation>;

/** The fields of a meter declaration. */
export const MeterMetadata = schema.object({
    /** The package-local name. */
    name: DeclarationName,
    /** What the meter counts. */
    description: schema.string().min(1),
    /** How the meter folds a period's events. */
    aggregation: MeterAggregation,
    /** The unit of an event's value, such as request or byte. */
    unit: schema.string().min(1),
});
/** The fields of a meter declaration. */
export type MeterDefinition = schema.Infer<typeof MeterMetadata>;

/** A meter declaration as manifests describe it. */
export const MeterDescription = MeterMetadata.extend({
    /** The declaring package. */
    package: Package,
});
/** A meter declaration as manifests describe it. */
export type MeterDescription = schema.Infer<typeof MeterDescription>;

/** The meter declaration kind, which telemetry defines. */
export const METER_KIND = defineDeclarationKind("meter", MeterDescription);

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

    /** Read the meters a build declares. */
    static async list(build: BuildReader): Promise<Meter[]> {
        const declared = await METER_KIND.read(build);

        return declared.map(
            ({ description: { package: declaring, ...definition } }) =>
                new Meter(declaring, definition),
        );
    }

    /** The identity its events and features refer to. */
    get reference(): DeclarationReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Serialise the meter as its reference. */
    toJSON(): DeclarationReference {
        return this.reference;
    }
}
