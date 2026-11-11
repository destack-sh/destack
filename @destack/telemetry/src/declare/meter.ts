import { ModuleMetadata, PackageVersion } from "@destack/package";
import { Meter, MeterMetadata, type MeterDefinition } from "../meter/meter.ts";

/** Declare a meter: what its events count and how a period's events fold into usage. */
export function defineMeter(definition: MeterDefinition, module?: ModuleMetadata): Meter {
    // stamp the declaring package
    const owner = PackageVersion.parse(ModuleMetadata.require(module, "defineMeter").package);

    return new Meter(owner, MeterMetadata.parse(definition));
}
