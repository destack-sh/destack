import { Package } from "@destack/package";
import { defineSchema, type schema } from "@destack/schema";
import { SkuMetadata } from "./sku.ts";

/** A SKU declaration as manifests describe it. */
export const SkuDescription = defineSchema(
    SkuMetadata.extend({
        /** The declaring package. */
        package: Package,
    }),
);
/** A SKU declaration as manifests describe it. */
export type SkuDescription = schema.Infer<typeof SkuDescription>;
