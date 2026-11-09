import { WorkerdEnvironment } from "@destack/environment/workerd";
import { BUILD_PATH } from "@destack/object/cloudflare";
import { Egress } from "@destack/object";
import { AlarmClock } from "@destack/db";
import {
    FacetHost,
    type HibernatingSocket,
    type PartitionBindings,
    PartitionName,
    type PartitionState,
} from "../instance.ts";

/** The scenario's bindings: the partitions, the loader, and the echo workload's module. */
interface ScenarioBindings extends PartitionBindings {
    /** The bundled echo workload, the one module of its build. */
    readonly MODULE: string;
    /** The echo workload's module name. */
    readonly MODULE_NAME: string;
    /** The space whose object serves the build. */
    readonly SPACE: string;
    /** The secret the host stamps on requests for the workload. */
    readonly SECRET: string;
}

/** A partition's object: the space's with its installations' facets, or a partition's. */
export class Partition {
    /** The object's name. */
    readonly #name: string;
    /** The Worker's bindings. */
    readonly #bindings: ScenarioBindings;
    /** The object's one alarm. */
    readonly #alarms: AlarmClock;
    /** The facets the object hosts. */
    readonly #facets: FacetHost;

    /** Take the object's name, state and bindings. */
    constructor(
        state: PartitionState & { readonly id: { readonly name?: string } },
        bindings: ScenarioBindings,
    ) {
        // keep the object's name and bindings, and host its facets on its alarm
        this.#name = state.id.name ?? "";
        this.#bindings = bindings;
        this.#alarms = new AlarmClock(state.storage, state.storage);
        this.#facets = new FacetHost(state, bindings, WorkerdEnvironment.system, this.#alarms);
    }

    /** Serve a facet, the space's build and its calls out. */
    async fetch(request: Request): Promise<Response> {
        // hand a request below an installation to its facet, the workloads' own calls among them
        const url = new URL(request.url);
        const { installation } = PartitionName.read(this.#name);
        const routed = this.#facets.route(request);
        if (routed !== undefined) {
            return routed;
        }
        // serve a partition's one facet
        else if (installation !== undefined) {
            return this.#facets.facet(installation).fetch(request);
        }
        // serve the space's build
        else if (url.pathname === BUILD_PATH) {
            return Response.json({ [this.#bindings.MODULE_NAME]: this.#bindings.MODULE });
        }
        // accept the space's calls out
        else if (url.pathname.startsWith(Egress.path)) {
            return Response.json({ calls: [] });
        }
        // refuse any other path
        else {
            return new Response(null, { status: 404 });
        }
    }

    /** Hand an accepted WebSocket's message to its installation's workload. */
    webSocketMessage(socket: HibernatingSocket, message: ArrayBuffer | string): Promise<void> {
        return this.#facets.socket(socket).webSocketMessage(socket, message);
    }

    /** Hand an accepted WebSocket's close to its installation's workload. */
    webSocketClose(socket: HibernatingSocket): Promise<void> {
        return this.#facets.socket(socket).webSocketClose(socket);
    }

    /** Fire the due facets' alarms through the object's schedule. */
    alarm(): Promise<void> {
        return this.#alarms.fire(WorkerdEnvironment.system.clock.now(), async (parties) => {
            for (const party of parties) {
                await this.#facets.facet(party).alarm();
            }
        });
    }
}

/** The scenario Worker. */
export default {
    /** Forward a request below its first path segment to the object it names. */
    fetch(request: Request, bindings: ScenarioBindings): Promise<Response> {
        // read the object's name and the path below it
        const url = new URL(request.url);
        const [, name = "", ...below] = url.pathname.split("/");
        url.pathname = `/${below.join("/")}`;

        // forward the request as the host, with the secret of the partition or of the installation below the space,
        //  unless the request brings its own
        const forwarded = new Request(url.href, request);
        const [hosted] = below;
        const secret =
            name === bindings.SPACE ? `${bindings.SECRET}:${hosted ?? ""}` : bindings.SECRET;
        if (!forwarded.headers.has("authorization")) {
            forwarded.headers.set("authorization", `Bearer ${secret}`);
        }
        const namespace = bindings.PARTITION;

        return namespace.get(namespace.idFromName(name)).fetch(forwarded);
    },
};
