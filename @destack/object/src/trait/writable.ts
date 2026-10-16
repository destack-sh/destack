import type { Table } from "@destack/db";
import type { Method } from "../method/method.ts";
import type { Empty } from "../method/procedure.ts";
import { method } from "../method/record.ts";
import { ObjectPermissions, ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** The permission whose holders create, update and delete the objects of a writable type. */
const WRITE = "write";

/** The verbs a type with a write permission derives, update and delete where other traits allow. */
type WriteVerb<
    Permissions extends string,
    IsUpdating extends boolean,
    IsDeleting extends boolean,
> = ["write"] extends [Permissions]
    ?
          | "create"
          | (IsUpdating extends true ? "update" : never)
          | (IsDeleting extends true ? "delete" : never)
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
          internal: false;
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
    IsUpdating extends boolean,
    IsDeleting extends boolean,
> = {
    readonly [
        Verb in WriteVerb<Permissions, IsUpdating, IsDeleting> as Verb extends keyof Methods
            ? never
            : Verb
    ]: WriteMethod<Verb, Defined, Written>;
};

/** The verbs a writable type derives under its write permission. */
interface Writes {
    /** Whether its objects change after creation, false for immutable versions. */
    readonly isUpdating: boolean;
    /** Whether it deletes its objects itself, false where another trait deletes them. */
    readonly isDeleting: boolean;
}

/** Objects created, updated and deleted by the holders of the type's write permission. */
export const writable: Trait<Writes> = {
    options: (definition) => {
        // write objects a store stores under a declared write permission, never events or another system's rows
        const storage = ObjectType.storageKind(definition.storage);
        const declared = Object.keys(ObjectPermissions.expressions(definition.permissions ?? []));
        if (storage === "events" || storage === "external" || !declared.includes(WRITE)) {
            return undefined;
        }

        return {
            isUpdating: definition.versioned === undefined,
            isDeleting: definition.recoverable === undefined,
        };
    },
    columns: () => ({}),
    constraints: () => [],
    methods: (options, declared) => ({
        ...(Object.hasOwn(declared, "create") ? {} : { create: method.create(WRITE) }),
        ...(!options.isUpdating || Object.hasOwn(declared, "update")
            ? {}
            : { update: method.update(WRITE) }),
        ...(!options.isDeleting || Object.hasOwn(declared, "delete")
            ? {}
            : { delete: method.delete(WRITE) }),
    }),
};
