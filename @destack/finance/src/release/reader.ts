import { DeclarationReference } from "@destack/package/declare";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { Meter } from "@destack/observability/meter";
import { CallError } from "@destack/object";
import { Feature } from "../feature/feature.ts";
import { Sku } from "../sku/sku.ts";

/** Read the features, meters and SKUs packages declare, in the releases one call resolves. */
export class ReleaseReader {
    /** Read the declarations of a package's current release. */
    readonly #read: (
        packageId: PackageId,
    ) => Promise<Awaited<ReturnType<typeof ReleaseReader.read>>>;
    /** The declarations read so far, by package, so one call reads one release of each. */
    readonly #releases = new Map<PackageId, ReturnType<ReleaseReader["release"]>>();

    /** Read each package's current release once, through a read of its declarations. */
    constructor(
        read: (packageId: PackageId) => Promise<Awaited<ReturnType<typeof ReleaseReader.read>>>,
    ) {
        this.#read = read;
    }

    /** Read the features, meters and SKUs a build declares. */
    static async read(build: Pick<BuildReader, "declared">) {
        const [features, meters, skus] = await Promise.all([
            Feature.list(build),
            Meter.list(build),
            Sku.list(build),
        ]);

        return { features, meters, skus };
    }

    /** Find a declared feature and refuse an undeclared one. */
    async feature(reference: DeclarationReference): Promise<Feature> {
        const feature = await this.findFeature(reference);
        if (feature === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `feature ${DeclarationReference.key(reference)} is undeclared`,
            });
        }

        return feature;
    }

    /** Find a declared feature, absent once a release no longer declares it. */
    async findFeature(reference: DeclarationReference): Promise<Feature | undefined> {
        const { features } = await this.release(reference.packageId);

        return features.find((each) => ReleaseReader.#isAt(each, reference));
    }

    /** Find a declared meter and refuse an undeclared one. */
    async meter(reference: DeclarationReference): Promise<Meter> {
        // look the meter up in its package's release
        const { meters } = await this.release(reference.packageId);
        const meter = meters.find((each) => ReleaseReader.#isAt(each, reference));
        if (meter === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `meter ${DeclarationReference.key(reference)} is undeclared`,
            });
        }

        return meter;
    }

    /** Find a declared SKU and refuse an undeclared one. */
    async sku(reference: DeclarationReference): Promise<Sku> {
        // look the SKU up in its package's release
        const { skus } = await this.release(reference.packageId);
        const sku = skus.find((each) => ReleaseReader.#isAt(each, reference));
        if (sku === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `sku ${DeclarationReference.key(reference)} is undeclared`,
            });
        }

        return sku;
    }

    /** Read a package's current release, once for the reader's call. */
    release(packageId: PackageId): Promise<Awaited<ReturnType<typeof ReleaseReader.read>>> {
        const read = this.#releases.get(packageId) ?? this.#read(packageId);
        this.#releases.set(packageId, read);

        return read;
    }

    /** Report whether a declaration is the one a reference points at. */
    static #isAt(
        declared: { readonly reference: DeclarationReference },
        reference: DeclarationReference,
    ): boolean {
        return DeclarationReference.key(declared.reference) === DeclarationReference.key(reference);
    }
}
