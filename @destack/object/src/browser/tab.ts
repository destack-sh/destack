import { type JsonObject, mapValues, schema } from "@destack/schema";
import { broadcastChannel } from "@destack/db/channel";
import { type Environment, waitAborted } from "@destack/environment";
import type {} from "@destack/environment/browser";
import { type ObjectReference, RequestId, type Subject } from "@destack/sync";
import { ObjectError } from "../error/index.ts";
import {
    type MutationContext,
    type Mutator,
    type ObjectAccess,
    type ObjectMutators,
    type ObjectQueries,
    type Submission,
} from "../client/client.ts";
import { ObjectNode } from "../client/node.ts";
import { ObjectQueryBuilder, type Subscriber } from "../client/query.ts";
import { TextDocument } from "../client/text.ts";
import type { Action } from "../declare/action.ts";
import type { ActionArguments } from "../declare/call.ts";
import type { Place } from "../declare/place.ts";
import type { ActionContext } from "../declare/template.ts";
import type { ActionInputs } from "../declare/variable.ts";
import { Method } from "../method/method.ts";
import type { ObjectType, PermissionOf } from "../object/object.ts";
import { ChannelRemote } from "../replica/channel.ts";
import type {} from "@destack/package/import-meta";

/** What the device worker tells the tab that started it: it serves, or it failed to open. */
const WorkerAnswer = schema.discriminatedUnion("kind", [
    schema.object({ kind: schema.literal("serving") }),
    schema.object({ kind: schema.literal("failed"), message: schema.string() }),
]);

/** The browser effects a tab takes: the origin's locks, broadcast channels and storage. */
export type BrowserTabEnvironment = Environment<"locks" | "broadcast" | "storage">;

/** Start the device worker, returning how to stop it. */
export type BrowserTabWorker = () => Promise<() => Promise<void>>;

/** A tab's node of one scope, its reads and writes forwarded to the person's device. */
export class BrowserTab<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> implements ObjectNode<Objects> {
    /** The scope whose objects the tab reads and writes. */
    readonly scope: string;
    /** The calling principal. */
    readonly caller: Subject;
    /** The object types the tab reads and writes. */
    readonly objects: readonly ObjectType[];
    /** The environment the tab runs in. */
    readonly environment: Environment;
    /** Follow queries live at the device, predictions included. */
    readonly subscriber: Subscriber;
    /** The relational queries of the object types, by key. */
    readonly query: ObjectQueries<Objects>;
    /** The person's undo and redo at the device. */
    readonly undoStack: ObjectNode["undoStack"];
    /** Settles once the device has the caller's access rows, or refuses with why not. */
    readonly ready: Promise<void>;
    /** The device the tab reads and writes through. */
    readonly #remote: ChannelRemote;

    /** Read and write one scope through the device on a channel. */
    private constructor(
        remote: ChannelRemote,
        options: {
            readonly objects: Objects;
            readonly scope: string;
            readonly caller: Subject;
            readonly election: DeviceElection;
            readonly environment: Environment;
        },
    ) {
        // read and write the scope's object types at the device
        this.#remote = remote;
        this.scope = options.scope;
        this.caller = options.caller;
        this.objects = Object.values(options.objects);
        this.environment = options.environment;
        this.subscriber = remote.subscriber(options.scope, this.objects);
        this.query = ObjectQueryBuilder.of(this.subscriber, options.objects);
        this.undoStack = remote.undoStack(options.scope);

        // be ready once the device has the scope's access rows, refusing once the election fails
        const ready = remote.ready(options.scope);
        this.ready = Promise.race([ready, options.election.failed]);
        this.ready.catch(() => {});
    }

    /** Open a tab's node of one scope over the person's device on the origin's channel. */
    static open<const Objects extends Readonly<Record<string, ObjectType>>>(options: {
        /** The device's name, shared by every tab that opens the same device. */
        readonly name: string;
        /** The object types the tab reads and writes, by key. */
        readonly objects: Objects;
        /** The scope whose objects the tab reads and writes. */
        readonly scope: string;
        /** The calling principal. */
        readonly caller: Subject;
        /** The page's election of the device, which starts its worker once the page leads. */
        readonly election: DeviceElection;
        /** The environment the tab runs in, with the browser's channels. */
        readonly environment: Environment & BrowserTabEnvironment;
    }): BrowserTab<Objects> {
        // join the device's channel as one party
        const { environment } = options;
        const channel = broadcastChannel(`destack:${options.name}`, environment);
        const remote = new ChannelRemote(channel, RequestId.create(environment), environment);

        return new BrowserTab(remote, options);
    }

    /** Read and change some of the tab's object types by key, refusing types it lacks. */
    of<const Selected extends Readonly<Record<string, ObjectType>>>(
        objects: Selected,
    ): ObjectAccess<Selected> {
        // require the tab's types
        for (const object of Object.values(objects)) {
            if (!this.objects.some((type) => type.same(object))) {
                throw new TypeError(`the tab has no object '${object.name}'`);
            }
        }

        return {
            query: ObjectQueryBuilder.of(this.subscriber, objects),
            mutate: mapValues<Selected, ObjectMutators<Selected>>(objects, (object) =>
                this.mutate(object),
            ),
            mutation: (run) => this.mutation(run),
            undoStack: this.undoStack,
            call: (object, name, input) => this.call(object, name, input),
            check: (object, id, permission) => this.check(object, id, permission),
        };
    }

