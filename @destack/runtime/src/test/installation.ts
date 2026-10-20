import type { Deriver } from "@destack/identity";
import { ResourceContext } from "@destack/resource/context";
import { InstallationRoot } from "../workload/root.ts";
import { MemoryBuild } from "@destack/package/test";
import type { Package } from "@destack/package";
import type { Uplink } from "@destack/sync";
import type { ServerContext } from "../workload/workload.ts";
import { until } from "../timer/timer.ts";

/** An uplink that streams nothing until stopped and refuses changes. */
const IDLE: Uplink = {
    async *stream(_subscription, signal) {
        await until(signal);
        yield* [];
    },
    confirm: async () => {},
    receive: () => Promise.reject(new Error("the idle machine cannot receive changes")),
    call: () => Promise.reject(new Error("the idle machine answers no queries")),
};

/** Open a fresh installation root's keys, as a test installation has. */
export function testKeys(): Promise<Deriver> {
    return InstallationRoot.open(crypto.getRandomValues(new Uint8Array(32)));
}

/** Build the server context of a package's installation in a space, as tests serve its objects. */
export async function testServerContext(owner: Package, space: string): Promise<ServerContext> {
    return {
        installation: "installation-019f5530-8000-7000-8000-00000000f001",
        name: "fixture",
        space,
        build: await MemoryBuild.write(new Map(), { package: owner }),
        settings: {},
        capabilities: [],
        resources: new ResourceContext(),
        keys: await testKeys(),
        fetch: (request) =>
            Promise.reject(new Error(`the fixture sends no request to ${request.url}`)),
        url: `https://fixture--${space}.test`,
        publisher: IDLE,
        publisherAt: () => IDLE,
        at: (address) => ({
            url: address,
            fetch: () => Promise.reject(new Error(`the idle machine cannot call ${address}`)),
        }),
    };
}
