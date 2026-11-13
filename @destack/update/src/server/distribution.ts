import type { ObjectStore } from "@destack/object/server";
import type { ObjectReconciliation } from "@destack/object";
import type { Identifier } from "@destack/schema";
import { now } from "@destack/db";
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

/** The distributions machines host, recorded in their devices. */
export const DistributionHost = {
    /** Record a host's distribution in its device once, as the device. */
    async record(
        objects: Pick<ObjectStore, "database" | "mutate" | "clock" | "as" | "environment">,
        device: Identifier<"device">,
        host: DistributionHost,
    ): Promise<void> {
        // read the device's recorded distribution
        const [row] = await objects.database
            .select()
            .from(distribution.table)
            .where(distribution.inScope(device));
        const context = objects.as(distribution, objects.clock.now());

        // create it with the running release, keeping a recorded one
        if (row === undefined) {
            const call = distribution.call("create", {
                ...distribution.in(device),
                installed: host.distribution.options.running.version,
            });
            await objects.mutate(context, {
                id: RequestId.create(objects.environment),
                calls: [call],
            });
        }
    },
};

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
                        throw UpdateError.of(
                            "RELEASE_NOT_FOUND",
                            "staged Destack release not found",
                        );
                    }

                    return staged.release.version;
                },
                handle: (context) =>
                    context.update({ staged: context.prepared, status: "activating" }),
                // restart into it once the activation is recorded
                confirm: async () => {
                    requireHost(host).activate();
                },
            },
        },
        // stage a recorded distribution never checked, then again once its last check is a day old
        controllers:
            host === undefined
                ? []
                : [
                      {
                          pending: { checkedAt: { isNull: true } },
                          reconcile: stage,
                      },
                      {
                          name: "daily",
                          pending: {
                              checkedAt: {
                                  lte: now.minus({ milliseconds: STAGE_INTERVAL_MILLISECONDS }),
                              },
                          },
                          reconcile: stage,
                      },
                  ],
    });
}

/** Stage the latest release of each pending distribution. */
async function stage(reconciliation: ObjectReconciliation<typeof distribution>): Promise<void> {
    await reconciliation.mutate(
        reconciliation.rows.map((row) => distribution.call("stage", distribution.target(row))),
    );
}

/** Read the host's distribution. */
function requireHost(host: DistributionHost | undefined): DistributionHost {
    if (host === undefined) {
        throw UpdateError.of(
            "DEVELOPMENT_BUILD_BLOCKED",
            "cannot update: development builds cannot take published updates",
        );
    }

    return host;
}
