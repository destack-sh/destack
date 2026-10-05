import {
    type Condition,
    Predicate,
    sql,
    TABLE,
    type Column,
    type SQL,
    type SQLWrapper,
    type Snapshot,
    type Table,
    type Tree,
    type Row,
} from "@destack/db";
import { Scope } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import type { Policy } from "../declare/policy.ts";
import { principal } from "../declare/principal.ts";
import type { SubjectType } from "../declare/subject.ts";
import type { Authorizer } from "./authorizer.ts";

/** The scope of a field relation naming objects by identifier across scopes, which only `through` follows. */
export const ANY_SCOPE = "*";

/** Where a policy's objects live: their table and the columns with their identity, scope, attributes and relations. */
export interface TableMapping {
    /** The policy of the objects. */
    readonly policy: Policy;
    /** The application table containing the protected objects. */
    readonly table: Table;
    /** The column identifying an object within its scope. */
    readonly id: string;
    /** The column selecting the object's scope, absent for objects living in the universe. */
    readonly scope?: string;
    /** Whether the table has several object types, told apart by its packageId and type columns. */
    readonly isShared?: boolean;
    /** The rows the scopes inside each row's scope copy, for inherited objects. */
    readonly inherited?: Condition;
    /** Application columns supplying declared scalar attributes. */
    readonly attributes: Readonly<Record<string, string>>;
    /** Relations with the subject in a field, the others kept in relationships. */
    readonly relations: Readonly<
        Record<
            string,
            {
                /** The column with the subject identifier. */
                readonly column: string;
                /** The scope of every subject in the column, the object's own scope when absent, and `*` for objects identified across scopes. */
                readonly scope?: string;
                /** The columns with the subject's type, scope and relation, for relations with several subject types. */
                readonly subject?: Omit<ReferenceColumns, "id"> & {
                    readonly relation?: string;
                };
                /** Whether the column has whole subject keys of any type, as `Subject.key` writes them. */
                readonly isKey?: true;
            }
        >
    >;
    /** The columns that refer to another object of any type, for `grants` expressions. */
    readonly references?: Readonly<Record<string, ReferenceColumns>>;
    /** Engine-maintained ancestor indexes for explicitly transitive relations. */
    readonly trees?: Readonly<Record<string, Tree>>;
    /** The relation to the owning parent, whose role bindings cover this object. */
    readonly parent?: string;
}

/** Where a policy's objects live: reading their rows and scopes, and validating a mapping against its policy. */
export const TableMapping = {
    read,
    scope,
    text,
    attribute: readAttribute,
    field: readField,
    reference: readReference,
    tree: readTree,
    scopeColumn,
    validate,
    freeze,
    decides,
};

/** Decide whether a mapping reads every attribute a policy condition reads, which a scope type mapped onto its scope rows does not. */
function decides(mapping: TableMapping, predicate: Predicate): boolean {
    return [...Predicate.fields(predicate)].every((name) => mapping.attributes[name] !== undefined);
}

/** The columns naming an object of any type: its package, type, scope and identifier. */
export interface ReferenceColumns {
    /** The column with the object's package. */
    readonly packageId: string;
    /** The column with the object's type. */
    readonly type: string;
    /** The column with the object's scope. */
    readonly scope: string;
    /** The column with the object's identifier. */
    readonly id: string;
}

/** A relation kept in a field and used as a subject set. */
export interface FieldRelation {
    /** The mapping of the type declaring the relation. */
    readonly mapping: TableMapping;
    /** The relation's name. */
    readonly relation: string;
    /** The field with its subject. */
    readonly field: TableMapping["relations"][string];
    /** The subject type of the field. */
    readonly subject: SubjectType;
}

/** Check mapped columns against their declared attribute and relation types. */
function validate(authorizer: Authorizer, mapping: TableMapping): void {
    // check the columns and the relations, trees and parent against the policy
    validateKeys(mapping);
    validateAttributes(mapping);
    validateReferenceColumns(mapping);
    validateRelations(authorizer, mapping);
    validateTrees(authorizer, mapping);
    validateParent(authorizer, mapping);
}

