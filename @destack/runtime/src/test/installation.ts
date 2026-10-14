import { ResourceContext } from "@destack/resource/context";
import { MemoryBuild } from "@destack/package/test";
import type { Package } from "@destack/package";
import type { Uplink } from "@destack/sync";
import type { InstallationContext } from "../workload/workload.ts";
import { until } from "../timer/timer.ts";

/** An uplink that streams nothing until stopped and refuses changes. */
const IDLE: Uplink = {
    async *stream(_subscription, signal) {
        await until(signal);
        yield* [];
    },
    receive: () => Promise.reject(new Error("the idle machine cannot receive changes")),
};

/** Build the context of a package's installation in a space, as tests serve its objects. */
export async function testInstallation(
    owner: Package,
    scope: string,
): Promise<InstallationContext> {
    return {
        id: "installation-019f5530-8000-7000-8000-00000000f001",
        scope,
        build: await MemoryBuild.write(new Map(), { package: owner }),
        settings: {},
        resources: new ResourceContext(),
        fetch: (request) =>
            Promise.reject(new Error(`the fixture sends no request to ${request.url}`)),
        publisher: IDLE,
        publisherAt: () => IDLE,
        at: (address) => ({
            url: address,
            fetch: () => Promise.reject(new Error(`the idle machine cannot call ${address}`)),
        }),
    };
}
