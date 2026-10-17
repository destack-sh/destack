import type { Call } from "@destack/sync";
import { AccessContext } from "@destack/access";
import type { MethodContext } from "@destack/object";
import type { PackageId } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import { release } from "@destack/forge/object";
import { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";
import { Customer, type Publisher, Usage } from "../object/index.ts";
import { Rating } from "./rating.ts";

/** The instruments capturing the uses their declarations or terms refuse. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The rating of each paying account's usage against the packages' catalogs. */
export class Metering {
    /** The distribution tag of each package's current release, such as latest or dev. */
    readonly tag: string;
    /** Who publishes each package's SKUs to the home's buyers. */
    readonly publisher: Publisher;

    /** The catalog of each release read so far, by package and version. */
    readonly #released = new Map<string, Catalog>();

    /** Meter usage against the packages' current catalogs and publishers. */
    constructor(tag: string, publisher: Publisher) {
        this.tag = tag;
        this.publisher = publisher;
    }

    /** Resolve each package's release at the tag once per call, parsing each release once. */
    catalog(
        source: Pick<MethodContext, "query">,
        caller: AccessContext,
        scope: string,
    ): (packageId: PackageId) => Promise<Catalog> {
        return Catalog.once(async (packageId) => {
            // resolve the version the tag points at
            const resolved = await source.query(
                caller,
                release.call("resolve", {
                    ...release.in(scope),
                    parentId: packageId,
                    release: this.tag,
                }),
            );

            // read the release's catalog the first time
            const key = `${packageId}@${resolved.version}`;
            const owner = {
                id: packageId,
                name: resolved.metadata.name,
                version: resolved.version,
            };
            const catalog = this.#released.get(key) ?? Catalog.of(owner, resolved.declarations);
            this.#released.set(key, catalog);

            return catalog;
        });
    }

    /** Rate a page of a paying account's usage in commit order, reporting uses that never rate. */
    async rate(
        payer: Identifier<"account">,
        page: readonly Usage[],
        context: MethodContext,
    ): Promise<void> {
        // keep the page's uses their declarations accept, reporting the ones they refuse
        const catalog = this.catalog(context, context.self, context.scope);
        const events = Usage.series(context);
        const accepted: Usage[] = [];
        for (const used of page) {
            if (await this.#accepts(used, catalog)) {
                accepted.push(used);
            }
        }

        // rate the uses of SKUs into their lines, reporting the ones their terms refuse
        const rating = await Rating.open(payer, catalog, this.publisher, context);
        const sold = accepted.filter((used) => used.keys.sku !== null);
        for (const refused of await rating.rateSkus(sold)) {
            instruments.captureException(refused);
        }

        // rate the period's usage of each seller meter used into its metered prices, in one write
        const metered = new Set(
            accepted.filter((used) => used.keys.sku === null).map((used) => used.keys.meter),
        );
        const rated: Call[] = [];
        for (const key of metered) {
            const reference = CatalogReference.of(key);
            const meter = (await catalog(reference.packageId)).meter(reference);
            rated.push(
                ...(await rating.rateMeter(meter, (period) =>
                    Usage.read(payer, meter, period, context.now, events),
                )),
            );
        }
        if (rated.length > 0) {
            await context.mutate(context.self, rated);
        }

        // derive the entitlements again from the counted usage
        await Customer.entitle(payer, catalog, context);
    }

    /** Report whether a use's declarations accept it, reporting one they refuse. */
    async #accepts(
        used: Usage,
        catalog: (packageId: PackageId) => Promise<Catalog>,
    ): Promise<boolean> {
        try {
            await Usage.require(used, catalog);

            return true;
        } catch (error) {
            // report a use that never passes, and retry the page for one that may
            instruments.captureException(Usage.refusal(error));

            return false;
        }
    }
}
