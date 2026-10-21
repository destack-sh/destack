import { implementPackage, type PackageImplementation } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import updatePackage from "../package.ts";
import {
    type DistributionHost,
    DistributionController,
    serveDistributions,
} from "./distribution.ts";

/** What a machine serves the update package with. */
export interface DistributionServerOptions {
    /** The machine, the scope of its distribution. */
    readonly machine: Identifier<"machine">;
    /** The installed distribution, absent for a development build. */
    readonly host?: DistributionHost;
}

/** Serve a machine's distribution in the machine's scope, staging and activating its releases. */
export function distributionServer(options: DistributionServerOptions): PackageImplementation {
    const { machine, host } = options;

    return implementPackage(updatePackage, {
        objects: [serveDistributions(host)],
        controllers:
            host === undefined ? [] : [(store) => new DistributionController(store, machine, host)],
    });
}
