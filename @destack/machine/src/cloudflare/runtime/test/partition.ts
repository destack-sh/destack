import { WorkerdEnvironment } from "@destack/environment/workerd";
import { BUILD_PATH } from "@destack/object/cloudflare";
import {
    type HibernatingSocket,
    PartitionFacet,
    type PartitionBindings,
    type PartitionState,
} from "../instance.ts";

/** The scenario's bindings: the partitions, the loader, and the echo workload's module. */
interface ScenarioBindings extends PartitionBindings {
    /** The bundled echo workload, the one module of its build. */
    readonly MODULE: string;
    /** The space whose object serves the build. */
    readonly SPACE: string;
    /** The secret the host stamps on requests for the workload. */
    readonly SECRET: string;
}

/** A partition's object. */
export class Partition {
    /** The object's name. */
    readonly #name: string;
    /** The Worker's bindings. */
    readonly #bindings: ScenarioBindings;
    /** The installation's partition, its workload in a facet. */
    readonly #facet: PartitionFacet;

    /** Take the object's name, the bindings and the facet. */
    constructor(
        state: PartitionState & { readonly id: { readonly name?: string } },
        bindings: ScenarioBindings,
    ) {
        this.#name = state.id.name ?? "";
        this.#bindings = bindings;
        this.#facet = new PartitionFacet(state, bindings, WorkerdEnvironment.system);
    }

    /** Serve the space's build and accept its calls out, else hand the request to the facet. */
    async fetch(request: Request): Promise<Response> {
        // serve the space's role
        if (this.#name === this.#bindings.SPACE) {
            const { pathname } = new URL(request.url);

            return pathname.startsWith(`${BUILD_PATH}/`)
                ? new Response(this.#bindings.MODULE)
                : Response.json({ calls: [] });
        }

        return this.#facet.fetch(request);
    }

    /** Hand an accepted WebSocket's message to the workload. */
    webSocketMessage(socket: HibernatingSocket, message: ArrayBuffer | string): Promise<void> {
        return this.#facet.webSocketMessage(socket, message);
    }

    /** Hand an accepted WebSocket's close to the workload. */
    webSocketClose(socket: HibernatingSocket): Promise<void> {
        return this.#facet.webSocketClose(socket);
    }

    /** Ring the workload's alarm. */
    alarm(): Promise<void> {
        return this.#facet.alarm();
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

        // forward the request as the host
        const forwarded = new Request(url.href, request);
        forwarded.headers.set("authorization", `Bearer ${bindings.SECRET}`);
        const namespace = bindings.PARTITION;

        return namespace.get(namespace.idFromName(name)).fetch(forwarded);
    },
};
