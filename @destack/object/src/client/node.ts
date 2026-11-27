import type { Environment } from "@destack/environment";
import { type JsonObject, JsonValue, present } from "@destack/schema";
import { ObjectReference, type Subject } from "@destack/sync";
import type { Action } from "../declare/action.ts";
import type { ActionArguments } from "../declare/call.ts";
import type { Place } from "../declare/place.ts";
import type { ActionContext } from "../declare/template.ts";
import type { ActionInputs } from "../declare/variable.ts";
import type { ObjectType, PermissionOf } from "../object/object.ts";
import type {
    LiveQuery,
    MutationContext,
    Mutator,
    ObjectAccess,
    ObjectQueries,
    SearchOptions,
    SearchResult,
    Submission,
} from "./access.ts";
import type { ObjectQueryBuilder, Subscriber } from "./query.ts";
import type { TextDocument } from "./text.ts";
import type { UndoStack } from "./undo.ts";
import { Observable } from "./observable.ts";

/** A node of one scope's objects that views read and write, on a database or over a channel. */
export interface ObjectNode<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> {
    /** The scope whose objects the node holds. */
    readonly scope: string;
    /** The calling principal. */
    readonly caller: Subject;
    /** The object types the node holds. */
    readonly objects: readonly ObjectType[];
    /** The environment the node runs in: its clock, network and randomness. */
    readonly environment: Environment;
    /** Follow queries live, predictions included. */
    readonly subscriber: Subscriber;
    /** The relational queries of the object types, by key. */
    readonly query: ObjectQueries<Objects>;
    /** Settles once the node has the caller's access rows. */
    readonly ready: Promise<void>;
    /** The party's undo and redo. */
    readonly undoStack: Pick<UndoStack, "undo" | "redo" | "latest" | "watch" | "close">;

    /** Read and change some of the node's object types by key. */
    readonly of: <const Selected extends Readonly<Record<string, ObjectType>>>(
        objects: Selected,
    ) => ObjectAccess<Selected>;
    /** Query one of the node's object types. */
    readonly queryType: <Object extends ObjectType>(
        object: Object,
    ) => ObjectQueryBuilder<Object, Objects[keyof Objects]>;
    /** Call an object type's mutating methods, one mutation per call. */
    readonly mutate: <Object extends ObjectType>(object: Object) => Mutator<Object>;
    /** Call an object type's mutating method by name as one mutation. */
    readonly call: (object: ObjectType, name: string, input: JsonObject) => Submission<unknown>;
    /** Predict several calls together and queue them as one mutation. */
    readonly mutation: <Value>(
        run: (mutation: MutationContext) => Promise<Value>,
    ) => Submission<Value>;
    /** Run an action's call on resolved inputs as one mutation. */
    readonly perform: <Inputs extends ActionInputs>(
        action: Action<Inputs>,
        values: ActionArguments<Inputs>,
    ) => Submission<readonly unknown[]>;
    /** Read what actions read here: who the node is for, its scope, the time, and some places. */
    readonly context: (places?: readonly Place[]) => ActionContext;
    /** Follow objects by reference across types and scopes, keyed by `ObjectReference.key`. */
    readonly get: (
        references: readonly ObjectReference[],
    ) => LiveQuery<ReadonlyMap<string, JsonObject>>;
    /** Read objects by reference once: the rows the node has, else the server's answer. */
    readonly fetch: (references: readonly ObjectReference[]) => Promise<readonly JsonObject[]>;
    /** Search the objects of some types live, the best and newest first. */
    readonly search: (text: string, options: SearchOptions) => LiveQuery<SearchResult>;
    /** Follow one text field of an object as an editor edits it. */
    readonly text: (object: ObjectType, id: string, field: string) => TextDocument;
    /** Read a scope and the scopes above it, nearest first, from the node's access rows. */
    readonly chain: (scope: string) => Promise<readonly string[]>;
    /** Check whether the caller has a permission on an object. */
    readonly check: <Object extends ObjectType>(
        object: Object,
        id: string,
        permission: PermissionOf<Object>,
    ) => Promise<boolean>;
    /** Stop following and close. */
    readonly close: () => Promise<void>;
}

