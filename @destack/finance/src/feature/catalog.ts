import { fromJsonSchema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import type { Package } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { ServiceError } from "@destack/service/error";
import { Feature, FeatureReference } from "./feature.ts";
import { Meter, MeterReference } from "../meter/meter.ts";
import { FeatureDescription } from "./description.ts";
import { MeterDescription } from "../meter/description.ts";

/** The features and meters a build declares, by key. */
export class FeatureCatalog {
    /** The package the build is of. */
    readonly package: Package;
    /** The declared features by key. */
    readonly #features: ReadonlyMap<string, Feature>;
    /** The declared meters by key. */
    readonly #meters: ReadonlyMap<string, Meter>;

    /** Hold a package's features and meters. */
    constructor(owner: Package, features: readonly Feature[], meters: readonly Meter[]) {
        this.package = owner;
        this.#features = new Map(
            features.map((feature) => [FeatureReference.key(feature.reference), feature]),
        );
        this.#meters = new Map(meters.map((meter) => [MeterReference.key(meter.reference), meter]));
    }

    /** Read the features and meters a build declares. */
    static async read(reader: BuildReader): Promise<FeatureCatalog> {
        // read both kinds of declarations
        const packageId = import.meta.destack.package.id;
        const [features, meters] = await Promise.all([
            reader.declared(packageId, "feature", FeatureDescription),
            reader.declared(packageId, "meter", MeterDescription),
        ]);

        // rebuild each feature's value schema from its JSON Schema
        const declared = features.map(({ description: { package: owner, ...definition } }) =>
            definition.kind === "static"
                ? new Feature(owner, { ...definition, value: fromJsonSchema(definition.value) })
                : new Feature(owner, definition),
        );

        return new FeatureCatalog(
            reader.manifest.package,
            declared,
            meters.map(
                ({ description: { package: owner, ...definition } }) =>
                    new Meter(owner, definition),
            ),
        );
    }

    /** Find a declared feature and refuse an undeclared one. */
    feature(reference: FeatureReference): Feature {
        // look the feature up by its key
        const key = FeatureReference.key(reference);
        const feature = this.#features.get(key);
        if (feature === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `feature ${key} is not declared` });
        }

        return feature;
    }

    /** Find a declared meter and refuse an undeclared one. */
    meter(reference: MeterReference): Meter {
        // look the meter up by its key
        const key = MeterReference.key(reference);
        const meter = this.#meters.get(key);
        if (meter === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `meter ${key} is not declared` });
        }

        return meter;
    }
}
