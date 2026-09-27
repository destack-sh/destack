import {
    jsonElements,
    sql,
    Statement,
    TABLE,
    type SQL,
    type SQLWrapper,
    type Table,
} from "@destack/db";
import type { Tree } from "@destack/db/tree";
import type { SetRow } from "./access.ts";
import { AccessError } from "../error/index.ts";
import type { Policy } from "../policy/policy.ts";
import type { SubjectType } from "../policy/subject.ts";
import { GLOBAL_SCOPE } from "../context/context.ts";
import type { Authorizer } from "./authorizer.ts";

/** Where a policy's objects live: their table and the columns holding their identity, scope, attributes and relations. */
export interface TableMapping {
    /** The policy of the objects. */
    readonly policy: Policy;
    /** The application table containing the protected objects. */
    readonly table: Table;
    /** The column identifying an object within its scope. */
    readonly id: string;
    /** The column selecting the object's scope; objects without one live in the global scope. */
    readonly scope?: string;
    /** Whether the table holds several object types, told apart by its packageId and type columns. */
    readonly isShared?: boolean;
    /** Application columns supplying declared scalar attributes. */
    readonly attributes: Readonly<Record<string, string>>;
    /** Relations whose subject a field holds; relationships hold every other relation. */
    readonly relations: Readonly<
        Record<
            string,
            {
                /** The column holding the subject identifier. */
                readonly column: string;
                /** The scope of every subject the column holds; the object's own scope when absent. */
                readonly scope?: string;
                /** The columns naming the subject's type, scope and relation, for relations accepting several subject types; objects need no relation column. */
                readonly subject?: Omit<ReferenceColumns, "id"> & { readonly relation?: string };
                /** Whether the column holds whole subject keys of any type, as `subjectKey` writes them. */
                readonly isKey?: true;
            }
        >
    >;
    /** Columns naming another object a row refers to, whatever its type, which `grants` expressions follow. */
    readonly references?: Readonly<Record<string, ReferenceColumns>>;
    /** Engine-maintained ancestor indexes for explicitly transitive relations. */
    readonly trees?: Readonly<Record<string, Tree>>;
    /** The relation to the owning parent, whose role bindings cover this object. */
    readonly parent?: string;
}

/** Where a policy's objects live: reading their rows and scopes, and validating a mapping against its policy. */
export const TableMapping = {
    rows,
    scope,
    scopeColumn,
    validate,
    freeze,
};

/** The columns naming an object of any type: its package, type, scope and identifier. */
export interface ReferenceColumns {
    /** The column holding the object's package. */
    readonly packageId: string;
    /** The column holding the object's type. */
    readonly type: string;
    /** The column holding the object's scope. */
    readonly scope: string;
    /** The column holding the object's identifier. */
    readonly id: string;
}

/** A relation a field holds, which some relation names as a subject set. */
export interface FieldRelation {
    /** The mapping of the type declaring the relation. */
    readonly mapping: TableMapping;
    /** The relation's name. */
    readonly relation: string;
    /** The field holding its subject. */
    readonly field: TableMapping["relations"][string];
    /** The subject type the field holds. */
    readonly subject: SubjectType;
    /** Read the sets the field makes a JSON list of subjects members of. */
    readonly sets: Statement<Pick<SetRow, "packageId" | "type" | "scope" | "id">>;
}

