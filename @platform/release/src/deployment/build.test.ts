import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { expect, test } from "@destack/test";
import { CHECKOUT } from "../distribution/index.ts";
import { DistributionManifest } from "@destack/package";
import { PlatformBuild } from "./build.ts";

/** How long a cold build of the platform's releases may take: about 60 units at seconds each. */
const BUILD_MILLISECONDS = 300_000;

/** The store keeping the platform's releases across runs, so warm runs skip their build. */
const RELEASES = join(CHECKOUT, "target", "test", "platform");

/** The builtins both machine runtimes link: every builtin the runtime is built with. */
const BUILTINS = [
    "@destack/access",
    "@destack/account",
    "@destack/connection",
    "@destack/control",
    "@destack/db",
    "@destack/environment",
    "@destack/error",
    "@destack/event",
    "@destack/identity",
    "@destack/locale",
    "@destack/machine",
    "@destack/object",
    "@destack/package",
    "@destack/registry",
    "@destack/resource",
    "@destack/schema",
    "@destack/setting",
    "@destack/stack",
    "@destack/sync",
    "@destack/telemetry",
    "@destack/update",
    "@destack/vault",
    "@destack/workflow",
];

test(
    "ship every release with its role, the builtins among them being what a cloud machine links",
    async () => {
        // build the platform with the desktop and the machine among its releases
        await using build = await PlatformBuild.build(CHECKOUT, RELEASES, testHost());

        // list the shipped builtins, every Destack builtin and none other
        const builtins = DistributionManifest.builtins(await build.distribution());
        const names = [...builtins.values()].map((release) => release.package.name);
        expect(names.toSorted()).toEqual(BUILTINS);
    },
    BUILD_MILLISECONDS,
);
