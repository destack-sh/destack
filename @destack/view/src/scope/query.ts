import { account, space, user } from "@destack/account/object";
import { type Method, type MutationName, type ObjectType } from "@destack/object";
import { type JsonObject, mapValues, present } from "@destack/schema";
import {
    type Mutator,
    type ObjectNode,
    type ObjectQueries,
    ObjectQueryBuilder,
    type RelationalQuery,
    type Subscriber,
} from "@destack/object/client";
import { type ContextScope, ViewContext } from "../declare/context.ts";
import { useMount } from "../page/context.ts";
import { type Accessor, createMemo, onCleanup } from "../solid/reactive.ts";
import { isServer } from "../solid/component.ts";

/** Where a hook reads and writes: a scope of the view's context, or a scope by its identifier. */
export interface ScopeOptions {
    /** The scope, by default the one the object type lives in for the view. */
    readonly scope?: ContextScope | Accessor<string>;
}

/** Follow a query of some object types live, on the view's node of the queried type's scope. */
export function useQuery<const Objects extends Readonly<Record<string, ObjectType>>, Value>(
    objects: Objects,
    query: (builders: ObjectQueries<Objects>) => RelationalQuery<Value>,
    options: ScopeOptions = {},
): Accessor<Value> {
    return createMemo(() => {
        // build the query over the node it reads on, tracking what it reads
        let node: ObjectNode | undefined;
        const current = () => present(node, "the query's node");
        const subscriber: Subscriber = {
            rows: (object, read, live) => current().subscriber.rows(object, read, live),
            results: (object, read, live) => current().subscriber.results(object, read, live),
        };
        const built = query(ObjectQueryBuilder.of(subscriber, objects));

        // follow it on its type's node until it or what it reads changes, or the component goes
        const scoped = nodeOf(built.object, options, useMount());
        if (scoped.kind === "open") {
            node = scoped.node;

            return follow(built, scoped.signal);
        }

        // else follow it once the node opens, as the mount's clients replace the opening node
        const opening = scoped.node.then((opened) => {
            node = opened;
        });

        return following(opening, built, scoped.signal);
    });
}

/** Follow a query once its node opens. */
async function* following<Value>(
    opening: Promise<void>,
    query: RelationalQuery<Value>,
    signal: AbortSignal,
): AsyncGenerator<Value, void> {
    await opening;
    yield* follow(query, signal);
}

/** Call an object type's mutating method on the view's node of its scope, predicted there first. */
export function useMutation<Object extends ObjectType, Name extends MutationName<Object>>(
    object: Object,
    name: Name,
    options: ScopeOptions = {},
): Mutator<Object>[Name] {
    // call on the current node, once it opened where it opens later
    const nodes = useNode(object, options);
    const run = async (input: JsonObject) => {
        const node = await nodes().node;

        return node.call(object, name, input);
    };

    // build the method's mutator, as a node's mutate does
    const method = present(object.methods[name], `the method '${name}' of '${object.name}'`);

    return mapValues<Readonly<Record<string, Method>>, Mutator<Object>>(
        { [name]: method },
        () => (input: JsonObject) => {
            // take the node's prediction and confirmation once the call reached it
            const submitted = run(input);
            const confirmed = submitted.then((submission) => submission.confirmed);
            confirmed.catch(() => {});

            return { predicted: submitted.then((submission) => submission.predicted), confirmed };
        },
    )[name];
}

/** A scope's node for a hook, held in an object since a memo of a promise awaits it. */
type ScopeNode = (
    | {
          /** The node is open. */
          readonly kind: "open";
          /** The node. */
          readonly node: ObjectNode;
      }
    | {
          /** The node is opening. */
          readonly kind: "opening";
          /** The node once it opened. */
          readonly node: Promise<ObjectNode>;
      }
) & {
    /** Aborts once the scope changes or the component goes. */
    readonly signal: AbortSignal;
};

/** Read the view's node of an object type's scope, opening a scope outside the view's context. */
export function useNode(object: ObjectType, options: ScopeOptions): Accessor<ScopeNode> {
    return createMemo(() => nodeOf(object, options, useMount()));
}

/** Take the mount's node of an object type's scope, opening one outside the view's context. */
function nodeOf(
    object: ObjectType,
    options: ScopeOptions,
    mount: ReturnType<typeof useMount>,
): ScopeNode {
    // refuse a node that does not hold the type, on first read as every read of the mount
    const held = (node: ObjectNode) => {
        if (!node.objects.some((type) => type.same(object))) {
            throw new TypeError(`the node has no object ${object.name}`);
        }

        return node;
    };

    // stop once the scope changes or the component goes
    const stopping = new AbortController();
    onCleanup(() => stopping.abort());

    // take an opened node of the scope, else open it
    const scope = scopeOf(mount.context, object, options);
    const opened = mount.clients().get(scope);
    if (opened !== undefined) {
        return { kind: "open", node: held(opened), signal: stopping.signal };
    }

    return {
        kind: "opening",
        node: mount.open(scope, stopping.signal).then(held),
        signal: stopping.signal,
    };
}

/** Read the scope a hook reads and writes: the one asked, else the one the object type lives in. */
export function scopeOf(view: ViewContext, object: ObjectType, options: ScopeOptions): string {
    // take the scope asked, a context scope by name or a scope by identifier
    const { scope } = options;
    if (typeof scope === "function") {
        return scope();
    } else if (scope !== undefined) {
        return ViewContext.scope(view, scope);
    }

    // take the account or the person for a type living there alone, else the view's space, which holds nested containers too
    if (!object.livesInType(space) && object.livesInType(account)) {
        return ViewContext.scope(view, "account");
    } else if (!object.livesInType(space) && object.livesInType(user)) {
        return view.user.id;
    } else {
        return ViewContext.scope(view, "space");
    }
}

/** Follow one query until the signal aborts, or read it once complete in a server render. */
async function* follow<Value>(
    query: RelationalQuery<Value>,
    signal: AbortSignal,
): AsyncGenerator<Value> {
    const live = query.subscribe();
    try {
        // render one complete snapshot on the server, which ends the page's stream
        if (isServer) {
            await live.ready;
            yield await live.read();
        }
        // else follow it live
        else {
            for await (const value of live.watch(signal)) {
                yield value;
            }
        }
    } finally {
        await live.close();
    }
}
