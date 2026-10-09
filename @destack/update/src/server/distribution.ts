import type { ObjectServer, Extension } from "@destack/object/server";
import { distributionService } from "../service/index.ts";
import type { Identifier } from "@destack/schema";
import type { Controller } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import type { Distribution } from "../distribution/index.ts";
import { distribution } from "../object/index.ts";

/** How often the distribution stages the channel's latest release: daily, as Sparkle checks by default. */
const STAGE_INTERVAL_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The distribution a machine hosts: the installed releases, and the restart activating a staged one. */
export interface DistributionHost {
    /** The installed distribution. */
    readonly distribution: Distribution;
    /** Hand activation to a process outliving the machine, which stops it, switches releases and starts it again. */
    activate(): void;
}

/** Serve a machine's distribution in the machine's own scope: staging and activating its releases, and recording and staging them daily where it runs an installed distribution. */
export function implementDistributions(options: {
    /** The machine, the scope of its distribution. */
    readonly machine: Identifier<"machine">;
    /** The installed distribution, absent for a development build. */
    readonly host?: DistributionHost;
}): Extension {
    const { host } = options;

    return {
        service: distributionService,
        objects: { distribution: serveDistributions(host) },
        serve: (server) => ({
            controllers:
                host === undefined
                    ? []
                    : [new DistributionController(server, options.machine, host)],
        }),
    };
}

/** Stage releases through the installed distribution, and hand activation to the host. */
export function serveDistributions(host: DistributionHost | undefined) {
    return distribution.handle({
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
            handler: (call) =>
                call.update({
                    installed: call.prepared.installed,
                    staged: call.prepared.staged,
                    status: call.prepared.staged === null ? "current" : "staged",
                    checkedAt: call.now,
                }),
        },
        activate: {
            prepare: async () => {
                // require a staged release
                const staged = await requireHost(host).distribution.staged();
                if (staged === undefined) {
                    throw new ServiceError("PRECONDITION_FAILED", {
                        message: "no Destack release is staged",
                    });
                }

                return staged.release.version;
            },
            handler: (call) => call.update({ staged: call.prepared, status: "activating" }),
            commit: async () => {
                // restart into it once the activation is recorded
                requireHost(host).activate();
            },
        },
    });
}

/** Keep this machine's distribution row and stage the channel's latest release daily. */
export class DistributionController implements Controller {
    /** The controller's name in reports. */
    readonly name = "distribution";
    /** The server keeping the distribution row. */
    readonly #server: Pick<ObjectServer, "database" | "principal" | "execute">;
    /** This machine, the scope of its distribution. */
    readonly #machineId: Identifier<"machine">;
    /** The installed distribution. */
    readonly #host: DistributionHost;

    /** Control the distribution of a machine. */
    constructor(
        server: Pick<ObjectServer, "database" | "principal" | "execute">,
        machineId: Identifier<"machine">,
        host: DistributionHost,
    ) {
        this.#server = server;
        this.#machineId = machineId;
        this.#host = host;
    }

    /** List this machine as the one key. */
    async list(): Promise<readonly string[]> {
        return [this.#machineId];
    }

    /** Record the installed distribution once, stage the latest release and look again a day later. */
    async reconcile(): Promise<number> {
        // record the distribution on its first start
        const now = Date.now();
        const scope = this.#machineId;
        const [found] = await this.#server.database
            .select()
            .from(distribution.table)
            .where(distribution.inScope(scope));
        const options = this.#host.distribution.options;
        const [recorded] =
            found === undefined
                ? await this.#server.execute(
                      this.#server.principal,
                      distribution,
                      "create",
                      [
                          {
                              scope,
                              input: {
                                  channel: options.channel,
                                  installed: options.running.version,
                              },
                          },
                      ],
                      now,
                      this.name,
                  )
                : [found];
        if (recorded === undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: "recording the distribution answered nothing",
            });
        }

        // stage the latest release
        await this.#server.execute(
            this.#server.principal,
            distribution,
            "stage",
            [{ scope, target: recorded }],
            now,
            this.name,
        );

        return STAGE_INTERVAL_MILLISECONDS;
    }
}

/** Read the host's distribution, refusing a machine running no installed distribution, such as a development build. */
function requireHost(host: DistributionHost | undefined): DistributionHost {
    if (host === undefined) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: "development builds take no published updates",
        });
    }

    return host;
}
