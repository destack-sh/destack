import type { ObjectType } from "@destack/object";
import type { Mutation, ObjectAccess, ObjectClient, Submission } from "@destack/object/client";
import type { JsonObject } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { useClient, useView } from "../page/view.ts";

/** An object of a scope: its type and its identifier. */
export interface ScopeObject {
    /** The object's type. */
    readonly type: ObjectType;
    /** The object's identifier. */
    readonly id: string;
}

/** The commands of a scope that name objects and methods at run time: permission checks and calls by name. */
export interface ScopeCommands {
    /** Decide whether the person holds a permission on an object, the view's target by default. */
    can(permission: string, object?: ScopeObject): Promise<boolean>;
    /** Call an object type's mutating method by name as one mutation. */
    call(object: ObjectType, method: string, input: JsonObject): Submission<unknown>;
}

/** One of a view's scopes: its object types' queries and mutators by key, atomic mutations and the person's history. */
export interface ScopeAccess<Objects extends Readonly<Record<string, ObjectType>>>
    extends ObjectAccess<Objects>, ScopeCommands {
    /** Make several calls as one atomic mutation, predicted at once and undone as one step. */
    mutation<Result>(run: (mutation: Mutation) => Promise<Result>): Submission<Result>;
    /** Undo the person's last mutation, resolving whether there was one. */
    undo(): Submission<boolean>;
    /** Redo the person's last undone mutation, resolving whether there was one. */
    redo(): Submission<boolean>;
}

/** Read and change object types of the view's space by key. */
export function useSpace<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ScopeAccess<Objects> {
    const view = useView();

    return access(useClient(view.space), objects, view.target);
}

/** Read and change object types of the view's account by key. */
export function useAccount<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ScopeAccess<Objects> {
    const view = useView();

    return access(useClient(view.account), objects, view.target);
}

/** Read and change object types of the person's home space by key, refusing a view the host opens no home for. */
export function useHome<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ScopeAccess<Objects> {
    const home = useView().home;
    if (home === undefined) {
        throw new TypeError("the host opens no home space for this view");
    }

    return access(useClient(home), objects, undefined);
}

/** Give one scope's client as the access to some of its object types. */
function access<const Objects extends Readonly<Record<string, ObjectType>>>(
    client: ObjectClient,
    objects: Objects,
    target: ObjectReference | undefined,
): ScopeAccess<Objects> {
    return {
        ...client.of(objects),
        mutation: (run) => client.mutation(run),
        undo: () => client.undo(),
        redo: () => client.redo(),
        can: (permission, object) => {
            const checked = object ?? targetOf(objects, target);

            return client.can(checked.type, checked.id, permission);
        },
        call: (object, method, input) => client.call(object, method, input),
    };
}

/** Find the view's target among a scope's object types, refusing a view without one or a type the access lacks. */
function targetOf(
    objects: Readonly<Record<string, ObjectType>>,
    target: ObjectReference | undefined,
): ScopeObject {
    const type =
        target === undefined
            ? undefined
            : Object.values(objects).find(
                  (entry) =>
                      ObjectReference.key(entry.reference(target.scope, target.id)) ===
                      ObjectReference.key(target),
              );
    if (target === undefined || type === undefined) {
        throw new TypeError(
            "can needs an object outside a view opened for one of the access's types",
        );
    }

    return { type, id: target.id };
}
