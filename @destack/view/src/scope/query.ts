import { account, space, user } from "@destack/account/object";
import { type Method, type MutationName, type ObjectType } from "@destack/object";
import { type JsonObject, mapValues, present } from "@destack/schema";
import type {
    Mutator,
    ObjectNode,
    ObjectQueryBuilder,
    RelationalQuery,
} from "@destack/object/client";
import { useContext } from "solid-js";
import { type ContextScope, ViewContext } from "../declare/context.ts";
import { useView, ViewMountContext } from "../page/context.ts";
import { type Accessor, createMemo, onCleanup } from "../solid/reactive.ts";

/** Where a hook reads and writes: a scope of the view's context, or a scope by its identifier. */
export interface ScopeOptions {
    /** The scope, by default the one the object type lives in for the view. */
    readonly scope?: ContextScope | Accessor<string>;
}

/** Follow a query of an object type live on the view's node of its scope. */
export function useQuery<Object extends ObjectType, Value>(
    object: Object,
    query: (builder: ObjectQueryBuilder<Object, ObjectType>) => RelationalQuery<Value>,
    options: ScopeOptions = {},
): Accessor<Value> {
    const nodes = useNode(object, options);

    return createMemo(() => {
        // follow the query on the current node until either changes or the component goes
        const node = nodes();
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());

        return follow(node.node, (opened) => query(opened.queryType(object)), stopping.signal);
    });
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
interface ScopeNode {
    /** The node, or the node once it opened. */
    readonly node: ObjectNode | Promise<ObjectNode>;
}

/** Read the view's node of an object type's scope, opening a scope outside the view's context. */
export function useNode(object: ObjectType, options: ScopeOptions): Accessor<ScopeNode> {
    // fail at the hook's call for a type the view's node of the scope does not hold
    const view = useView();
    const mount = useContext(ViewMountContext);
    const held = (node: ObjectNode) => {
        if (!node.objects.some((type) => type.same(object))) {
            throw new TypeError(`the node has no object ${object.name}`);
        }

        return node;
    };
    const known = mount.clients.get(scopeOf(view, object, options));
    if (known !== undefined) {
        held(known);
    }

    // follow the scope's node
    return createMemo((): ScopeNode => {
        // take an opened node of the scope
        const scope = scopeOf(view, object, options);
        const opened = mount.clients.get(scope);
        if (opened !== undefined) {
            return { node: held(opened) };
        }

        // open the scope's node until the scope changes or the component goes
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());

        return { node: mount.open(scope, stopping.signal).then(held) };
    });
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

/** Follow one query on a node until the signal aborts. */
async function* follow<Value>(
    node: ObjectNode | Promise<ObjectNode>,
    query: (node: ObjectNode) => RelationalQuery<Value>,
    signal: AbortSignal,
): AsyncGenerator<Value> {
    const live = query(await node).subscribe();
    try {
        for await (const value of live.watch(signal)) {
            yield value;
        }
    } finally {
        await live.close();
    }
}
