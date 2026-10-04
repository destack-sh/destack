import { defineSchema, present, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { managerChecks } from "@destack/access";
import { identifier, integer, text, type Column, type Row } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { defineMethod, type Method } from "../method/method.ts";
import type { Call } from "../method/call.ts";
import type { ObjectTable } from "../object/table.ts";
import {
    type Procedure,
    RevisionShape,
    type ReplayShape,
    type RevisionField,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { Gated, Trait } from "./trait.ts";

/** The schema of the stack declaration managing a record. */
const ManagerSchema = defineSchema(
    schema.object({
        /** The installation that applied the declaration. */
        installationId: schema.identifier("installation"),
        /** The immutable identity of the declaring package. */
        packageId: PackageId,
        /** The declaration's path within the package, such as installations/notes. */
        name: schema.string().regex(/^[a-z][a-z0-9-]*(?:\/[a-z][a-z0-9-]*)*$/u),
    }),
);
/** The stack declaration managing a record. */
export type Manager = schema.Infer<typeof ManagerSchema>;

/** The stack declaration managing a record. */
export const Manager = {
    /** The schema of a manager. */
    schema: ManagerSchema,

    /** Read a record's manager, or null for records managed at runtime. */
    read(row: {
        /** The installation that applied the declaration. */
        readonly managerInstallationId: string | null;
        /** The declaring package. */
        readonly managerPackageId: string | null;
        /** The declaration's name. */
        readonly managerName: string | null;
    }): Manager | null {
        return row.managerInstallationId === null
            ? null
            : ManagerSchema.parse({
                  installationId: row.managerInstallationId,
                  packageId: row.managerPackageId,
                  name: row.managerName,
              });
    },

    /** Decide whether a declaration still manages a record. */
    isManaging(row: Row | undefined): boolean {
        return (
            row?.["managerInstallationId"] !== undefined &&
            row["managerInstallationId"] !== null &&
            row["detachedAt"] === null
        );
    },

    /** Write a record's manager into its columns. */
    values(manager: Manager | null) {
        return {
            managerInstallationId: manager?.installationId ?? null,
            managerPackageId: manager?.packageId ?? null,
            managerName: manager?.name ?? null,
        };
    },
};

/** Declare the columns naming a record's managing declaration. */
export function managedColumns() {
    return {
        /** The installation that applied the declaration. */
        managerInstallationId: identifier("manager_installation_id", "installation"),
        /** The declaring package. */
        managerPackageId: identifier("manager_package_id", "package"),
        /** The declaration's name within the declaring package. */
        managerName: text("manager_name"),
        /** The time the declaration stopped managing the record, null while managed. */
        detachedAt: integer("detached_at"),
    };
}

/** The table of declarable objects: the record columns and the manager's. */
export type DeclarableTable = ObjectTable<
    string,
    unknown,
    {},
    { readonly declarable: DeclarableDefinition }
>;

/** The columns naming a record's manager. */
export interface ManagedColumnMap {
    /** The installation that applied the declaration. */
    readonly managerInstallationId: Column;
    /** The declaring package. */
    readonly managerPackageId: Column;
    /** The declaration's name. */
    readonly managerName: Column;
    /** The time the declaration stopped managing the record. */
    readonly detachedAt: Column;
}

/** The options of declarable records. */
export interface DeclarableDefinition<Declared = unknown> {
    /** The schema of one declaration in a stack. */
    readonly schema: schema.Schema<Declared>;
}

/** The methods detachable records take. */
export type DetachableMethodMap<Detach> = [Detach] extends [string]
    ? { readonly detach: Method<{ kind: "detach"; permission: Detach; mutates: true }> }
    : {};

/** Records that stacks declare, managed by their declaration until detached. */
export const declarable: Trait<DeclarableDefinition> = {
    key: "declarable",
    isDurable: true,
    options: (definition) => definition.declarable,
    columns: () => managedColumns(),
    constraints: (_options, table, columns) =>
        managerChecks(table, {
            managerInstallationId: present(columns["managerInstallationId"], "the manager column"),
            managerPackageId: present(columns["managerPackageId"], "the manager package column"),
            managerName: present(columns["managerName"], "the manager name column"),
            detachedAt: present(columns["detachedAt"], "the detachment column"),
        }),
    methods: () => ({}),
};

/** Declared records callers detach from their declaration. */
export const detachable: Trait<Gated> & {
    /** Declare the method detaching a record from its declaration. */
    detach: typeof detach;
} = {
    key: "detachable",
    isDurable: true,
    options: (definition) => definition.detachable,
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => ({ detach: detach(options.by) }),
    validate: (_options, object, definition) => {
        // require records a stack declares
        if (definition.declarable === undefined) {
            throw new TypeError(`object ${object.name} is detachable but not declarable`);
        }
    },
    detach: (permission) => detach(permission),
};

/** Detach an object from its stack declaration. */
export function detach<const Permission extends string>(
    permission: Permission,
): Method<{ kind: "detach"; permission: Permission; mutates: true }> {
    return defineMethod<{ kind: "detach"; permission: Permission; mutates: true }>({
        kind: "detach",
        permission,
        mutates: true,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/detach" },
            input: shapes.target.extend({ ...shapes.replay, ...RevisionShape }),
            output: shapes.row,
        }),
        handler: (call: Call<DeclarableTable>) => call.update({ detachedAt: call.now }),
        async execute(call) {
            // require an object its declaration still manages
            if (!Manager.isManaging(call.target)) {
                throw new ServiceError("CONFLICT", {
                    message: `${call.object.name} is not managed by a declaration`,
                });
            }

            return this.handler(call);
        },
    });
}

/** The procedure detaching derives. */
export type DetachableProcedures<Object extends ObjectType> = {
    detach: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & RevisionField>,
        RowSchema<Object>
    >;
};
