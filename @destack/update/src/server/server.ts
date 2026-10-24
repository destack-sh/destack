import { implementPackage, type PackageImplementation } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import updatePackage from "../package.ts";
import {
    type DistributionHost,
    DistributionController,
    serveDistributions,
} from "./distribution.ts";

/** What a computer serves the update package with. */
export interface DistributionServerOptions {
    /** The computer's device, the scope of its distribution. */
    readonly device: Identifier<"device">;
    /** The installed distribution, absent for a development build. */
    readonly host?: DistributionHost;
}

/** Serve a computer's distribution in its device's scope, staging and activating its releases. */
export function distributionServer(options: DistributionServerOptions): PackageImplementation {
    const { device, host } = options;

    return implementPackage(updatePackage, {
        objects: [serveDistributions(host)],
        controllers:
            host === undefined ? [] : [(store) => new DistributionController(store, device, host)],
    });
}
