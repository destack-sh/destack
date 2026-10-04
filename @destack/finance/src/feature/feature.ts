import { PackageId, type Package } from "@destack/package";
import { defineSchema, schema, type JsonValue } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { MeterReference } from "../meter/meter.ts";

/** A package-local feature name, kept across releases. */
export const FeatureName = defineSchema(
    schema.string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$(?![\s\S])/u),
);

/** What a feature grants: access alone, a fixed value, or usage of a meter up to a limit. */
export const FEATURE_KINDS = ["boolean", "static", "metered"] as const;

/** What a feature grants. */
export const FeatureKind = defineSchema(schema.enum(FEATURE_KINDS));
/** What a feature grants. */
export type FeatureKind = schema.Infer<typeof FeatureKind>;

/** When a metered feature's usage starts again: at each billing period of its source, or never. */
export const FeatureReset = defineSchema(schema.enum(["period", "never"]));
/** When a metered feature's usage starts again. */
export type FeatureReset = schema.Infer<typeof FeatureReset>;

/** The limit a metered feature's grant sets on its usage per period, unlimited without one. */
export const FeatureLimit = defineSchema(schema.number().int().min(0));

/** The identity of a feature across package renames and releases. */
export const FeatureReference = Object.assign(
    defineSchema(
        schema.object({
            /** The package declaring the feature. */
            packageId: PackageId,
            /** The package-local name. */
            name: FeatureName,
        }),
    ),
    {
        /** Key a feature by its package and name. */
        key(reference: FeatureReference): string {
            return `${reference.packageId}/${reference.name}`;
        },
    },
);
/** The identity of a feature. */
export type FeatureReference = schema.Infer<typeof FeatureReference>;

/** The fields of a feature declaration beside its kind. */
export const FeatureMetadata = defineSchema(
    schema.object({
        /** The package-local name. */
        name: FeatureName,
        /** What the feature grants. */
        description: schema.string().min(1),
    }),
);

/** A feature as authored. */
export type FeatureDefinition<Value extends schema.Schema = schema.Schema> = schema.Infer<
    typeof FeatureMetadata
> &
    (
        | {
              /** Access alone. */
              readonly kind: "boolean";
          }
        | {
              /** A fixed value each grant sets. */
              readonly kind: "static";
              /** The schema of the value a grant sets. */
              readonly value: Value;
          }
        | {
              /** Usage of a meter up to the limit a grant sets. */
              readonly kind: "metered";
              /** The meter whose events count as usage. */
              readonly meter: MeterReference;
              /** When the usage starts again. */
              readonly reset: FeatureReset;
          }
    );

/** A feature declaration: what a product grants to the accounts buying it. */
export class Feature<Value extends schema.Schema = schema.Schema> {
    /** The declaring package. */
    readonly package: Package;
    /** The name, description and kind. */
    readonly definition: FeatureDefinition<Value>;

    /** Hold a checked declaration. */
    constructor(owner: Package, definition: FeatureDefinition<Value>) {
        this.package = owner;
        this.definition = definition;
    }

    /** The identity products and entitlements refer to. */
    get reference(): FeatureReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Serialise the feature as its reference. */
    toJSON(): FeatureReference {
        return this.reference;
    }

    /** Require the value a product's grant sets: none for access alone, a valid value, or a limit if any. */
    requireGrant(value: JsonValue | null): void {
        // read the value each kind accepts
        const definition = this.definition;
        let isAccepted: boolean;
        // accept no value for access alone
        if (definition.kind === "boolean") {
            isAccepted = value === null;
        }
        // accept a value the feature's schema parses
        else if (definition.kind === "static") {
            isAccepted = definition.value.safeParse(value).success;
        }
        // accept a limit, or none for unlimited usage
        else {
            isAccepted = value === null || FeatureLimit.safeParse(value).success;
        }

        // refuse any other value
        if (!isAccepted) {
            throw new ServiceError("BAD_REQUEST", {
                message: `feature ${FeatureReference.key(this.reference)} does not accept the granted value`,
            });
        }
    }
}
