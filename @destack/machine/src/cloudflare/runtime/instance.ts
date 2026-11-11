import type { Environment } from "@destack/environment";
import type { Alarm, AlarmClock } from "@destack/db";
import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import {
    CONTENT_PATH,
    DATABASE_PATH,
    type DurableObjectNamespace,
    Jurisdiction,
} from "@destack/db/cloudflare";
import { Digest, present, schema } from "@destack/schema";
import { Bearer } from "@destack/access/token";
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

/** The prefix of each installation's start the object stores, resumed after an eviction. */
const START_PREFIX = "start:";

/** The class of the static object that answers database operations before a workload starts. */
const DATABASE_EXPORT = "DurableObjectDatabase";

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
        /** List the values below a prefix. */
        readonly list: (options: { readonly prefix: string }) => Promise<Map<string, unknown>>;
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
    /** Read the tags a WebSocket was accepted under. */
    readonly getTags: (socket: HibernatingSocket) => string[];
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

    /** Read the space, installation and partition an object name addresses. */
    read(name: string): {
        readonly space: string;
        readonly installation?: string;
        readonly partition?: string;
    } {
        const [space = "", installation, ...partition] = name.split("--").toReversed();

        return {
            space,
            ...(installation === undefined ? {} : { installation }),
            ...(partition.length === 0 ? {} : { partition: partition.toReversed().join("--") }),
        };
    },
};

/** One installation's workload in a facet of its space's object or its partition's. */
export class InstallationFacet {
    /** The installation, which names the facet, its stored start, its alarm and its sockets. */
    readonly installation: string;
    /** The object's state. */
    readonly #state: PartitionState;
    /** The Worker's bindings. */
    readonly #bindings: PartitionBindings;
    /** The clock the object's alarms ring at and the telemetry its refusals report to. */
    readonly #environment: Pick<Environment, "clock" | "telemetry" | "crypto">;
    /** The installation's alarm on the object's clock. */
    readonly #alarm: Alarm;
    /** The stored start, read from storage once; this facet is its only writer. */
    #start: Promise<DurableObjectInstanceStart | undefined> | undefined;

    /** Host an installation's facet in an object, its alarm on the object's clock. */
    constructor(
        installation: string,
        state: PartitionState,
        bindings: PartitionBindings,
        environment: Pick<Environment, "clock" | "telemetry" | "crypto">,
        alarm: Alarm,
    ) {
        // keep the installation, the object and its alarm
        this.installation = installation;
        this.#state = state;
        this.#bindings = bindings;
        this.#environment = environment;
        this.#alarm = alarm;
    }

    /** Require a workload's call to prove the running start's secret. */
    async #requireWorkload(request: Request): Promise<void> {
        const started = await this.#started();
        if (started?.workload.start.secret !== Bearer.require(request.headers)) {
            throw MachineError.of(
                "INSTANCE_SECRET_UNAUTHENTICATED",
                "authentication required: the instance secret is invalid",
            );
        }
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
                await this.#state.storage.put(`${START_PREFIX}${this.installation}`, started);
                this.#start = Promise.resolve(started);