/** Check mapped columns against their declared attribute and relation types. */
function validate(authorizer: Authorizer, mapping: TableMapping): void {
    // resolve the record identifier and scope before inspecting its attributes
    const definition = mapping.policy.definition;
    for (const name of mapping.scope === undefined ? [mapping.id] : [mapping.id, mapping.scope]) {
        const attribute = column(mapping.table, name).definition;
        if (attribute.kind !== "text" || attribute.nullable) {
            throw new AccessError("INVALID_DECLARATION", `invalid object key column: ${name}`);
        }
    }

    // require non-null columns with the scalar type declared by the object
    for (const [name, expected] of Object.entries(definition.attributes)) {
        const attribute = column(mapping.table, mapping.attributes[name]).definition;
        const kind = attribute.kind;
        const type =
            kind === "integer" || kind === "real" ? "number" : kind === "text" ? "string" : kind;
        if (attribute.nullable || type !== expected) {
            throw new AccessError("INVALID_DECLARATION", `incompatible attribute column: ${name}`);
        }
    }

    // require the text columns naming polymorphic subjects and referenced objects
    const typed = [
        ...Object.values(mapping.relations).flatMap((field) =>
            field.subject === undefined ? [] : Object.values(field.subject),
        ),
        ...Object.values(mapping.references ?? {}).flatMap((reference) => Object.values(reference)),
    ];
    for (const name of typed) {
        if (column(mapping.table, name).definition.kind !== "text") {
            throw new AccessError("INVALID_DECLARATION", `invalid reference column: ${name}`);
        }
    }

    // require a field-held relation to accept exactly one type in a text column, unless columns name its subject's type
    for (const [name, field] of Object.entries(mapping.relations)) {
        const relation = authorizer.relation(mapping.policy, name);
        if (
            (field.subject === undefined &&
                field.isKey === undefined &&
                (relation.subjects.length !== 1 || relation.open)) ||
            relation.subjects.some((subject) => subject.wildcard) ||
            column(mapping.table, field.column).definition.kind !== "text" ||
            field.scope === ""
        ) {
            throw new AccessError("INVALID_DECLARATION", `invalid relation mapping: ${name}`);
        }
    }

    // require each ancestor index to maintain this table's declared self relation
    for (const [name, tree] of Object.entries(mapping.trees ?? {})) {
        const field = mapping.relations[name];
        const [subject] = authorizer.relation(mapping.policy, name).subjects;
        if (
            !field ||
            field.scope !== undefined ||
            subject.packageId !== definition.packageId ||
            subject.type !== definition.name ||
            tree.definition.table !== mapping.table ||
            tree.definition.parent !== field.column ||
            tree.definition.id !== mapping.id ||
            tree.definition.scope !== mapping.scope
        ) {
            throw new AccessError("INVALID_DECLARATION", `invalid tree mapping: ${name}`);
        }
    }

    // require the owning parent to be a relation a field holds
    if (mapping.parent !== undefined && !mapping.relations[mapping.parent]) {
        throw new AccessError(
            "INVALID_DECLARATION",
            `owning parent must be a field-held relation: ${mapping.parent}`,
        );
    }
}

/** Copy mapping selections while retaining the registered table and declaration. */
function freeze(mapping: TableMapping): TableMapping {
    // copy nested relation fields so callers cannot change a prepared query
    const relations = Object.fromEntries(
        Object.entries(mapping.relations).map(([name, field]) => [
            name,
            Object.freeze({ ...field }),
        ]),
    );

    return Object.freeze({
        ...mapping,
        attributes: Object.freeze({ ...mapping.attributes }),
        relations: Object.freeze(relations),
        references:
            mapping.references === undefined ? undefined : Object.freeze({ ...mapping.references }),
        trees: mapping.trees === undefined ? undefined : Object.freeze({ ...mapping.trees }),
    });
}

/** Build the read of rows of a mapped type by a JSON list of identifiers in a scope, of the type where the table is shared. */
function rows(mapping: TableMapping): Statement {
    const table = mapping.table;
    const definition = mapping.policy.definition;

    return new Statement(
        (value) => sql`SELECT ${columnsOf(table)} FROM ${jsonElements(value("ids"), "listed_row")}
            JOIN ${from(table)} ON ${column(table, mapping.id)} = listed_row.value ->> 0
            WHERE ${mapping.scope === undefined ? sql`true` : sql`${column(table, mapping.scope)} = ${value("scope")}`}
                ${mapping.isShared ? sql`AND ${column(table, "packageId")} = ${definition.packageId} AND ${column(table, "type")} = ${definition.name}` : sql``}`,
    );
}

/** Read the scope a mapped row lives in. */
function scope(mapping: TableMapping, row: Readonly<Record<string, unknown>>): string {
    return mapping.scope === undefined ? GLOBAL_SCOPE : String(row[mapping.scope]);
}

/** Select the scope a mapped row of a table, or of one of its aliases, lives in. */
function scopeColumn(table: Table, mapping: TableMapping): SQLWrapper {
    return mapping.scope === undefined ? sql`${GLOBAL_SCOPE}` : column(table, mapping.scope);
}

/** Resolve a mapped application column without interpolating caller SQL. */
export function column(table: Table, name: string) {
    // accept only the table's own columns
    if (!Object.hasOwn(table[TABLE].columns, name)) {
        throw new AccessError("INVALID_DECLARATION", `unknown mapped column: ${name}`);
    }

    return table[TABLE].columns[name];
}

/** Select a table's columns under their property names. */
export function columnsOf(table: Table): SQL {
    return sql.join(
        Object.entries(table[TABLE].columns).map(
            ([property, entry]) => sql`${entry} AS ${sql.identifier(property)}`,
        ),
        sql`, `,
    );
}

/** Declare a source table and its alias inside a correlated SQL expression. */
export function from(table: Table): SQL {
    return table[TABLE].source ? sql`${table[TABLE].source} AS ${table}` : sql`${table}`;
}
