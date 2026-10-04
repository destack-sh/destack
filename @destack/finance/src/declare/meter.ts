import { ModuleMetadata, Package } from "@destack/package";
import { Meter, MeterMetadata, type MeterDefinition } from "../meter/meter.ts";

/** Declare a meter whose events count as the usage of metered features. */
export function defineMeter(definition: MeterDefinition, module?: ModuleMetadata): Meter {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineMeter").package);

    return new Meter(owner, MeterMetadata.parse(definition));
}
