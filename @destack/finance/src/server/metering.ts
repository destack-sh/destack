import type { Call } from "@destack/sync";
import { AccessContext } from "@destack/access";
import type { MethodContext } from "@destack/object";
import type { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { Identifier } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import { release } from "@destack/forge/object";
import { ReleaseReader } from "../release/reader.ts";
import { CatalogReference } from "../catalog/reference.ts";
import { type Publisher, Usage } from "../object/index.ts";
import { Rating } from "./rating.ts";

/** The instruments capturing the uses their declarations or terms refuse. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The rating of each paying account's usage against the packages' current releases. */
export class Metering {
    /** The distribution tag of each package's current release, such as latest or dev. */
    readonly tag: string;
    /** Who publishes each package's SKUs to the home's buyers. */
    readonly publisher: Publisher;
    /** Open a package's published build by its manifest's digest. */
    readonly #open: (packageId: PackageId, manifest: string) => Promise<BuildReader>;

    /** The declarations of each release read so far, by package and version. */
    readonly #released = new Map<string, Awaited<ReturnType<typeof ReleaseReader.read>>>();

    /** Meter usage against the packages' current builds, and their publishers. */
    constructor(
        tag: string,
        publisher: Publisher,
        open: (packageId: PackageId, manifest: string) => Promise<BuildReader>,
    ) {
        this.tag = tag;
        this.publisher = publisher;
        this.#open = open;
    }

    /** Read the releases the tag points at for one call, parsing each release once. */
    reader(source: Pick<MethodContext, "query">, caller: AccessContext, scope: string): ReleaseReader {
        return new ReleaseReader(async (packageId) => {
            // resolve the version the tag points at
            const resolved = await source.query(
                caller,
                release.call("resolve", {
                    ...release.in(scope),
                    parentId: packageId,
                    release: this.tag,
                }),
            );

            // read the declarations of the release's build the first time
            const key = `${packageId}@${resolved.version}`;
            const declared =
                this.#released.get(key) ??
                (await ReleaseReader.read(await this.#open(packageId, resolved.manifest)));
            this.#released.set(key, declared);

            return declared;
        });
    }

    /** Rate a page of a paying account's usage in commit order, reporting uses that never rate. */
    async rate(
        payer: Identifier<"account">,
        page: readonly Usage[],
        context: MethodContext,
    ): Promise<void> {
        // keep the page's uses their declarations accept, reporting the ones they refuse
        const releases = this.reader(context, context.self, context.scope);
        const events = Usage.series(context);
        const accepted: Usage[] = [];
        for (const used of page) {
            if (await this.#accepts(used, releases)) {
                accepted.push(used);
            }
        }

        // rate the uses of SKUs into their lines, reporting the ones their terms refuse
        const rating = await Rating.open(payer, releases, this.publisher, context);
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
            const meter = await releases.meter(reference);
            rated.push(
                ...(await rating.rateMeter(meter, (period) =>
                    Usage.read(payer, meter, period, context.now, events),
                )),
            );
        }
        if (rated.length > 0) {
            await context.mutate(context.self, rated);
        }
    }

    /** Report whether a use's declarations accept it, reporting one they refuse. */
    async #accepts(used: Usage, releases: ReleaseReader): Promise<boolean> {
        try {
            await Usage.require(used, releases);

            return true;
        } catch (error) {
            // report a use that never passes, and retry the page for one that may
            instruments.captureException(Usage.refusal(error));

            return false;
        }
    }
}
