import type { Package } from "@destack/package";
import type { Publisher } from "@destack/sync";
import type { CellDirectory, InstallationContext } from "../workload/workload.ts";
import { until } from "../timer/timer.ts";
import { emptyBuild } from "./build.ts";

/** A publisher streaming nothing until stopped, as a cell with no chain to copy. */
const IDLE: Publisher = {
    async *stream(_subscription, signal) {
        await until(signal);
        yield* [];
    },
};

/** A directory in which nobody lives anywhere, has set a locale, or receives projected rows. */
const NOBODY: CellDirectory = {
    isHome: () => Promise.resolve(false),
    locale: () => Promise.resolve(undefined),
    address: () => Promise.resolve(),
};

/** Build the context of a package's installation in a space, as tests serve its objects, with an empty build and an idle cell. */
export async function testInstallation(
    owner: Package,
    scope: string,
    directory: CellDirectory = NOBODY,
): Promise<InstallationContext> {
    return {
        id: "installation-019f5530-8000-7000-8000-00000000f001",
        scope,
        build: await emptyBuild(owner),
        publisher: IDLE,
        publisherAt: () => IDLE,
        directory,
    };
}
