import type { PackageHandle } from "@destack/object";
import type { PackageImplementation } from "@destack/object/server";
import type { BuildReader } from "@destack/package/manifest";
import type { Connector } from "@destack/resource";
import type { TestSpaceOptions } from "./space.ts";

/** A package a test space installs: its linked build, its server and what it needs. */
export interface TestPackage {
    /** The package, whose tables the space's database has. */
    readonly handle: Pick<PackageHandle, "objects" | "tables">;
    /** The build the machine links and the space installs. */
    readonly build: BuildReader;
    /** The server the machine runs the package with in process. */
    readonly server: PackageImplementation;
    /** The connectors its bound resources call. */
    readonly connectors: readonly Connector[];
    /** The providers the space offers it, such as a mail provider, served ones in process. */
    readonly providers: NonNullable<TestSpaceOptions["providers"]>;
}

/** The packages a test space installs, in dependency order. */
export const TestPackage = {
    /** Read the options of a space installing some packages, with each one's needs once. */
    options(packages: readonly TestPackage[]) {
        return {
            packages: packages.map((each) => each.handle),
            servers: packages.map((each) => each.server),
            linked: packages.map((each) => each.build),
            connectors: [...new Set(packages.flatMap((each) => each.connectors))],
            providers: [...new Set(packages.flatMap((each) => each.providers))],
        } satisfies TestSpaceOptions;
    },
};