                return await this.#facet(started).fetch(
                    new Request(`https://workload${START_PATH}`, {
                        method: "POST",
                        body: JSON.stringify(started.workload),
                    }),
                );
            }
            // drain the workload and forget its start
            else if (pathname === STOP_PATH) {
                await this.#stop();

                return Response.json({});
            }
            // move the wake-up the workload asks for
            else if (pathname === ALARM_PATH) {
                await this.#requireWorkload(request);
                const { at } = DurableObjectAlarm.parse(await request.json());
                await (at === null ? this.#alarm.deleteAlarm() : this.#alarm.setAlarm(at));

                return Response.json({});
            }
            // answer the workload's sends, closes and listing of its accepted sockets
            else if (pathname === SOCKET_PATH || pathname.startsWith(`${SOCKET_PATH}/`)) {
                await this.#requireWorkload(request);

                return await this.#answerSockets(request, pathname);
            }
            // read a build file or a database copy's content through the space serving it
            else if (
                pathname.startsWith(`${BUILD_PATH}/`) ||
                pathname.startsWith(`${CONTENT_PATH}/`)
            ) {
                const started = await this.#started();
                if (started === undefined) {
                    throw MachineError.of(
                        "WORKLOAD_UNAVAILABLE",
                        "workload is unavailable: no workload runs here",
                    );
                }

                return await this.#space(started.space).fetch(request);
            }

            // hand a database operation or a request to the facet
            const started = await this.#started();
            if (started === undefined && pathname !== DATABASE_PATH) {
                throw MachineError.of(
                    "WORKLOAD_UNAVAILABLE",
                    "workload is unavailable: no workload runs here",
                );
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
        this.#state.acceptWebSocket(pair[1], [
            this.installation,
            `${this.installation}/${description.id}`,
        ]);
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
            throw MachineError.of(
                "INSTANCE_SECRET_UNAUTHENTICATED",
                "authentication required: the instance secret is invalid",
            );
        }

        // list every accepted socket's description
        const id = decodeURIComponent(pathname.slice(SOCKET_PATH.length + 1));
        if (pathname === SOCKET_PATH) {
            return Response.json(
                this.#state
                    .getWebSockets(this.installation)
                    .map((socket) => SocketDescription.parse(socket.deserializeAttachment())),
            );
        }

        // send to, close or replace the stored value of the socket of the identifier
        const [socket] = this.#state.getWebSockets(`${this.installation}/${id}`);
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
        this.#start ??= this.#state.storage.get(`${START_PREFIX}${this.installation}`).then(
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
        await this.#state.storage.delete(`${START_PREFIX}${this.installation}`);
        this.#start = Promise.resolve(undefined);
        this.#state.facets.abort(this.installation, new Error("the instance's start changed"));
    }

    /** Get the facet. */
    #facet(started: DurableObjectInstanceStart | undefined): {
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        return this.#state.facets.get(this.installation, () => ({
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
            modules: await InstallationFacet.#modules(space, secret),
            env: { PARTITION: self },
            globalOutbound: space,
        }));
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
                "MODULE_UNREACHABLE",
                `could not read the space's modules: ${response.status} '${await response.text()}'`,
            );
        }
        const modules = schema
            .record(schema.string(), schema.string())
            .parse(await response.json());

        // type each as a module explicitly, whatever its extension, such as an npm build's .mjs
        return Object.fromEntries(Object.entries(modules).map(([name, js]) => [name, { js }]));
    }

    /** Get a space's object, in its jurisdiction. */
    #space(space: string): { readonly fetch: (request: Request) => Promise<Response> } {
        const bindings = this.#bindings;
        const namespace = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);

        return namespace.get(namespace.idFromName(PartitionName.of({ space })));
    }
}

/** The facets an object hosts, by installation, each with its party on the object's one alarm. */
export class FacetHost {
    /** The object's state. */
    readonly #state: PartitionState;
    /** The Worker's bindings. */
    readonly #bindings: PartitionBindings;
    /** The clock the facets' alarms ring at and the telemetry their refusals report to. */
    readonly #environment: Pick<Environment, "clock" | "telemetry" | "crypto">;
    /** The object's one alarm. */
    readonly #alarms: AlarmClock;
    /** The facets hosted so far, by installation. */
    readonly #facets = new Map<string, InstallationFacet>();

    /** Host facets in an object, their alarms on its clock. */
    constructor(
        state: PartitionState,
        bindings: PartitionBindings,
        environment: Pick<Environment, "clock" | "telemetry" | "crypto">,
        alarms: AlarmClock,
    ) {
        // keep the object and its alarm
        this.#state = state;
        this.#bindings = bindings;
        this.#environment = environment;
        this.#alarms = alarms;
    }

    /** Read an installation's facet, hosting it once. */
    facet(installation: string): InstallationFacet {
        // reuse the installation's facet, else host it
        const known = this.#facets.get(installation);
        if (known !== undefined) {
            return known;
        }
        const facet = new InstallationFacet(
            installation,
            this.#state,
            this.#bindings,
            this.#environment,
            this.#alarms.alarm(installation),
        );
        this.#facets.set(installation, facet);

        return facet;
    }

    /** Hand a request to the facet its first path segment addresses, absent for another path. */
    route(request: Request): Promise<Response> | undefined {
        // read the installation the path starts with, leaving any other path to the object
        const url = new URL(request.url);
        const [, first = "", ...below] = url.pathname.split("/");
        const installation = schema.identifier("installation").safeParse(first);
        if (!installation.success) {
            return undefined;
        }

        // hand the facet the path below the installation
        url.pathname = `/${below.join("/")}`;

        return this.facet(installation.data).fetch(new Request(url.href, request));
    }

    /** Read the facet of the installation that accepted a WebSocket, its first tag. */
    socket(socket: HibernatingSocket): InstallationFacet {
        const [installation] = this.#state.getTags(socket);

        return this.facet(present(installation, "the WebSocket's installation"));
    }
}

export type { DurableObjectNamespace };
