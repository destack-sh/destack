import type { Environment } from "@destack/environment";
import { type JsonObject, JsonValue, present } from "@destack/schema";
import type { ObjectReference, Subject } from "@destack/sync";
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
    /** Call an object type's mutating method as one mutation, then read as of its prediction. */
    readonly call: (
        object: ObjectType,
        name: string,
        input: JsonObject,
        then?: (mutation: Pick<MutationContext, "read">) => Promise<void>,
    ) => Submission<unknown>;
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
    /** Read an object's row, from the server where the node lacks it. */
    readonly find: (
        type: ObjectType,
        reference: ObjectReference,
    ) => Promise<JsonObject | undefined>;
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

    /** Run an action's call: ephemeral and local calls one by one, any other as one mutation. */
    perform<Inputs extends ActionInputs>(
        node: Pick<ObjectNode, "context" | "find" | "call">,
        action: Action<Inputs>,
        { values, initial = {} }: ActionArguments<Inputs>,
        mutation: ObjectNode["mutation"],
    ): Submission<readonly unknown[]> {
        // resolve the call's inputs with the node's reads
        const call = present(action.call, `the call of action '${action.name}'`);
        const invocations = call.invocations(
            { values: JsonValue.object(values), initial },
            { context: node.context(), read: (type, reference) => node.find(type, reference) },
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
