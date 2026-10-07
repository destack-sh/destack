import { ModuleMetadata, Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import {
    ALLOWANCE_KINDS,
    Feature,
    FeatureMetadata,
    FeatureReset,
    type FeatureDefinition,
} from "../feature/feature.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** Declare a typed feature that products grant. */
export function defineFeature<Value extends schema.Schema>(
    definition: FeatureDefinition<Value>,
    module?: ModuleMetadata,
): Feature<Value> {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineFeature").package);

    // validate the name, the description and the allowance, each resource allowed by its kind of feature
    FeatureMetadata.parse({
        name: definition.name,
        description: definition.description,
        ...(definition.allowance === undefined ? {} : { allowance: definition.allowance }),
    });
    if (
        definition.allowance !== undefined &&
        ALLOWANCE_KINDS[definition.allowance] !== definition.kind
    ) {
        throw new TypeError(
            `feature ${definition.name} allows ${definition.allowance} as no ${ALLOWANCE_KINDS[definition.allowance]} feature`,
        );
    }

    // require a declarative value schema, or meters and their reset
    if (definition.kind === "static") {
        defineSchema(definition.value);
    } else if (definition.kind === "metered") {
        const meters = schema.array(CatalogReference).min(1).parse(definition.meters);
        FeatureReset.parse(definition.reset);

        // count each meter once
        const keys = new Set(meters.map((meter) => CatalogReference.key(meter)));
        if (keys.size < meters.length) {
            throw new TypeError(`feature ${definition.name} counts a meter twice`);
        }
    }

    return new Feature(owner, definition);
}
