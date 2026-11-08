import type { Environment } from "@destack/environment";
import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import {
    CONTENT_PATH,
    DATABASE_PATH,
    type DurableObjectNamespace,
    Jurisdiction,
} from "@destack/db/cloudflare";
import { Digest, schema } from "@destack/schema";
import {
    ALARM_MILLISECONDS,
    ALARM_PATH,
    BUILD_PATH,
    DurableObjectAlarm,
    DurableObjectStart,
    START_PATH,
    STOP_PATH,
} from "@destack/object/cloudflare";
import { WORKLOAD_EXPORT } from "@destack/object/workload";
import { refusal } from "@destack/object";
import {
    SocketDescription,
    SOCKET_HEADER,
    SOCKET_PATH,
    SocketAcceptance,
} from "@destack/object/workload";
import { Attribution } from "../attribution.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../error/index.ts";

/** The facet running the instance's workload and storing its databases. */
const FACET = "workload";

/** The key of the start the object stores, which it resumes after an eviction. */
const START_KEY = "instance";

/** The class of the static object that answers database operations before a workload starts. */
const DATABASE_EXPORT = "DurableObjectDatabase";

/** The path a partition's object takes a call of a build's export at. */
export const CALL_PATH = "/.destack/call";

/** The module a call's isolate starts from, which calls the export a request names. */
const CALL_MODULE = "destack-call.js";

/** A call of a build's export, once, in an isolate of its own. */
export const DurableObjectCall = schema.object({
    /** The space whose object serves the build's files. */
    space: schema.string().min(1),
    /** The secret the space serves the build's modules to. */
    secret: schema.string().min(1),
    /** The module with the export, named below its build's digest. */
    main: schema.string().min(1),
    /** The function the module exports. */
    name: schema.string().min(1),
    /** The JSON arguments of the call. */
    input: schema.array(schema.json()),
});
/** A call of a build's export, once, in an isolate of its own. */
export type DurableObjectCall = schema.Infer<typeof DurableObjectCall>;

/** A start of an instance's workload. */
export const DurableObjectInstanceStart = schema.object({
    /** The workload's start and its build's manifest. */
    workload: DurableObjectStart,
    /** The space whose object serves the instance's egress and build files. */
    space: schema.string().min(1),
    /** The module exporting the workload's class, named below its build's digest. */
    main: schema.string().min(1),
});
/** A start of an instance's workload. */
export type DurableObjectInstanceStart = schema.Infer<typeof DurableObjectInstanceStart>;

/** The loader of a workload's code into an isolate of its own. */
export interface WorkerLoader {
    /** Load code under a name once, reusing its isolate while it lives. */
    readonly get: (
        name: string,
        code: () => Promise<{
            readonly compatibilityDate: string;
            readonly compatibilityFlags: readonly string[];
            readonly mainModule: string;
            readonly modules: Readonly<Record<string, { readonly js: string }>>;
            readonly env: Readonly<Record<string, unknown>>;
            readonly globalOutbound: { readonly fetch: (request: Request) => Promise<Response> };
        }>,
    ) => {
        /** Read a Durable Object class the code exports. */
        readonly getDurableObjectClass: (name: string) => unknown;
        /** Read the code's default entrypoint. */
        readonly getEntrypoint: () => { readonly fetch: (request: Request) => Promise<Response> };
    };
}