/** The verbs every node runs over its own reads and writes. */
export const ObjectNode = {
    /** Read what actions read on a node: who it is for, its scope, the time, and some places. */
    context(
        node: Pick<ObjectNode, "caller" | "scope" | "environment">,
        places: readonly Place[],
    ): ActionContext {
        const { packageId, type, scope, id } = node.caller;
        const { clock, timeZone } = node.environment;

        return {
            subject: { packageId, type, scope, id },
            scope: node.scope,
            moment: { now: clock.now(), timeZone },
            places,
        };
    },

    /** Follow objects by reference on a node, one query per type over every readable scope. */
    get(
        node: Pick<ObjectNode, "objects" | "queryType">,
        references: readonly ObjectReference[],
    ): LiveQuery<ReadonlyMap<string, JsonObject>> {
        // group the identifiers by type, refusing a type the node does not hold
        const groups = new Map<ObjectType, string[]>();
        for (const reference of references) {
            const type = node.objects.find(
                (each) => each.package.id === reference.packageId && each.name === reference.type,
            );
            if (type === undefined) {
                throw new TypeError(`the node holds no type ${reference.type}`);
            }
            groups.set(type, [...(groups.get(type) ?? []), reference.id]);
        }

        // follow each type's rows by identifier, keyed by their references
        const lives = [...groups].map(([type, ids]) => ({
            type,
            live: node
                .queryType(type)
                .findMany({ scopes: "every", where: { id: { in: ids } } })
                .subscribe(),
        }));

        // key each row by its reference, leaving out the rows the caller cannot read
        const merge = (reads: readonly (readonly unknown[])[]): ReadonlyMap<string, JsonObject> =>
            new Map(
                reads.flatMap((items, index) =>
                    items.map((item): [string, JsonObject] => {
                        // read the row's place, which every object row has
                        const row = JsonValue.object(item);
                        const { scope, id } = row;
                        if (typeof scope !== "string" || typeof id !== "string") {
                            throw new TypeError("an object row has no scope or identifier");
                        }
                        const { type } = present(lives[index], "a followed type");

                        return [ObjectReference.key(type.reference(scope, id)), row];
                    }),
                ),
            );

        return {
            ready: Promise.all(lives.map(({ live }) => live.ready)).then(() => {}),
            read: async () => merge(await Promise.all(lives.map(({ live }) => live.read()))),
            watch: (signal) =>
                Observable.latest(
                    lives.map(({ live }) => live.watch(signal)),
                    merge,
                ),
            close: async () => {
                await Promise.all(lives.map(({ live }) => live.close()));
            },
        };
    },

    /** Read objects by reference once on a node: the rows it has, else the server's answer. */
    async fetch(
        node: Pick<ObjectNode, "get">,
        references: readonly ObjectReference[],
    ): Promise<readonly JsonObject[]> {
        // take the rows the node has, else wait for the server's answer
        const live = node.get(references);
        try {
            const known = await live.read();
            if (references.every((reference) => known.has(ObjectReference.key(reference)))) {
                return [...known.values()];
            }
            await live.ready;

            return [...(await live.read()).values()];
        } finally {
            await live.close();
        }
    },

    /** Run an action's call: ephemeral and local calls one by one, any other as one mutation. */
    perform<Inputs extends ActionInputs>(
        node: Pick<ObjectNode, "context" | "fetch" | "call">,
        action: Action<Inputs>,
        { values, initial = {} }: ActionArguments<Inputs>,
        mutation: ObjectNode["mutation"],
    ): Submission<readonly unknown[]> {
        // resolve the call's inputs with the node's reads
        const call = present(action.call, `the call of action '${action.name}'`);
        const invocations = call.invocations(
            { values: JsonValue.object(values), initial },
            {
                context: node.context(),
                read: async (_type, references) => node.fetch(references),
            },
        );

        // run an ephemeral or local method call by call, and any other as one mutation
        const { object, method } = call;
        const isSingle = object.storage === "ephemeral" || object.storage === "local";
        const submitted = invocations.then((each): Submission<readonly unknown[]>[] =>
            isSingle
                ? each.map(({ input }) => {
                      const { predicted, confirmed } = node.call(object, method, input);

                      return { predicted: predicted.then((result) => [result]), confirmed };
                  })
                : [
                      mutation(async (context) => {
                          const results: unknown[] = [];
                          for (const { input } of each) {
                              results.push(await context.invoke(object, method, input));
                          }

                          return results;
                      }),
                  ],
        );
        const predicted = submitted.then(async (each) =>
            (await Promise.all(each.map((submission) => submission.predicted))).flat(),
        );
        const confirmed = submitted.then(async (each) => {
            await Promise.all(each.map((submission) => submission.confirmed));
        });

        // mark the confirmation handled, and answer its failure in the returned submission
        confirmed.catch(() => {});

        return { predicted, confirmed };
    },
};
