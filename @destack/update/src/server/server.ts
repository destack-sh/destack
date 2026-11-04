import { implementPackage, type PackageImplementation } from "@destack/object/server";
import updatePackage from "../package.ts";
import { type DistributionHost, serveDistributions } from "./distribution.ts";

/** Serve a computer's distribution in its device's scope, staging and activating its releases. */
export function distributionServer(host: DistributionHost | undefined): PackageImplementation {
    return implementPackage(updatePackage, { objects: [serveDistributions(host)] });
}
