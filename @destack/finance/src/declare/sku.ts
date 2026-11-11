import { ModuleMetadata, PackageVersion } from "@destack/package";
import type { Meter } from "@destack/telemetry/meter";
import { Sku, SkuMetadata, type SkuDefinition } from "../sku/sku.ts";

/** Declare a SKU: a provider's list price per pricing unit of the usage a meter measures. */
export function defineSku(
    options: Omit<SkuDefinition, "meter"> & {
        /** The meter measuring its billing unit. */
        readonly meter: Meter;
    },
    module?: ModuleMetadata,
): Sku {
    // stamp the declaring package, and refer to the meter
    const owner = PackageVersion.parse(ModuleMetadata.require(module, "defineSku").package);

    return new Sku(owner, SkuMetadata.parse({ ...options, meter: options.meter.reference }));
}
