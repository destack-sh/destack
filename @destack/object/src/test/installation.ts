import { schema } from "@destack/schema";
import type { Deriver } from "@destack/identity";
import { ResourceContext } from "@destack/resource/context";
import { InstallationRoot } from "../workload/root.ts";
import { MemoryBuild } from "@destack/package/test";
import type { Package } from "@destack/package";
import type { Uplink } from "@destack/sync";
import type { InstalledServerContext } from "../workload/workload.ts";
import { until, type Environment } from "@destack/environment";

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

/** The bytes of a test installation's root. */
const ROOT_BYTES = 32;

/** Open a fresh installation root's keys in a world, as a test installation has. */
export function testKeys(environment: Pick<Environment, "crypto" | "random">): Promise<Deriver> {
    return InstallationRoot.open(environment.random.bytes(ROOT_BYTES), environment.crypto);
}

/** Build the server context of a package's installation in a space and world, as tests serve it. */
export async function testServerContext(
    owner: Package,
    space: string,
    environment: Environment,
): Promise<InstalledServerContext> {
    return {
        origin: "installed",
        installation: schema
            .identifier("installation")
            .parse("installation-019f5530-8000-7000-8000-00000000f001"),
        name: "fixture",
        scope: space,
        build: await MemoryBuild.write(new Map(), { package: owner }, environment.crypto),
        settings: {},
        capabilities: [],
        resources: new ResourceContext(),
        environment,
        keys: await testKeys(environment),
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
