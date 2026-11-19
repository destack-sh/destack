import type { Table } from "@destack/db";
import type { Method } from "../method/method.ts";
import type { Empty } from "../method/procedure.ts";
import { method } from "../method/builder.ts";
import { ObjectPermissions, ObjectType } from "../object/object.ts";
import type { TraitBehavior } from "./trait.ts";

/** The permission whose holders create, update and delete the objects of a writable type. */
const WRITE = "write";

/** The verbs a type with a write permission derives, delete where other traits allow. */
type WriteVerb<Permissions extends string, IsDeleting extends boolean> = ["write"] extends [
    Permissions,
]
    ? "create" | "update" | (IsDeleting extends true ? "delete" : never)
    : never;

/** The method a type derives for a verb under its write permission. */
type WriteMethod<Verb, Defined extends Table, Written extends string> = Verb extends "create"
    ? Method<{
          kind: "create";
          permission: "write";
          input: never;
          prepared: never;
          mutates: true;
          fields: Written;
          filled: never;
          variants: never;
          table: Defined;
          visibility: "client";
          target: "scope";
      }>
    : Verb extends "update"
      ? Method<{
            kind: "update";
            target: "object";
            permission: "write";
            input: never;
            prepared: never;
            mutates: true;
            fields: Written;
            table: Defined;
        }>
      : Method<{
            kind: "delete";
            target: "object";
            permission: "write";
            output: typeof Empty;
            prepared: never;
            mutates: true;
            table: Defined;
        }>;

/** The create, update and delete methods a type derives from its write permission. */
export type WritableMethodMap<
    Methods,
    Permissions extends string,
    Defined extends Table,
    Written extends string,
    IsDeleting extends boolean,
> = {
    readonly [
        Verb in WriteVerb<Permissions, IsDeleting> as Verb extends keyof Methods ? never : Verb
    ]: WriteMethod<Verb, Defined, Written>;
};

/** The verbs a writable type derives under its write permission. */
interface Writes {
    /** Whether it deletes its objects itself, false where another trait deletes them. */
    readonly isDeleting: boolean;
}

/** Objects created, updated and deleted by the holders of the type's write permission. */
export const writable: TraitBehavior<Writes> = {
    options: (definition) => {
        // write objects a store stores under a declared write permission, never events or another system's rows
        const storage = ObjectType.storageKind(definition.storage);
        const declared = Object.keys(ObjectPermissions.expressions(definition.permissions ?? []));
        if (storage === "events" || storage === "external" || !declared.includes(WRITE)) {
            return undefined;
        }

        return { isDeleting: definition.recoverable === undefined };
    },
    columns: () => ({}),
    constraints: () => [],
    methods: (options, declared) => {
        // add each default the type declares no method of its kind or name for
        const kinds = new Set(
            Object.values(declared).flatMap((each) => (each === false ? [] : [each.kind])),
        );
        const isDeclared = (kind: "create" | "update" | "delete") =>
            kinds.has(kind) || Object.hasOwn(declared, kind);

        return {
            ...(isDeclared("create") ? {} : { create: method.create(WRITE) }),
            ...(isDeclared("update") ? {} : { update: method.update(WRITE) }),
            ...(!options.isDeleting || isDeclared("delete")
                ? {}
                : { delete: method.delete(WRITE) }),
        };
    },
};
