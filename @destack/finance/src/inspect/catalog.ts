import { fromJsonSchema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import type { BuildReader } from "@destack/package/manifest";
import { ServiceError } from "@destack/service/error";
import { Feature, FeatureReference } from "../feature/feature.ts";
import { Meter, MeterReference } from "../meter/meter.ts";
import { FeatureDescription } from "./feature.ts";
import { MeterDescription } from "./meter.ts";

/** The catalog read from each build so far. */
const READ = new WeakMap<BuildReader, Promise<FeatureCatalog>>();

/** The features and meters a build declares, by key. */
export class FeatureCatalog {
    /** The declared features by key. */
    readonly #features: ReadonlyMap<string, Feature>;
    /** The declared meters by key. */
    readonly #meters: ReadonlyMap<string, Meter>;

    /** Hold a build's features and meters. */
    constructor(features: readonly Feature[], meters: readonly Meter[]) {
        this.#features = new Map(
            features.map((feature) => [FeatureReference.key(feature.reference), feature]),
        );
        this.#meters = new Map(meters.map((meter) => [MeterReference.key(meter.reference), meter]));
    }

    /** Read the features and meters a build declares once per build. */
    static read(reader: BuildReader): Promise<FeatureCatalog> {
        // build each release's validators once
        let catalog = READ.get(reader);
        if (catalog === undefined) {
            catalog = FeatureCatalog.#read(reader);
            READ.set(reader, catalog);

            // read again after a failed read
            catalog.catch(() => READ.delete(reader));
        }

        return catalog;
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

    /** Read a build's feature and meter descriptions back into declarations. */
    static async #read(reader: BuildReader): Promise<FeatureCatalog> {
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
            declared,
            meters.map(
                ({ description: { package: owner, ...definition } }) =>
                    new Meter(owner, definition),
            ),
        );
    }
}
