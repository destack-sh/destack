/// <reference path="./supplied.d.ts" />
import { PackageImplementation } from "@destack/object/server";
import { DistributionManifest } from "@destack/package";
import type { Digest } from "@destack/schema";
import { MachineError } from "../error/index.ts";

/** The module a cloud machine's deploy supplies with the servers it runs in its process. */
export const SERVER_BUNDLE_MODULE = "destack:servers";

/** The servers a cloud machine runs in its process, compiled into its Worker by its deploy. */
export class ServerBundle {
    /** The bundled server of each shipped package, by package name. */
    readonly #servers: ReadonlyMap<string, PackageImplementation>;
    /** The distribution the deploy shipped. */
    readonly #shipped: DistributionManifest;

    /** Keep the bundled servers beside the distribution they ship with. */
    private constructor(
        servers: ReadonlyMap<string, PackageImplementation>,
        shipped: DistributionManifest,
    ) {
        this.#servers = servers;
        this.#shipped = shipped;
    }

    /** Read the servers the deploy supplied, refusing a module exporting anything else. */
    static async read(shipped: DistributionManifest): Promise<ServerBundle> {
        const supplied: { readonly default: Readonly<Record<string, unknown>> } =
            await import("destack:servers");
        const servers = new Map(
            Object.entries(supplied.default).map(([name, server]) => {
                if (!PackageImplementation.is(server)) {
                    throw MachineError.of(
                        "SERVER_BROKEN",
                        `'${name}' exports no package implementation as its server`,
                    );
                }

                return [name, server] as const;
            }),
        );

        return new ServerBundle(servers, shipped);
    }

    /** Find the bundled server of a shipped release's lock, none without one, refusing others. */
    server(lock: Digest): PackageImplementation | undefined {
        // refuse a lock the deploy did not ship
        const release = DistributionManifest.releases(this.#shipped, "workerd").find(
            (each) => each.lock === lock,
        );
        if (release === undefined) {
            throw MachineError.of(
                "SERVER_BROKEN",
                `a cloud machine runs in its process only the servers its deploy ships, not lock '${lock}'`,
            );
        }

        return this.#servers.get(release.package.name);
    }
}