/** Require the record identifier and scope columns to hold text that is never null. */
function validateKeys(mapping: TableMapping): void {
    for (const name of mapping.scope === undefined ? [mapping.id] : [mapping.id, mapping.scope]) {
        const attribute = column(mapping.table, name).definition;
        if (attribute.kind !== "text" || attribute.nullable) {
            throw new AccessError("INVALID_DECLARATION", `invalid object key column: ${name}`);
        }
    }
}

/** Require columns with the object's declared scalar type, where null reads as missing. */
function validateAttributes(mapping: TableMapping): void {
    const definition = mapping.policy.definition;
    for (const [name, expected] of Object.entries(definition.attributes)) {
        // leave the attributes of a scope type mapped onto its scope rows undecided
        const mapped = mapping.attributes[name];
        if (mapped === undefined && mapping.table === Scope.table) {
            continue;
        } else if (mapped === undefined) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `no column maps attribute ${name} of ${definition.name}`,
            );
        }

        // compare the column's kind with the declared type
        const attribute = column(mapping.table, mapped).definition;
        const kind = attribute.kind;
        const type =
            kind === "integer" || kind === "real" ? "number" : kind === "text" ? "string" : kind;
        if (type !== expected) {
            throw new AccessError("INVALID_DECLARATION", `incompatible attribute column: ${name}`);
        }
    }
}

/** Require the text columns naming polymorphic subjects and referenced objects. */
function validateReferenceColumns(mapping: TableMapping): void {
    // collect the subject type columns of fields and the columns of references
    const typed = [
        ...Object.values(mapping.relations).flatMap((field) =>
            field.subject === undefined ? [] : Object.values<string>(field.subject),
        ),
        ...Object.values(mapping.references ?? {}).flatMap((reference) => [
            reference.packageId,
            reference.type,
            reference.scope,
            reference.id,
        ]),
    ];

    // require each to hold text
    for (const name of typed) {
        if (column(mapping.table, name).definition.kind !== "text") {
            throw new AccessError("INVALID_DECLARATION", `invalid reference column: ${name}`);
        }
    }
}

/** Require a field relation to accept one type in a text column, unless columns keep its subject's type. */
function validateRelations(authorizer: Authorizer, mapping: TableMapping): void {
    for (const [name, field] of Object.entries(mapping.relations)) {
        const relation = authorizer.relation(mapping.policy, name);
        if (
            (field.subject === undefined &&
                field.isKey === undefined &&
                (relation.subjects.length !== 1 || relation.open)) ||
            relation.subjects.some((subject) => subject.wildcard) ||
            column(mapping.table, field.column).definition.kind !== "text" ||
            field.scope === "" ||
            (field.scope === ANY_SCOPE && relation.subjects.some(isPrincipalType))
        ) {
            throw new AccessError("INVALID_DECLARATION", `invalid relation mapping: ${name}`);
        }
    }
}

