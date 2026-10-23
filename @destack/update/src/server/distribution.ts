import type { ObjectStore } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import type { ReconcileController } from "@destack/db";
import type { Distribution } from "../distribution/index.ts";
import { UpdateError } from "../error/index.ts";
import { distribution } from "../object/index.ts";
import { RequestId } from "@destack/sync";
import type {} from "@destack/package/import-meta";

/** How often the distribution stages the feed's latest release: daily, Sparkle's default. */
const STAGE_INTERVAL_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The distribution a machine hosts: its installed releases and the restart into a staged one. */
export interface DistributionHost {
    /** The installed distribution. */
    readonly distribution: Distribution;
    /** Hand activation to a process that outlives the machine. */
    readonly activate: () => void;
}

/** Stage releases through the installed distribution, and hand activation to the host. */
export function serveDistributions(host: DistributionHost | undefined) {
    return distribution.implement({
        methods: {
            stage: {
                prepare: async () => {
                    // stage a newer release, and read the installed one after
                    const installed = requireHost(host).distribution;
                    const staged = await installed.stage();

                    return {
                        installed: (await installed.installed()).version,
                        staged: staged?.release.version ?? null,
                    };
                },
                handle: (context) =>
                    context.update({
                        installed: context.prepared.installed,
                        staged: context.prepared.staged,
                        status: context.prepared.staged === null ? "current" : "staged",
                        checkedAt: context.now,
                    }),
            },
            activate: {
                prepare: async () => {
                    // require a staged release
                    const staged = await requireHost(host).distribution.staged();
                    if (staged === undefined) {
                        throw UpdateError.of("RELEASE_NOT_FOUND", "no Destack release is staged");
                    }

                    return staged.release.version;
                },
                handle: (context) =>
                    context.update({ staged: context.prepared, status: "activating" }),
                settle: async (_call, settled) => {
                    // restart into it once the activation is recorded
                    if (settled.status === "committed") {
                        requireHost(host).activate();
                    }
                },
            },
        },
    });
}

/** Keep this machine's distribution row and stage the feed's latest release daily. */
export class DistributionController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "distribution";
    /** The store keeping the distribution row. */
    readonly #store: Pick<ObjectStore, "database" | "serving" | "mutate">;
    /** This machine, the scope of its distribution. */
    readonly #machineId: Identifier<"machine">;
    /** The installed distribution. */
    readonly #host: DistributionHost;

    /** Control the distribution of a machine. */
    constructor(
        store: Pick<ObjectStore, "database" | "serving" | "mutate">,
        machineId: Identifier<"machine">,
        host: DistributionHost,
    ) {
        this.#store = store;
        this.#machineId = machineId;
        this.#host = host;
    }

    /** List this machine as the one key. */
    async list(): Promise<readonly string[]> {
        return [this.#machineId];
    }

    /** Record the installed distribution once, then stage the latest release each day. */
    async reconcile(): Promise<number> {
        // record the distribution on its first start
        const now = Date.now();
        const scope = this.#machineId;
        const [found] = await this.#store.database
            .select()
            .from(distribution.table)
            .where(distribution.inScope(scope));
        const options = this.#host.distribution.options;
        const [recorded] =
            found === undefined
                ? await this.#store.mutate(
                      this.#store.serving(distribution),
                      {
                          id: RequestId.create(now),
                          calls: [
                              distribution.call("create", {
                                  ...distribution.in(scope),
                                  installed: options.running.version,
                              }),
                          ],
                      },
                      { component: this.name },
                  )
                : [found];
        if (recorded === undefined) {
            throw new TypeError("recording the distribution returned nothing");
        }

        // stage the latest release
        await this.#store.mutate(
            this.#store.serving(distribution),
            {
                id: RequestId.create(now),
                calls: [distribution.call("stage", distribution.target(recorded))],
            },
            { component: this.name },
        );

        return STAGE_INTERVAL_MILLISECONDS;
    }
}

/** Read the host's distribution. */
function requireHost(host: DistributionHost | undefined): DistributionHost {
    if (host === undefined) {
        throw UpdateError.of(
            "DEVELOPMENT_BUILD",
            "development builds cannot take published updates",
        );
    }

    return host;
}
