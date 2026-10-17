import {
    ENTITLEMENT_RESOURCES,
    type EntitlementResource,
    FEATURE_KINDS,
} from "@destack/account/object";
import { PackageId, type Package } from "@destack/package";
import { defineSchema, schema, type JsonValue } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { CatalogName, CatalogReference } from "../catalog/reference.ts";

/** What a feature grants. */
export const FeatureKind = defineSchema(schema.enum(FEATURE_KINDS));
/** What a feature grants. */
export type FeatureKind = schema.Infer<typeof FeatureKind>;

/** The kind of feature allowing each platform resource. */
export const RESOURCE_KINDS: Readonly<Record<EntitlementResource, FeatureKind>> = {
    database: "metered",
    storage: "metered",
    compute: "metered",
    network: "metered",
    tenancy: "static",
    domain: "boolean",
};

/** When a metered feature's usage starts again: at each billing period of its source, or never. */
export const FeatureReset = defineSchema(schema.enum(["period", "never"]));
/** When a metered feature's usage starts again. */
export type FeatureReset = schema.Infer<typeof FeatureReset>;

/** The limit a metered feature's grant sets on its usage per period, unlimited without one. */
export const FeatureLimit = defineSchema(schema.number().int().min(0));

/** The fields of a feature declaration beside its kind. */
export const FeatureMetadata = defineSchema(
    schema.object({
        /** The package-local name. */
        name: CatalogName,
        /** What the feature grants. */
        description: schema.string().min(1),
        /** The platform resource the feature's grant allows, absent for a package's own feature. */
        resource: schema.enum(ENTITLEMENT_RESOURCES).exactOptional(),
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
              /** Usage of meters up to the limit a grant sets. */
              readonly kind: "metered";
              /** The meters of one unit whose events count together as usage. */
              readonly meters: readonly CatalogReference[];
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

    /** Create a feature from a checked declaration. */
    constructor(owner: Package, definition: FeatureDefinition<Value>) {
        this.package = owner;
        this.definition = definition;
    }

    /** The identity products and entitlements refer to. */
    get reference(): CatalogReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Serialise the feature as its reference. */
    toJSON(): CatalogReference {
        return this.reference;
    }

    /** Grant the feature at a value or limit as a product declares it, refusing invalid values. */
    grant(value: JsonValue | null): {
        readonly packageId: PackageId;
        readonly feature: string;
        readonly value: JsonValue | null;
    } {
        this.requireGrant(value);

        return { packageId: this.package.id, feature: this.name, value };
    }

    /** Require the value a product's grant sets: none for access, a valid value, or a limit. */
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
            throw new CallError("BAD_REQUEST", {
                message: `feature ${CatalogReference.key(this.reference)} refuses the granted value`,
            });
        }
    }
}