/** Require each ancestor index to maintain this table's declared self relation. */
function validateTrees(authorizer: Authorizer, mapping: TableMapping): void {
    const definition = mapping.policy.definition;
    for (const [name, tree] of Object.entries(mapping.trees ?? {})) {
        const field = mapping.relations[name];
        const [subject] = authorizer.relation(mapping.policy, name).subjects;
        if (
            !field ||
            subject === undefined ||
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
}

/** Require the owning parent to be a field relation of one subject type. */
function validateParent(authorizer: Authorizer, mapping: TableMapping): void {
    // refuse a parent kept in relationships
    if (mapping.parent !== undefined && !mapping.relations[mapping.parent]) {
        throw new AccessError(
            "INVALID_DECLARATION",
            `owning parent must be a field relation: ${mapping.parent}`,
        );
    }
    // require one subject type of a parent kept in a field
    else if (mapping.parent !== undefined) {
        authorizer.onlySubject(mapping.policy, mapping.parent);
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
        ...(mapping.references === undefined
            ? {}
            : { references: Object.freeze({ ...mapping.references }) }),
        ...(mapping.trees === undefined ? {} : { trees: Object.freeze({ ...mapping.trees }) }),
    });
}

/** Decide whether a subject type is a principal, which callers authenticate as. */
function isPrincipalType(type: SubjectType): boolean {
    return Object.values(principal).some(
        (kind) =>
            kind.definition.packageId === type.packageId && kind.definition.name === type.type,
    );
}

/** Read rows of a mapped type in a scope by identifier from a snapshot. */
async function read(
    snapshot: Snapshot,
    mapping: TableMapping,
    within: string,
    ids: readonly string[],
): Promise<Row[]> {
    const definition = mapping.policy.definition;
    const rows = await snapshot.select(
        mapping.table,
        [mapping.id],
        ids.map((id) => [id]),
    );

    return rows.filter(
        (row) =>
            (within === ANY_SCOPE || scope(mapping, row) === within) &&
            (mapping.isShared !== true ||
                (row["packageId"] === definition.packageId && row["type"] === definition.name)),
    );
}

/** Read the scope a mapped row lives in. */
function scope(mapping: TableMapping, row: Row): string {
    return mapping.scope === undefined ? Scope.universe.id : text(row, mapping.scope);
}

/** Read a mapped row's text column, refusing a value of another kind. */
function text(row: Row, name: string): string {
    const value = row[name];
    if (typeof value !== "string") {
        throw new TypeError(`column ${name} keeps no text`);
    }

    return value;
}

/** Read the column keeping a mapping's attribute, refusing an unmapped attribute. */
function readAttribute(mapping: TableMapping, name: string): string {
    const mapped = Object.hasOwn(mapping.attributes, name) ? mapping.attributes[name] : undefined;
    if (mapped === undefined) {
        throw new AccessError("INVALID_DECLARATION", `no column maps attribute ${name}`);
    }

    return mapped;
}

/** Read the ancestor index maintaining a mapping's self relation, refusing a relation without one. */
function readTree(mapping: TableMapping, relation: string): Tree {
    // require the declared index
    const trees = mapping.trees ?? {};
    const tree = Object.hasOwn(trees, relation) ? trees[relation] : undefined;
    if (tree === undefined) {
        throw new AccessError("INVALID_DECLARATION", `no ancestor index maintains ${relation}`);
    }

    return tree;
}

/** Read the field keeping a mapping's relation, refusing a relation without one. */
function readField(mapping: TableMapping, relation: string): TableMapping["relations"][string] {
    const field = Object.hasOwn(mapping.relations, relation)
        ? mapping.relations[relation]
        : undefined;
    if (field === undefined) {
        throw new AccessError("INVALID_DECLARATION", `${relation} is no field relation`);
    }

    return field;
}

/** Read the columns of a mapping's declared object reference. */
function readReference(mapping: TableMapping, name: string): ReferenceColumns {
    // require the declared reference
    const references = mapping.references ?? {};
    const columns = Object.hasOwn(references, name) ? references[name] : undefined;
    if (columns === undefined) {
        throw new AccessError("INVALID_DECLARATION", `unknown object reference: ${name}`);
    }

    return columns;
}

/** Select the scope a mapped row of a table, or of one of its aliases, lives in. */
function scopeColumn(table: Table, mapping: TableMapping): SQLWrapper {
    return mapping.scope === undefined ? sql`${Scope.universe.id}` : column(table, mapping.scope);
}

/** Resolve a mapped application column without interpolating caller SQL. */
export function column(table: Table, name: string): Column {
    // accept only the table's own columns
    const found = Object.hasOwn(table[TABLE].columns, name)
        ? table[TABLE].columns[name]
        : undefined;
    if (found === undefined) {
        throw new AccessError("INVALID_DECLARATION", `unknown mapped column: ${name}`);
    }

    return found;
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