/** The state of an instance's object. */
export interface PartitionState {
    /** The object's identifier, which addresses the object itself. */
    readonly id: unknown;
    /** The object's storage: its stored start and its wake-up. */
    readonly storage: {
        /** Read a stored value. */
        readonly get: (key: string) => Promise<unknown>;
        /** Store a value. */
        readonly put: (key: string, value: unknown) => Promise<void>;
        /** Forget a value. */
        readonly delete: (key: string) => Promise<boolean>;
        /** Wake the object at a time. */
        readonly setAlarm: (at: number) => Promise<void>;
        /** Wake the object at no time. */
        readonly deleteAlarm: () => Promise<void>;
    };
    /** The object's facets, each an object of its own beside it with storage of its own. */
    readonly facets: {
        /** Get a facet, starting it with a class when it does not run. */
        readonly get: (
            name: string,
            options: () => { readonly class: unknown } | Promise<{ readonly class: unknown }>,
        ) => { readonly fetch: (request: Request) => Promise<Response> };
        /** Stop a facet, leaving its storage. */
        readonly abort: (name: string, reason: unknown) => void;
    };
    /** The Worker's own exports, such as the static object answering database operations. */
    readonly exports: Readonly<Record<string, unknown>>;
    /** Accept a WebSocket that stays open across hibernation under a tag. */
    readonly acceptWebSocket: (socket: HibernatingSocket, tags: readonly string[]) => void;
    /** List the accepted WebSockets, those of one tag when given. */
    readonly getWebSockets: (tag?: string) => HibernatingSocket[];
    /** Answer a text message on every accepted WebSocket without waking the object. */
    readonly setWebSocketAutoResponse: (pair: unknown) => void;
}

/** The server end of a WebSocket workerd leaves open for a Durable Object across hibernation. */
export interface HibernatingSocket {
    /** Send one message. */
    readonly send: (message: ArrayBuffer | Uint8Array) => void;
    /** Close the connection. */
    readonly close: () => void;
    /** Attach a value to the WebSocket across hibernation. */
    readonly serializeAttachment: (value: unknown) => void;
    /** Read the value stored with the WebSocket. */
    readonly deserializeAttachment: () => unknown;
}

/** The pair of WebSocket ends workerd creates for an upgrade, the client's first. */
declare const WebSocketPair: new () => { readonly 0: unknown; readonly 1: HibernatingSocket };

/** A text message and the answer workerd sends to it on a hibernating WebSocket's behalf. */
declare const WebSocketRequestResponsePair: new (request: string, response: string) => unknown;

/** What workerd answers an upgrade with: the switch, and the client's end of the WebSocket. */
interface Upgrade extends ResponseInit {
    /** The client's end of the WebSocket. */
    readonly webSocket: unknown;
}

/** The bindings a partition's object reads. */
export interface PartitionBindings {
    /** The loader of the workloads' code. */
    readonly LOADER: WorkerLoader;
    /** The partitions' objects. */
    readonly PARTITION: DurableObjectNamespace;
    /** The jurisdiction of the partitions' objects. */
    readonly DESTACK_JURISDICTION: Jurisdiction;
}

/** The name of the object serving a partition: its address, innermost first, joined by `--`. */
export const PartitionName = {
    /** Read the object name of a partition: a space's, an installation's, or one below one. */
    of(place: {
        readonly space: string;
        readonly installation?: string;
        readonly partition?: string;
    }): string {
        return [place.partition, place.installation, place.space]
            .filter((part) => part !== undefined)
            .join("--");
    },
};

/** An installation's partition in a partition's object. */
export class PartitionFacet {
    /** The object's state. */
    readonly #state: PartitionState;
    /** The Worker's bindings. */
    readonly #bindings: PartitionBindings;
    /** The clock the object's alarms ring at and the telemetry its refusals report to. */
    readonly #environment: Pick<Environment, "clock" | "telemetry" | "crypto">;
    /** The stored start, read from storage once; this object is its only writer. */
    #start: Promise<DurableObjectInstanceStart | undefined> | undefined;

    /** Take the object's state, bindings and environment. */
    constructor(
        state: PartitionState,
        bindings: PartitionBindings,
        environment: Pick<Environment, "clock" | "telemetry" | "crypto">,
    ) {
        this.#state = state;
        this.#bindings = bindings;
        this.#environment = environment;
    }

