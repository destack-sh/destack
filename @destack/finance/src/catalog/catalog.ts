import { fromJsonSchema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import type { Package, PackageId } from "@destack/package";
import { type BuildReader, BuildCache } from "@destack/package/manifest";
import type { Registry } from "@destack/forge/client";
import { ServiceError } from "@destack/service/error";
import { Feature } from "../feature/feature.ts";
import { Meter } from "../meter/meter.ts";
import { FeatureDescription } from "../feature/description.ts";
import { MeterDescription } from "../meter/description.ts";
import { Sku } from "../sku/sku.ts";
import { SkuDescription } from "../sku/description.ts";
import { CatalogReference } from "./reference.ts";

/** The catalogs read from each build so far. */
const CATALOGS = new BuildCache((reader) => Catalog.read(reader));

/** The features, meters and SKUs a build declares, by key. */
export class Catalog {
    /** The package the build is of. */
    readonly package: Package;
    /** The declared features by key. */
    readonly #features: ReadonlyMap<string, Feature>;
    /** The declared meters by key. */
    readonly #meters: ReadonlyMap<string, Meter>;
    /** The declared SKUs by key. */
    readonly #skus: ReadonlyMap<string, Sku>;

    /** Hold a package's features, meters and SKUs. */
    constructor(
        owner: Package,
        features: readonly Feature[],
        meters: readonly Meter[],
        skus: readonly Sku[],
    ) {
        // key each declaration by its reference
        this.package = owner;
        this.#features = new Map(
            features.map((feature) => [CatalogReference.key(feature.reference), feature]),
        );
        this.#meters = new Map(
            meters.map((meter) => [CatalogReference.key(meter.reference), meter]),
        );
        this.#skus = new Map(skus.map((sku) => [CatalogReference.key(sku.reference), sku]));
    }

    /** Read each package's catalog from the release a registry's distribution tag names, each build's once. */
    static tagged(
        registry: Pick<Registry, "open">,
        tag: string,
    ): (packageId: PackageId) => Promise<Catalog> {
        return (packageId) => registry.open(packageId, tag).then((reader) => CATALOGS.read(reader));
    }

    /** Read each package's catalog once, keeping one release for a whole call. */
    static once(
        read: (packageId: PackageId) => Promise<Catalog>,
    ): (packageId: PackageId) => Promise<Catalog> {
        const opened = new Map<PackageId, Promise<Catalog>>();

        return (packageId) => {
            const catalog = opened.get(packageId) ?? read(packageId);
            opened.set(packageId, catalog);

            return catalog;
        };
    }

    /** Read the features, meters and SKUs a build declares. */
    static async read(reader: BuildReader): Promise<Catalog> {
        // read the three kinds of declarations
        const packageId = import.meta.destack.package.id;
        const [features, meters, skus] = await Promise.all([
            reader.declared(packageId, "feature", FeatureDescription),
            reader.declared(packageId, "meter", MeterDescription),
            reader.declared(packageId, "sku", SkuDescription),
        ]);

        // rebuild each feature's value schema from its JSON Schema
        const declared = features.map(({ description: { package: owner, ...definition } }) =>
            definition.kind === "static"
                ? new Feature(owner, { ...definition, value: fromJsonSchema(definition.value) })
                : new Feature(owner, definition),
        );

        return new Catalog(
            reader.manifest.package,
            declared,
            meters.map(
                ({ description: { package: owner, ...definition } }) =>
                    new Meter(owner, definition),
            ),
            skus.map(
                ({ description: { package: owner, ...definition } }) => new Sku(owner, definition),
            ),
        );
    }

    /** Find a declared feature and refuse an undeclared one. */
    feature(reference: CatalogReference): Feature {
        const feature = this.find(reference);
        if (feature === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `feature ${CatalogReference.key(reference)} is not declared`,
            });
        }

        return feature;
    }

    /** Find a declared feature, absent once a release no longer declares it. */
    find(reference: CatalogReference): Feature | undefined {
        return this.#features.get(CatalogReference.key(reference));
    }

    /** Find a declared meter and refuse an undeclared one. */
    meter(reference: CatalogReference): Meter {
        // look the meter up by its key
        const key = CatalogReference.key(reference);
        const meter = this.#meters.get(key);
        if (meter === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `meter ${key} is not declared` });
        }

        return meter;
    }

    /** Find a declared SKU and refuse an undeclared one. */
    sku(reference: CatalogReference): Sku {
        // look the SKU up by its key
        const key = CatalogReference.key(reference);
        const sku = this.#skus.get(key);
        if (sku === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `sku ${key} is not declared` });
        }

        return sku;
    }
}
