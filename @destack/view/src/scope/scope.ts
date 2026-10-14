import type { ObjectType } from "@destack/object";
import type { Mutation, ObjectAccess, ObjectClient, Submission } from "@destack/object/client";
import type { JsonObject } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { requireScope, useClient, useView } from "../page/view.ts";

/** An object of a scope: its type and its identifier. */
export interface ScopeObject {
    /** The object's type. */
    readonly type: ObjectType;
    /** The object's identifier. */
    readonly id: string;
}

/** The commands of a scope that refer to objects and methods at run time. */
export interface ScopeCommands {
    /** Check whether the person has a permission on an object, the view's target by default. */
    check(permission: string, object?: ScopeObject): Promise<boolean>;
    /** Call an object type's mutating method by name as one mutation. */
    call(object: ObjectType, method: string, input: JsonObject): Submission<unknown>;
}

/** One of a view's scopes. */
export interface ScopeAccess<Objects extends Readonly<Record<string, ObjectType>>>
    extends ObjectAccess<Objects>, ScopeCommands {
    /** Make several calls as one atomic mutation, predicted at once and undone as one step. */
    mutation<Result>(run: (mutation: Mutation) => Promise<Result>): Submission<Result>;
    /** Undo the person's last mutation, resolving whether there was one. */
    undo(): Submission<boolean>;
    /** Redo the person's last undone mutation, resolving whether there was one. */
    redo(): Submission<boolean>;
}

/** Read and change object types of the view's space by key, as its space permissions grant. */
export function useSpace<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ScopeAccess<Objects> {
    const view = useView();

    return access(useClient(requireScope(view, "space")), objects, view.target);
}

/** Read and change object types of the view's account by key, as its account permissions grant. */
export function useAccount<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ScopeAccess<Objects> {
    const view = useView();

    return access(useClient(requireScope(view, "account")), objects, view.target);
}

/** Read and change object types of the person's home by key. */
export function useHome<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ScopeAccess<Objects> {
    return access(useClient(requireScope(useView(), "home")), objects, undefined);
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
        check: (permission, object) => {
            const checked = object ?? findTarget(objects, target);

            return client.check(checked.type, checked.id, permission);
        },
        call: (object, method, input) => client.call(object, method, input),
    };
}

/** Find the view's target among a scope's object types. */
function findTarget(
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
            "check needs an object outside a view opened for one of the access's types",
        );
    }

    return { type, id: target.id };
}
