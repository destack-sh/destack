import { MemoryBuild } from "@destack/package/test";
import type { Package } from "@destack/package";
import type { Uplink } from "@destack/sync";
import type { InstallationDirectory, InstallationContext } from "../workload/workload.ts";
import { until } from "../timer/timer.ts";

/** An uplink streaming nothing until stopped and refusing changes, as a machine with no chain to copy. */
const IDLE: Uplink = {
    async *stream(_subscription, signal) {
        await until(signal);
        yield* [];
    },
    receive: () => Promise.reject(new Error("the idle machine receives no changes")),
};

/** A directory in which nobody lives anywhere, reads a locale, receives projected rows or is sent messages. */
const NOBODY: InstallationDirectory = {
    isHome: () => Promise.resolve(false),
    locale: (subject) => Promise.reject(new Error(`no person ${subject} reads a locale here`)),
    address: () => Promise.resolve(),
    messageKey: (space) => Promise.reject(new Error(`no space ${space} is sent messages here`)),
};

/** Build the context of a package's installation in a space, as tests serve its objects, with an empty build, no settings and an idle machine. */
export async function testInstallation(
    owner: Package,
    scope: string,
    directory: InstallationDirectory = NOBODY,
): Promise<InstallationContext> {
    return {
        id: "installation-019f5530-8000-7000-8000-00000000f001",
        scope,
        build: await MemoryBuild.write(new Map(), { package: owner }),
        settings: {},
        publisher: IDLE,
        publisherAt: () => IDLE,
        at: (address) => ({
            url: address,
            fetch: () => Promise.reject(new Error(`the idle machine reaches no ${address}`)),
        }),
        directory,
    };
}
