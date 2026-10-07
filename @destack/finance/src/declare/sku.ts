import { ModuleMetadata, Package } from "@destack/package";
import { Sku, SkuMetadata, type SkuDefinition } from "../sku/sku.ts";

/** Declare a SKU: a provider's list price per pricing unit of the usage a meter measures. */
export function defineSku(definition: SkuDefinition, module?: ModuleMetadata): Sku {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineSku").package);

    return new Sku(owner, SkuMetadata.parse(definition));
}