    /** Take a start, a stop or a database operation from the space. */
    async fetch(request: Request): Promise<Response> {
        try {
            // attribute the fetch to the installation of the running workload
            const { pathname } = new URL(request.url);
            await this.#attribute();

            // store a start and load its workload's code
            if (pathname === START_PATH) {
                const started = DurableObjectInstanceStart.parse(await request.json());
                await this.#stop();
                await this.#state.storage.put(START_KEY, started);
                this.#start = Promise.resolve(started);

                return await this.#facet(started).fetch(
                    new Request(`https://workload${START_PATH}`, {
                        method: "POST",
                        body: JSON.stringify(started.workload),
                    }),
                );
            }
            // call a build's export once in an isolate of its own
            else if (pathname === CALL_PATH) {
                return await this.#call(DurableObjectCall.parse(await request.json()));
            }
            // drain the workload and forget its start
            else if (pathname === STOP_PATH) {
                await this.#stop();

                return Response.json({});
            }
            // move the wake-up the workload asks for
            else if (pathname === ALARM_PATH) {
                const { at } = DurableObjectAlarm.parse(await request.json());
                await (at === null
                    ? this.#state.storage.deleteAlarm()
                    : this.#state.storage.setAlarm(at));

                return Response.json({});
            }
            // answer the workload's sends, closes and listing of its accepted sockets
            else if (pathname === SOCKET_PATH || pathname.startsWith(`${SOCKET_PATH}/`)) {
                return await this.#answerSockets(request, pathname);
            }
            // read a build file or a database copy's content through the space serving it
            else if (
                pathname.startsWith(`${BUILD_PATH}/`) ||
                pathname.startsWith(`${CONTENT_PATH}/`)
            ) {
                const started = await this.#started();
                if (started === undefined) {
                    throw MachineError.of("WORKLOAD_MISSING", "no workload runs here");
                }

                return await this.#space(started.space).fetch(request);
            }

            // hand a database operation or a request to the facet
            const started = await this.#started();
            if (started === undefined && pathname !== DATABASE_PATH) {
                throw MachineError.of("WORKLOAD_MISSING", "no workload runs here");
            }

            const response = await this.#facet(started).fetch(request);

            // accept a WebSocket the workload accepted, answering the upgrade
            const accepted = response.headers.get(SOCKET_HEADER);

            return accepted === null
                ? response
                : this.#accept(SocketAcceptance.parse(JSON.parse(accepted)));
        } catch (error) {
            return refusal(error, this.#environment.telemetry);
        }
    }

    /** Hand the workload an accepted WebSocket's message. */
    async webSocketMessage(
        socket: HibernatingSocket,
        message: ArrayBuffer | string,
    ): Promise<void> {
        const body = typeof message === "string" ? new TextEncoder().encode(message) : message;
        await this.#event(socket, { method: "POST", body });
    }

    /** Hand the workload an accepted WebSocket's close. */
    async webSocketClose(socket: HibernatingSocket): Promise<void> {
        await this.#event(socket, { method: "DELETE" });
    }

    /** Hibernate a WebSocket the workload accepted under a new identifier. */
    #accept(acceptance: SocketAcceptance): Response {
        // accept the server's end with its description
        const pair = new WebSocketPair();
        const description: SocketDescription = {
            id: acceptance.id,
            attachment: acceptance.attachment,
        };
        this.#state.acceptWebSocket(pair[1], [description.id]);
        pair[1].serializeAttachment(description);

        // answer the declared text message without waking the object
        if (acceptance.respond !== undefined) {
            const { request, response } = acceptance.respond;
            this.#state.setWebSocketAutoResponse(
                new WebSocketRequestResponsePair(request, response),
            );
        }

        // answer the upgrade with the client's end
        const upgrade: Upgrade = {
            status: 101,
            webSocket: pair[0],
            ...(acceptance.protocol === undefined
                ? {}
                : { headers: { "sec-websocket-protocol": acceptance.protocol } }),
        };

