import { present, type schema } from "@destack/schema";
import { managerChecks } from "@destack/access";
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
import type { Gated, Trait } from "./trait.ts";

import { detach, apply } from "../method/declarable.ts";
/** Decide whether a declaration still manages a record. */
export function isManaging(row: Row | undefined): boolean {
    return (
        row?.["managerInstallationId"] !== undefined &&
        row["managerInstallationId"] !== null &&
        row["detachedAt"] === null
    );
}

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

/** The methods declared records take: applying them where their rows live. */
export type DeclarableMethodMap<Declarable> = [Declarable] extends [undefined]
    ? {}
    : { readonly apply: typeof apply };

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
    methods: () => ({ apply }),
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

/** The procedure applying a declared record derives, which only the system calls. */
export type DeclarableProcedures<Object extends ObjectType> = {
    apply: Procedure<schema.Object<Readonly<Record<string, schema.Schema>>>, RowSchema<Object>>;
};

/** The procedure detaching derives. */
export type DetachableProcedures<Object extends ObjectType> = {
    detach: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & RevisionField>,
        RowSchema<Object>
    >;
};
