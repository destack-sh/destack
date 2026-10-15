import { present, type schema } from "@destack/schema";
import { Manager, managerChecks } from "@destack/access";
import { identifier, integer, text, type Column, type Row } from "@destack/db";
import { type Method } from "../method/method.ts";
import {
    type Procedure,
    type ReplayShape,
    type RevisionField,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import { type Gated, derivePermissions, type Trait } from "./trait.ts";

import { adopt, apply, detach } from "../method/declarable.ts";

/** Declare the columns identifying a record's managing declaration. */
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

/** The columns identifying a record's manager. */
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

/** The options of declarable records: their own declaration schema, or their create input. */
export type DeclarableDefinition<Declared = unknown> =
    | {
          /** The schema of one declaration in a stack. */
          readonly schema: schema.Schema<Declared>;
      }
    | {
          /** Declare the records as their create method takes them. */
          readonly from: "create";
      };

/** The methods declared records take: applying them where their rows live. */
export type DeclarableMethodMap<Declarable> = [Declarable] extends [undefined]
    ? {}
    : { readonly apply: typeof apply };

/** The methods adoptable records take: detach and adopt. */
export type AdoptableMethodMap<Gate> = [Gate] extends [string]
    ? {
          readonly detach: Method<{ kind: "detach"; permission: Gate; mutates: true }>;
          readonly adopt: Method<{ kind: "adopt"; permission: Gate; mutates: true }>;
      }
    : {};

/** Records that stacks declare, managed by their declaration until detached. */
export const declarable: Trait<DeclarableDefinition> & {
    /** Decide whether a declaration still manages a record. */
    readonly isManaging: (row: Row | undefined) => boolean;
} = {
    key: "declarable",
    storages: ["durable"],
    options: (definition) => definition.declarable,
    columns: () => managedColumns(),
    constraints: (_options, table, columns) =>
        managerChecks(table, {
            managerInstallationId: present(columns["managerInstallationId"], "the manager column"),
            managerPackageId: present(columns["managerPackageId"], "the manager package column"),
            managerName: present(columns["managerName"], "the manager name column"),
            detachedAt: present(columns["detachedAt"], "the detachment column"),
        }),
    policy: (_options, _object, permissions) => derivePermissions(permissions, ["declare"]),
    methods: () => ({ apply }),
    isManaging(row: Row | undefined): boolean {
        return (
            row?.["managerInstallationId"] !== undefined &&
            row["managerInstallationId"] !== null &&
            row["detachedAt"] === null
        );
    },
};

/** Declared records detached for direct editing, and direct records a declaration adopts. */
export const adoptable: Trait<Gated> & {
    /** Declare the method detaching a record from its declaration. */
    detach: typeof detach;
    /** Declare the method adopting a direct record into a declaration. */
    adopt: typeof adopt;
} = {
    key: "adoptable",
    storages: ["durable"],
    options: (definition) => definition.adoptable,
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => ({ detach: detach(options.by), adopt: adopt(options.by) }),
    validate: (_options, object, definition) => {
        // require records a stack declares
        if (definition.declarable === undefined) {
            throw new TypeError(`object ${object.name} is adoptable but not declarable`);
        }
    },
    detach: (permission) => detach(permission),
    adopt: (permission) => adopt(permission),
};

/** The procedure applying a declared record derives, which stays out of the routes people call. */
export type DeclarableProcedures<Object extends ObjectType> = {
    apply: Procedure<schema.Object<Readonly<Record<string, schema.Schema>>>, RowSchema<Object>>;
};

/** The procedures the adoptable trait derives. */
export type AdoptableProcedures<Object extends ObjectType> = {
    detach: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & RevisionField>,
        RowSchema<Object>
    >;
    adopt: Procedure<
        schema.Object<
            TargetShape<Object> &
                ReplayShape<Object> &
                RevisionField & { readonly manager: typeof Manager.schema }
        >,
        RowSchema<Object>
    >;
};
