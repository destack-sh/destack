import { fromJsonSchema, schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import { graph, type Package, type PackageId } from "@destack/package";
import { CallError } from "@destack/runtime/error";
import { Feature } from "../feature/feature.ts";
import { Meter, MeterDescription } from "@destack/observability/meter";
import observabilityPackage from "@destack/observability/package";
import { FeatureDescription } from "../feature/description.ts";
import { Sku } from "../sku/sku.ts";
import { SkuDescription } from "../sku/description.ts";
import { CatalogReference } from "./reference.ts";

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

    /** Create a package's catalog of features, meters and SKUs. */
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

    /** Read each package's catalog once, reading one release for a whole call. */
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

    /** Read the features, meters and SKUs among a release's declarations. */
    static of(owner: Package, declarations: readonly graph.Declaration[]): Catalog {
        // read the declarations of each kind finance or observability declares
        const packageId = import.meta.destack.package.id;

        // rebuild each feature's value schema from its JSON Schema
        const features = Catalog.#declared(
            declarations,
            "feature",
            packageId,
            FeatureDescription,
        ).map(({ package: declaring, ...definition }) =>
            definition.kind === "static"
                ? new Feature(declaring, { ...definition, value: fromJsonSchema(definition.value) })
                : new Feature(declaring, definition),
        );
        const meters = Catalog.#declared(
            declarations,
            "meter",
            observabilityPackage.id,
            MeterDescription,
        ).map(({ package: declaring, ...definition }) => new Meter(declaring, definition));
        const skus = Catalog.#declared(declarations, "sku", packageId, SkuDescription).map(
            ({ package: declaring, ...definition }) => new Sku(declaring, definition),
        );

        return new Catalog(owner, features, meters, skus);
    }

    /** Read the descriptions of one kind a package declares among a release's declarations. */
    static #declared<Item extends schema.Schema>(
        declarations: readonly graph.Declaration[],
        kind: string,
        owning: PackageId,
        item: Item,
    ): schema.Output<Item>[] {
        return declarations
            .filter((entry) => entry.kind === kind && entry.package === owning)
            .map((entry) => item.parse(entry.description));
    }

    /** Find a declared feature and refuse an undeclared one. */
    feature(reference: CatalogReference): Feature {
        const feature = this.find(reference);
        if (feature === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `feature ${CatalogReference.key(reference)} is undeclared`,
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
            throw new CallError("NOT_FOUND", { message: `meter ${key} is undeclared` });
        }

        return meter;
    }

    /** Find a declared SKU and refuse an undeclared one. */
    sku(reference: CatalogReference): Sku {
        // look the SKU up by its key
        const key = CatalogReference.key(reference);
        const sku = this.#skus.get(key);
        if (sku === undefined) {
            throw new CallError("NOT_FOUND", { message: `sku ${key} is undeclared` });
        }

        return sku;
    }
}
