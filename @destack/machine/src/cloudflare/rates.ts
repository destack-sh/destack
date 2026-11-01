import { Sku, SkuMetadata } from "@destack/finance";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";

/** What a cloud machine's Worker rates its spaces' use by, as the deployment binds it. */
const machineRatesSchema = schema.object({
    /** The SKUs that rate each meter on the machine, one per meter. */
    skus: schema.array(schema.object({ package: Package, definition: SkuMetadata })),
    /** The memory each active Durable Object is billed at, in gigabytes. */
    gigabytes: schema.number().positive(),
});
/** What a cloud machine's Worker rates its spaces' use by. */
export type RateCard = schema.Infer<typeof machineRatesSchema>;

/** What a cloud machine's Worker rates its spaces' use by, read from their binding. */
export const RateCard = Object.assign(machineRatesSchema, {
    /** Read the rates of a binding's JSON, with each SKU as its package declares it. */
    read(binding: string): Omit<RateCard, "skus"> & { readonly skus: readonly Sku[] } {
        const rates = machineRatesSchema.parse(JSON.parse(binding));
        const skus = rates.skus.map((entry) => new Sku(entry.package, entry.definition));

        return { ...rates, skus };
    },
});