    /** Query one of the tab's object types. */
    queryType<Object extends ObjectType>(
        object: Object,
    ): ObjectQueryBuilder<Object, Objects[keyof Objects]> {
        return new ObjectQueryBuilder(this.subscriber, object);
    }

    /** Call an object type's mutating methods, one mutation per call. */
    mutate<Object extends ObjectType>(object: Object): Mutator<Object> {
        return mapValues<Readonly<Record<string, Method>>, Mutator<Object>>(
            Object.fromEntries(
                Object.entries(object.methods).filter(
                    ([, method]) => method.mutates && Method.isRouted(method),
                ),
            ),
            (_method, name) => (input: JsonObject) => this.call(object, name, input),
        );
    }

    /** Call an object type's mutating method by name as one mutation, predicted at the device. */
    call(object: ObjectType, name: string, input: JsonObject): Submission<unknown> {
        return this.#remote.mutate(this.scope, object, name, input);
    }

    /** Predict several calls together at the device and queue them as one mutation. */
    mutation<Value>(run: (mutation: MutationContext) => Promise<Value>): Submission<Value> {
        return this.#remote.mutation(this.scope, run);
    }

    /** Run an action's call on resolved inputs, predicted at the device. */
    perform<Inputs extends ActionInputs>(
        action: Action<Inputs>,
        values: ActionArguments<Inputs>,
    ): Submission<readonly unknown[]> {
        return ObjectNode.perform(this, action, values, (run) => this.mutation(run));
    }

    /** Read what actions read here: who the tab is for, its scope, the time, and some places. */
    context(places: readonly Place[] = []): ActionContext {
        return ObjectNode.context(this, places);
    }

    /** Read an object's row at the device, from the server where the device lacks it. */
    find(type: ObjectType, reference: ObjectReference): Promise<JsonObject | undefined> {
        return this.#remote.find(this.scope, type, reference);
    }

    /** Follow one text field of an object as an editor edits it. */
    text(object: ObjectType, id: string, field: string): TextDocument {
        return new TextDocument(this, object, id, field);
    }

    /** Read a scope's chain at the device. */
    chain(scope: string): Promise<readonly string[]> {
        return this.#remote.chain(this.scope, scope);
    }

    /** Check at the device whether the caller has a permission on an object. */
    check<Object extends ObjectType>(
        object: Object,
        id: string,
        permission: PermissionOf<Object>,
    ): Promise<boolean> {
        return this.#remote.check(this.scope, object, id, permission);
    }

    /** Leave the device's channel. */
    async close(): Promise<void> {
        await this.#remote.close();
    }

    /** Start a module's device worker at once, asking it for the device once the page can. */
    static worker(
        module: URL,
        name: string,
        request: Promise<JsonObject>,
        environment: Environment<"storage" | "workers">,
    ): BrowserTabWorker {
        return async () => {
            // ask to persist the storage beside the start, as a browser may wait for the person's answer
            void environment.storage
                .persist()
                .catch((error: unknown) => globalThis.reportError(error));

            // start the device worker loading its modules, then ask for the device once the page knows its request
            const worker = environment.workers.start(
                (Worker) => new Worker(module, { type: "module", name: `destack:${name}` }),
            );
            worker.postMessage(await request);

            // wait until the worker serves the device, refusing one that cannot open its database or fails to load
            const message = await new Promise<unknown>((resolve, reject) => {
                worker.addEventListener("message", (event) => resolve(event.data));
                worker.addEventListener("error", (event) =>
                    reject(new Error(`the device worker failed: ${event.message}`)),
                );
            });
            const answer = WorkerAnswer.parse(message);
            if (answer.kind === "failed") {
                worker.terminate();
                throw ObjectError.of(
                    "DATABASE_UNAVAILABLE",
                    `database '${name}' is unavailable: this browser cannot open it: ${answer.message}`,
                );
            }

            return async () => worker.terminate();
        };
    }
}

/** A page's election to lead its device, running its worker while the page leads. */
export class DeviceElection {
    /** Rejects once the worker fails to start, which fails the page's tabs. */
    readonly failed: Promise<never>;
    /** Aborts the election, stopping a started worker. */
    readonly #stopping: AbortController;
    /** Settles once the page no longer leads or waits to. */
    readonly #leading: Promise<void>;

    /** Stand with a stopping controller, the settled election and its failure. */
    private constructor(stopping: AbortController, leading: Promise<void>, failed: Promise<never>) {
        this.#stopping = stopping;
        this.#leading = leading;
        this.failed = failed;
    }

    /** Stand to lead a device, starting its worker once the page leads. */
    static start(
        name: string,
        worker: BrowserTabWorker,
        environment: Environment<"locks">,
    ): DeviceElection {
        // run the worker while holding the device's lock, failing the election on its failure unless closing
        const stopping = new AbortController();
        const { signal } = stopping;
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => undefined);
        const lead = async () => {
            if (!signal.aborted) {
                const stop = await worker();
                try {
                    await waitAborted(signal);
                } finally {
                    await stop();
                }
            }
        };
        const leading = environment.locks
            .request(`destack:${name}:device`, { signal }, lead)
            .catch((error: unknown) => {
                if (!signal.aborted) {
                    failed.reject(error);
                }
            });

        return new DeviceElection(stopping, leading, failed.promise);
    }

    /** Leave the election, stopping a started worker. */
    async close(): Promise<void> {
        this.#stopping.abort();
        await this.#leading;
    }
}
