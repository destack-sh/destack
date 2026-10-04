import { ModuleMetadata, Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import {
    Feature,
    FeatureMetadata,
    FeatureReset,
    type FeatureDefinition,
} from "../feature/feature.ts";
import { MeterReference } from "../meter/meter.ts";

/** Declare a typed feature that products grant. */
export function defineFeature<Value extends schema.Schema>(
    definition: FeatureDefinition<Value>,
    module?: ModuleMetadata,
): Feature<Value> {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineFeature").package);

    // validate the name and description
    FeatureMetadata.parse({ name: definition.name, description: definition.description });

    // require a declarative value schema, or a meter and its reset
    if (definition.kind === "static") {
        defineSchema(definition.value);
    } else if (definition.kind === "metered") {
        MeterReference.parse(definition.meter);
        FeatureReset.parse(definition.reset);
    }

    return new Feature(owner, definition);
}
