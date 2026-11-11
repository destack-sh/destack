import { DeclarationReference } from "@destack/package/declare";
import { ModuleMetadata, PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import {
    RESOURCE_KINDS,
    Feature,
    FeatureMetadata,
    FeatureReset,
    type FeatureDefinition,
    type FeatureOptions,
} from "../feature/feature.ts";

/** Declare a typed feature that products grant. */
export function defineFeature<Value extends schema.Schema>(
    options: FeatureOptions<Value>,
    module?: ModuleMetadata,
): Feature<Value> {
    // stamp the declaring package, and refer to a metered feature's meters
    const owner = PackageVersion.parse(ModuleMetadata.require(module, "defineFeature").package);
    const definition: FeatureDefinition<Value> =
        options.kind === "metered"
            ? { ...options, meters: options.meters.map((meter) => meter.reference) }
            : options;

    // validate the name, the description and the resource, each allowed by its kind of feature
    FeatureMetadata.parse({
        name: definition.name,
        description: definition.description,
        ...(definition.resource === undefined ? {} : { resource: definition.resource }),
    });
    if (
        definition.resource !== undefined &&
        RESOURCE_KINDS[definition.resource] !== definition.kind
    ) {
        throw new TypeError(
            `feature '${definition.name}' allows '${definition.resource}', which needs a '${RESOURCE_KINDS[definition.resource]}' feature`,
        );
    }

    // require a metered feature's meters and reset
    if (definition.kind === "metered") {
        const meters = schema.array(DeclarationReference).min(1).parse(definition.meters);
        FeatureReset.parse(definition.reset);

        // count each meter once
        const keys = new Set(meters.map((meter) => DeclarationReference.key(meter)));
        if (keys.size < meters.length) {
            throw new TypeError(`feature '${definition.name}' counts a meter twice`);
        }
    }

    return new Feature(owner, definition);
}
