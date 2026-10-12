import { ProviderCode, Tenancy } from "@destack/account/object";
import type { Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { Currency, DecimalAmount } from "../rate/amount.ts";
import { CatalogName, CatalogReference } from "../catalog/reference.ts";

/** The FOCUS ServiceCategories of the services SKUs price. */
export const SERVICE_CATEGORIES = ["Compute", "Databases", "Storage", "Networking"] as const;

/** The FOCUS ServiceCategory of the service a SKU prices. */
export const ServiceCategory = defineSchema(schema.enum(SERVICE_CATEGORIES));
/** The FOCUS ServiceCategory of a service. */
export type ServiceCategory = schema.Infer<typeof ServiceCategory>;

/** The fields of a SKU declaration. */
export const SkuMetadata = defineSchema(
    schema.object({
        /** The package-local name. */
        name: CatalogName,
        /** What the SKU prices. */
        description: schema.string().min(1),
        /** The provider selling it. */
        provider: ProviderCode,
        /** The provider's region the price applies in, absent for a price every region shares. */
        region: schema.string().min(1).exactOptional(),
        /** The provider's service, the FOCUS ServiceName, such as Durable Objects. */
        service: schema.string().min(1),
        /** The FOCUS ServiceCategory. */
        category: ServiceCategory,
        /** The tenancy of the priced machines, absent for a SKU every tenancy uses. */
        tenancy: Tenancy.exclude(["own"]).exactOptional(),
        /** The meter measuring its billing unit. */
        meter: CatalogReference,
        /** The FOCUS PricingUnit, such as GB-month. */
        pricingUnit: schema.string().min(1),
        /** The meter's units in one pricing unit, such as 10^9 byte-months in a GB-month. */
        unitSize: schema.number().positive(),
        /** The FOCUS PricingCurrency of the list prices. */
        currency: Currency,
        /** The list price per pricing unit, in fractional minor units. */
        unitAmount: DecimalAmount,
        /** The most pricing units one resource pays for per billing period, absent if uncapped. */
        maximumQuantity: schema.number().positive().exactOptional(),
        /** The page publishing the price. */
        source: schema.url(),
        /** The day the price was read at its source, as YYYY-MM-DD. */
        observedAt: schema.string().regex(/^\d{4}-\d{2}-\d{2}$/u),
    }),
);
/** The fields of a SKU declaration. */
export type SkuDefinition = schema.Infer<typeof SkuMetadata>;

/** A SKU: a provider's list price per pricing unit of a service in a region, as in FOCUS. */
export class Sku {
    /** The declaring package, whose publishing account resells the SKU. */
    readonly package: Package;
    /** The provider, service, meter, unit and prices. */
    readonly definition: SkuDefinition;

    /** Hold a checked declaration. */
    constructor(owner: Package, definition: SkuDefinition) {
        this.package = owner;
        this.definition = definition;
    }

    /** The identity meter events and charges refer to. */
    get reference(): CatalogReference {
        return { packageId: this.package.id, name: this.definition.name };
    }

    /** The package-local name. */
    get name(): string {
        return this.definition.name;
    }

    /** Serialise the SKU as its reference. */
    toJSON(): CatalogReference {
        return this.reference;
    }

    /** List the SKUs on a meter. */
    static on(skus: readonly Sku[], meter: CatalogReference): readonly Sku[] {
        const key = CatalogReference.key(meter);

        return skus.filter((sku) => CatalogReference.key(sku.definition.meter) === key);
    }
}