        return new Response(null, upgrade);
    }

    /** Send an accepted WebSocket's event to the workload, as its description. */
    async #event(socket: HibernatingSocket, request: RequestInit): Promise<void> {
        // describe the socket to the running workload
        const description = SocketDescription.parse(socket.deserializeAttachment());
        const started = await this.#started();
        if (started === undefined) {
            socket.close();

            return;
        }
        Attribution.publish(Attribution.of(started.workload.start));

        // hand the event over below the socket's path
        const id = encodeURIComponent(description.id);
        const response = await this.#facet(started).fetch(
            new Request(`https://workload${SOCKET_PATH}/${id}`, {
                ...request,
                headers: {
                    authorization: `Bearer ${started.workload.start.secret}`,
                    [SOCKET_HEADER]: JSON.stringify(description),
                },
            }),
        );
        await response.arrayBuffer();
    }

    /** Answer the workload's listing, send or close of its accepted sockets. */
    async #answerSockets(request: Request, pathname: string): Promise<Response> {
        // require the running workload's secret, comparing the credentials' digests
        const started = await this.#started();
        const secret = started?.workload.start.secret;
        const { crypto } = this.#environment;
        const given = await Digest.of(request.headers.get("authorization") ?? "", crypto);
        const isSecret =
            secret !== undefined && given === (await Digest.of(`Bearer ${secret}`, crypto));
        if (!isSecret) {
            throw MachineError.of("INSTANCE_SECRET_INVALID", "invalid instance secret");
        }

        // list every accepted socket's description
        const id = decodeURIComponent(pathname.slice(SOCKET_PATH.length + 1));
        if (pathname === SOCKET_PATH) {
            return Response.json(
                this.#state
                    .getWebSockets()
                    .map((socket) => SocketDescription.parse(socket.deserializeAttachment())),
            );
        }

        // send to, close or replace the stored value of the socket of the identifier
        const [socket] = this.#state.getWebSockets(id);
        // drop a call to a closed socket, as a WebSocket discards sends once closed
        if (socket === undefined) {
            await request.body?.cancel();
        }
        // close the socket
        else if (request.method === "DELETE") {
            socket.close();
        }
        // replace its stored value
        else if (request.method === "PUT") {
            const description = SocketDescription.parse(socket.deserializeAttachment());
            const attachment = schema.json().parse(await request.json());
            socket.serializeAttachment({ ...description, attachment });
        }
        // send to it otherwise
        else {
            socket.send(new Uint8Array(await request.arrayBuffer()));
        }

        return new Response(null, { status: 204 });
    }

    /** Run the workload's due controllers for at most ten minutes. */
    async alarm(): Promise<void> {
        // ring nothing before a start
        const started = await this.#started();
        if (started === undefined) {
            return;
        }
        Attribution.publish(Attribution.of(started.workload.start));

        // run the workload's controllers until the ring's deadline
        const ring: DurableObjectAlarm = { at: this.#environment.clock.now() + ALARM_MILLISECONDS };
        const response = await this.#facet(started).fetch(
            new Request(`https://workload${ALARM_PATH}`, {
                method: "POST",
                body: JSON.stringify(ring),
            }),
        );
        await response.arrayBuffer();
    }

    /** Attribute the invocation to the running workload's installation, none before a start. */
    async #attribute(): Promise<void> {
        const started = await this.#started();
        if (started !== undefined) {
            Attribution.publish(Attribution.of(started.workload.start));
        }
    }

    /** Read the stored start, absent before a start and after a stop. */
    #started(): Promise<DurableObjectInstanceStart | undefined> {
        this.#start ??= this.#state.storage.get(START_KEY).then(
            (stored) =>
                stored === undefined ? undefined : DurableObjectInstanceStart.parse(stored),
            (error: unknown) => {
                // read storage again after a failed read
                this.#start = undefined;
                throw error;
            },
        );

        return this.#start;
    }

    /** Drain the running workload and stop its facet, leaving its databases. */
    async #stop(): Promise<void> {
        // drain the workload of the stored start
        const started = await this.#started();
        if (started !== undefined) {
            const response = await this.#facet(started).fetch(
                new Request(`https://workload${STOP_PATH}`, { method: "POST" }),
            );
            await response.arrayBuffer();
        }

        // forget the start and stop the facet
        await this.#state.storage.delete(START_KEY);
        this.#start = Promise.resolve(undefined);
        this.#state.facets.abort(FACET, new Error("the instance's start changed"));
    }

    /** Get the facet. */
    #facet(started: DurableObjectInstanceStart | undefined): {
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        return this.#state.facets.get(FACET, () => ({
            class:
                started === undefined
                    ? this.#state.exports[DATABASE_EXPORT]
                    : this.#load(started).getDurableObjectClass(WORKLOAD_EXPORT),
        }));
    }

    /** Load a start's workload code into an isolate of its own. */
    #load(started: DurableObjectInstanceStart): ReturnType<WorkerLoader["get"]> {
        // key the isolate by the instance, its code read through the space serving the build
        const bindings = this.#bindings;
        const self = bindings.PARTITION.get(this.#state.id);
        const space = this.#space(started.space);
        const { secret, instance } = started.workload.start;

        return bindings.LOADER.get(instance, async () => ({
            compatibilityDate: WORKERD_COMPATIBILITY_DATE,
            compatibilityFlags: ["nodejs_compat"],
            mainModule: started.main,
            modules: await PartitionFacet.#modules(space, secret),
            env: { PARTITION: self },
            globalOutbound: space,
        }));
    }

    /** Call a build's export in an isolate of its own, keyed by the call's secret. */
    async #call(call: DurableObjectCall): Promise<Response> {
        // load the output's modules beside the module calling the export a request names
        const space = this.#space(call.space);
        const main = JSON.stringify(`./${call.main}`);
        const isolate = this.#bindings.LOADER.get(call.secret, async () => ({
            compatibilityDate: WORKERD_COMPATIBILITY_DATE,
            compatibilityFlags: ["nodejs_compat"],
            mainModule: CALL_MODULE,
            modules: {
                ...(await PartitionFacet.#modules(space, call.secret)),
                [CALL_MODULE]: {
                    js: `import * as module from ${main};\nexport default {\n    async fetch(request) {\n        const { name, input } = await request.json();\n        return Response.json((await module[name](...input)) ?? null);\n    },\n};\n`,
                },
            },
            env: {},
            globalOutbound: space,
        }));

        // call the export with the call's input
        return await isolate.getEntrypoint().fetch(
            new Request("https://call", {
                method: "POST",
                body: JSON.stringify({ name: call.name, input: call.input }),
            }),
        );
    }

    /** Read a start's modules through the space serving its builds, in one request. */
    static async #modules(
        space: { readonly fetch: (request: Request) => Promise<Response> },
        secret: string,
    ): Promise<Record<string, { readonly js: string }>> {
        // read every module the start reaches, refusing an answer of a failure
        const response = await space.fetch(
            new Request(`https://space${BUILD_PATH}`, {
                headers: { authorization: `Bearer ${secret}` },
            }),
        );
        if (!response.ok) {
            throw MachineError.of(
                "MODULE_NOT_SERVED",
                `the space serves no modules: ${response.status} ${await response.text()}`,
            );
        }
        const modules = schema
            .record(schema.string(), schema.string())
            .parse(await response.json());

        // type each as a module explicitly, whatever its extension, such as an npm build's .mjs
        return Object.fromEntries(Object.entries(modules).map(([name, js]) => [name, { js }]));
    }

    /** Get a space's object, in its jurisdiction. */
    #space(space: string): {
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        const bindings = this.#bindings;
        const namespace = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);

        return namespace.get(namespace.idFromName(PartitionName.of({ space })));
    }
}

export type { DurableObjectNamespace };
