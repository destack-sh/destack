import type { Authentication } from "@destack/access";
import { SPACE_KEY } from "@destack/access/token";
import { Identity } from "@destack/account/object";
import { copyRequest, type Listener } from "@destack/environment";
import type { Simulation } from "@destack/environment/simulation";
import { InstallationMount, PackageMount, SpaceMount } from "@destack/object";
import type { TestObjectStore } from "@destack/object/test";
import type { PackageId } from "@destack/package";
import { PeerCall } from "../server/index.ts";
import type { TestSpace } from "./space.ts";

/** The port a machine's HTTPS origin answers at. */
const HTTPS_PORT = 443;

/** The port a machine's HTTP origin answers at. */
const HTTP_PORT = 80;

/** The machines of a world answering at their origins on its network, each space at its mount. */
export class TestGateway {
    /** The world whose network the machines answer on. */
    readonly #simulation: Simulation;
    /** The served spaces by space. */
    readonly #served = new Map<
        string,
        { readonly space: TestSpace; readonly served: TestObjectStore }
    >();
    /** The listener of each machine answering, by machine. */
    readonly #listening = new Map<string, Listener<unknown>>();

    /** Answer for the machines of a world. */
    constructor(simulation: Simulation) {
        this.#simulation = simulation;
    }

    /** Answer a served space's requests at its machine's origin until the returned stop. */
    serve(space: TestSpace, served: TestObjectStore): () => void {
        // listen at the machine's origin once, on the machine's node
        const machineId = space.host.machine.id;
        if (!this.#listening.has(machineId)) {
            const origin = new URL(space.host.machine.origin);
            const listener = this.#simulation.network.server(machineId).http({
                hostname: origin.hostname,
                port: Number(
                    origin.port || (origin.protocol === "https:" ? HTTPS_PORT : HTTP_PORT),
                ),
                fetch: (request) => this.answer(request),
            });
            this.#listening.set(machineId, listener);
        }

        // route the space's requests to its store until it stops
        this.#served.set(space.id, { space, served });

        return () => {
            if (this.#served.get(space.id)?.served === served) {
                this.#served.delete(space.id);
            }
        };
    }

    /** Answer a request to a space's package or its installation's, as a machine does. */
    async answer(request: Request): Promise<Response> {
        // find the served space, and the request below its mount or its installation's
        const url = new URL(request.url);
        const mounted = SpaceMount.parse(url.pathname);
        const entry = mounted === undefined ? undefined : this.#served.get(mounted.spaceId);
        if (mounted === undefined || entry === undefined) {
            return new Response(null, { status: 404 });
        }
        url.pathname = mounted.path;
        const below = InstallationMount.of(request)?.request ?? copyRequest(request, {}, url.href);

        // find the served package the path mounts
        const called = PackageMount.of(below);
        const handler = entry.served.store.packages.find(
            (each) => each.package.id === called?.packageId,
        );
        if (called === undefined || handler === undefined) {
            return new Response(null, { status: 404 });
        }

        // verify the caller, then answer as the package
        const authentication = await TestGateway.#verify(entry.space, request, handler.package.id);

        return entry.served.server(handler.package).fetch(called.request, authentication);
    }

    /** Verify a caller as a machine does: a peer installation, else a token for the package. */
    static async #verify(
        space: TestSpace,
        request: Request,
        audience: PackageId,
    ): Promise<Authentication | undefined> {
        // take another space's installation calling one of the space's, inside its account's chain
        const { host } = space;
        const installed = InstallationMount.of(request);
        const peer = PeerCall.of(request);
        if (installed !== undefined && peer !== undefined) {
            return (await peer.receive(host, installed)).authentication;
        }

        // verify the universe's or a space's token, placing another space's caller in its account's chain
        const { machine } = host;
        const verified = await Identity.authenticate(machine.directory.snapshot, request, {
            audience,
            universe: machine.universe.issuer,
            now: machine.environment.clock.now(),
            environment: machine.environment,
        });
        if (verified === undefined) {
            return undefined;
        }
        await host.requirePartition(verified);

        return verified.credential.kind === SPACE_KEY ? host.contain(verified) : verified;
    }

    /** Stop the controllers of every served space at once, so none follows a space that closes. */
    async stopControllers(): Promise<void> {
        await Promise.all([...this.#served.values()].map(({ served }) => served.stopControllers()));
    }

    /** Stop answering for every machine. */
    async [Symbol.asyncDispose](): Promise<void> {
        await Promise.all([...this.#listening.values()].map((listener) => listener.stop(true)));
        this.#listening.clear();
    }
}
