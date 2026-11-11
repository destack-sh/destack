import { DeclarationReference } from "@destack/package/declare";
import type {} from "@destack/package/import-meta";
import type { BuildReader } from "@destack/package/manifest";
import { DeclarationName, defineKind, PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { Message } from "@destack/locale";

/** How a meter folds a period's events into one usage. */
export const MeterAggregation = schema.enum(["sum", "count", "max", "last", "average"]);
/** How a meter folds the events of a period into one usage. */
export type MeterAggregation = schema.Infer<typeof MeterAggregation>;

/** The values an attribute of a meter's events takes, such as read and write. */
export const MeterAttribute = schema.array(schema.string().min(1)).min(1);
/** The values an attribute of a meter's events takes. */
export type MeterAttribute = schema.Infer<typeof MeterAttribute>;

/** The value of each attribute one event of a meter carries, such as direction read. */
export const MeterAttributes = schema.record(schema.string().min(1), schema.string().min(1));
/** The value of each attribute one event of a meter carries. */
export type MeterAttributes = schema.Infer<typeof MeterAttributes>;

/** The fields of a meter declaration. */
export const MeterMetadata = schema.object({
    /** The package-local name. */
    name: DeclarationName,
    /** What the meter counts. */
    description: Message,
    /** How the meter folds a period's events. */
    aggregation: MeterAggregation,
    /** The unit of an event's value, such as request or byte. */
    unit: schema.string().min(1),
    /** The attributes its events carry, each with the values it takes, priced apart. */
    attributes: schema.record(schema.string().min(1), MeterAttribute),
});
/** The fields of a meter declaration. */
export type MeterDefinition = schema.Infer<typeof MeterMetadata>;

/** A meter declaration as manifests describe it. */
export const MeterDescription = MeterMetadata.extend({
    /** The declaring package. */
    package: PackageVersion,
});
/** A meter declaration as manifests describe it. */
export type MeterDescription = schema.Infer<typeof MeterDescription>;

/** The meter declaration kind, which telemetry defines. */
export const METER_KIND = defineKind({ name: "meter", description: MeterDescription });

/** A meter declaration: what its events count and how a period's events fold into usage. */
export class Meter {
    /** The declaring package. */
    readonly package: PackageVersion;
    /** The name, description, aggregation and unit. */
    readonly definition: MeterDefinition;

    /** Take a checked declaration. */
    constructor(owner: PackageVersion, definition: MeterDefinition) {
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

    /** Check that attribute values set each declared attribute, and only those, to its values. */
    accepts(attributes: MeterAttributes): boolean {
        const declared = Object.entries(this.definition.attributes);

        return (
            Object.keys(attributes).length === declared.length &&
            declared.every(([name, values]) => values.includes(attributes[name] ?? ""))
        );
    }

    /** Read the key events with some attribute values are rated by, refusing values it lacks. */
    key(attributes: MeterAttributes): string {
        // require exactly the declared attributes, each at a declared value
        if (!this.accepts(attributes)) {
            throw new TypeError(
                `meter '${this.name}' takes attributes ${JSON.stringify(this.definition.attributes)}, not ${JSON.stringify(attributes)}`,
            );
        }

        return Meter.key(this.reference, attributes);
    }

    /** Write the key of a meter's events with some attribute values, attributes in name order. */
    static key(meter: DeclarationReference, attributes: MeterAttributes): string {
        const pairs = Object.entries(attributes)
            .toSorted(([left], [right]) => left.localeCompare(right))
            .map(([name, value]) => `${name}=${value}`);

        return [DeclarationReference.key(meter), ...pairs].join(" ");
    }

    /** Serialise the meter as its reference. */
    toJSON(): DeclarationReference {
        return this.reference;
    }
}
