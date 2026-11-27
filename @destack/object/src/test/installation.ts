import { BuildLayer } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import type { Deriver } from "@destack/identity";
import { ResourceContext } from "@destack/resource/context";
import { InstallationRoot } from "../workload/root.ts";
import { MemoryBuild } from "@destack/package/test";
import type { PackageVersion } from "@destack/package";
import { TestRemote } from "./publisher.ts";
import type { ServerContext } from "../workload/workload.ts";
import { waitAborted, type Environment } from "@destack/environment";

/** A remote that streams nothing until stopped and refuses writes. */
const IDLE = TestRemote.of({
    async *stream(_subscription, signal) {
        await waitAborted(signal);
        yield* [];
    },
    request: async () => {},
});

/** The bytes of a test installation's root. */
const ROOT_BYTES = 32;

/** Open a fresh installation root's keys in a world, as a test installation has. */
export function testKeys(environment: Pick<Environment, "crypto" | "random">): Promise<Deriver> {
    return InstallationRoot.open(environment.random.bytes(ROOT_BYTES), environment.crypto);
}

/** Build the server context of a package's installation in a space and world, as tests serve it. */
export async function testServerContext(
    owner: PackageVersion,
    space: string,
    environment: Environment,
): Promise<ServerContext> {
    return {
        installation: schema
            .identifier("installation")
            .parse("installation-019f5530-8000-7000-8000-00000000f001"),
        name: "fixture",
        scope: space,
        build: await MemoryBuild.write(
            new Map(),
            { role: "app", package: owner },
            environment.crypto,
        ),
        distribution: BuildLayer.empty(),
        settings: {},
        capabilities: [],
        resources: new ResourceContext(),
        environment,
        keys: await testKeys(environment),
        fetch: (request) =>
            Promise.reject(new Error(`the fixture sends no request to '${request.url}'`)),
        url: `https://fixture--${space}.test`,
        remote: IDLE,
        connect: () => IDLE,
        act: () => {
            throw new TypeError("the fixture acts for no subject");
        },
        at: (address) => ({
            url: address,
            fetch: () => Promise.reject(new Error(`the idle machine cannot call '${address}'`)),
        }),
    };
}
