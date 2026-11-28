/// <reference path="./supplied.d.ts" />
import { PackageImplementation } from "@destack/object/server";
import { Workload } from "@destack/object/workload";
import { DistributionManifest } from "@destack/package";
import type { Digest } from "@destack/schema";
import { MachineError } from "../error/index.ts";

/** The module a cloud machine's deploy supplies with the servers it runs in its process. */
export const SERVER_BUNDLE_MODULE = "destack:servers";

/** The module a cloud machine's deploy supplies with the view workloads it runs in its process. */
export const VIEWS_BUNDLE_MODULE = "destack:views";

/** The servers a cloud machine runs in its process, compiled into its Worker by its deploy. */
export class ServerBundle {
    /** The bundled server of each shipped package, by package name. */
    readonly #servers: ReadonlyMap<string, PackageImplementation>;
    /** The bundled view workload of each shipped package with views, by package name. */
    readonly #views: ReadonlyMap<string, Workload>;
    /** The distribution the deploy shipped. */
    readonly #shipped: DistributionManifest;

    /** Keep the bundled servers beside the distribution they ship with. */
    private constructor(
        servers: ReadonlyMap<string, PackageImplementation>,
        views: ReadonlyMap<string, Workload>,
        shipped: DistributionManifest,
    ) {
        this.#servers = servers;
        this.#views = views;
        this.#shipped = shipped;
    }

    /** Read the servers the deploy supplied, refusing a module exporting anything else. */
    static async read(shipped: DistributionManifest): Promise<ServerBundle> {
        // read the servers the deploy supplied, refusing an export that is no package implementation
        const suppliedServers: { readonly default: Readonly<Record<string, unknown>> } =
            await import("destack:servers");
        const servers = new Map(
            Object.entries(suppliedServers.default).map(([name, server]) => {
                if (!PackageImplementation.is(server)) {
                    throw MachineError.of(
                        "SERVER_BROKEN",
                        `'${name}' exports no package implementation as its server`,
                    );
                }

                return [name, server] as const;
            }),
        );

        // read the view workloads the deploy supplied, refusing an export that is no workload
        const suppliedViews: { readonly default: Readonly<Record<string, unknown>> } =
            await import("destack:views");
        const views = new Map(
            Object.entries(suppliedViews.default).map(([name, workload]) => {
                if (!Workload.is(workload)) {
                    throw MachineError.of(
                        "SERVER_BROKEN",
                        `'${name}' exports no workload from its view workload module`,
                    );
                }

                return [name, workload] as const;
            }),
        );

        return new ServerBundle(servers, views, shipped);
    }

    /** Find the bundled server of a shipped release's image, none without one, refusing others. */
    server(image: Digest): PackageImplementation | undefined {
        return this.#servers.get(this.#shippedName(image));
    }

    /** Find the bundled view workload of a shipped release's image, none without one. */
    views(image: Digest): Workload | undefined {
        return this.#views.get(this.#shippedName(image));
    }

    /** Read the package name of a shipped release's image, refusing one the deploy did not ship. */
    #shippedName(image: Digest): string {
        const release = DistributionManifest.releases(this.#shipped, "workerd").find(
            (each) => each.image === image,
        );
        if (release === undefined) {
            throw MachineError.of(
                "SERVER_BROKEN",
                `a cloud machine runs in its process only the servers its deploy ships, not image '${image}'`,
            );
        }

        return release.package.name;
    }
}
