import {
    Condition,
    Namespace,
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

/** The scope of a field relation with identifiers across scopes, followed only by `through`. */
export const ANY_SCOPE = "*";

/** Where a policy's objects live: their table and identity, scope, attribute, relation columns. */
export interface TableMapping {
    /** The policy of the objects. */
    readonly policy: Policy;
    /** The application table containing the protected objects. */
    readonly table: Table;
    /** The column identifying an object within its scope. */
    readonly id: string;
    /** The column selecting the object's scope, absent for objects living in the universe. */
    readonly scope?: string;
    /** The relation to each object's owner in another scope, whose bindings cover the object. */
    readonly owner?: string;
    /** Whether the table has several object types, told apart by its packageId and type columns. */
    readonly isShared?: boolean;
    /** The rows the replicas of the scopes inside each row's scope store, for inherited objects. */
    readonly inherited?: Condition;
    /** Application columns supplying declared scalar attributes. */
    readonly attributes: Readonly<Record<string, string>>;
    /** Relations with the subject in a field, the others stored as relationships. */
    readonly relations: Readonly<
        Record<
            string,
            {
                /** The column with the subject identifier. */
                readonly column: string;
                /** The scope of the column's subjects, the object's if absent, or `*`. */
                readonly scope?: string;
                /** The subject's type, scope and relation columns, for several subject types. */
                readonly subject?: Omit<ReferenceColumns, "id"> & {
                    readonly relation?: string;
                };
            }
        >
    >;
    /** The columns that refer to another object of any type, for `grants` expressions. */
    readonly references?: Readonly<Record<string, ReferenceColumns>>;
    /** Engine-maintained ancestor indexes for explicitly transitive relations. */
    readonly trees?: Readonly<Record<string, Tree>>;
    /** The relation to the owner parent, whose role bindings cover this object. */
    readonly parent?: string;
}

/** Where a policy's objects live: reading rows and scopes, and validating mappings. */
export const TableMapping = {
    /** Map a scope type onto the ancestry every database stores, its rows standing by reference. */
    ancestry(policy: Policy): TableMapping {
        return {
            policy,
            table: Scope.table,
            id: "scope",
            scope: "parent",
            isShared: true,
            attributes: {},
            relations: {},
        };
    },

    /** Decide whether the scopes below a row's scope receive it: a scope's, global or inherited. */
    isHandedDown(mapping: TableMapping, row: Row): boolean {
        // hand down scope objects and global rows, and inherited rows their condition matches
        if (mapping.policy.definition.scope === true || mapping.scope === undefined) {
            return true;
        } else if (mapping.inherited === undefined) {
            return false;
        }
        const predicate = Condition.resolve(mapping.inherited, Namespace.fields(mapping.table));

        return Predicate.matches(Predicate.compile(predicate, mapping.table), row);
    },

    /** Match a type's rows that the scopes below their scope receive, in a table or its alias. */
    handedDown(mapping: TableMapping, table: Table): SQL {
        // hand down scope objects and global rows, and inherited rows their condition matches
        if (mapping.policy.definition.scope === true || mapping.scope === undefined) {
            return sql`true`;
        } else if (mapping.inherited === undefined) {
            return sql`false`;
        }
        const predicate = Condition.resolve(mapping.inherited, Namespace.fields(mapping.table));

        return sql`coalesce(${Predicate.render(predicate, Predicate.bind(table))}, false)`;
    },

    /** Decide whether a mapping reads every attribute a policy condition reads. */
    decides(mapping: TableMapping, predicate: Predicate): boolean {
        return [...Predicate.fields(predicate)].every(
            (name) => mapping.attributes[name] !== undefined,
        );
    },

    /** Check mapped columns against their declared attribute and relation types. */
    validate(authorizer: Authorizer, mapping: TableMapping): void {
        // check the columns and the relations, trees and parent against the policy
        TableMapping.validateKeys(mapping);
        TableMapping.validateAttributes(mapping);
        TableMapping.validateReferenceColumns(mapping);
        TableMapping.validateRelations(authorizer, mapping);
        TableMapping.validateTrees(authorizer, mapping);
        TableMapping.validateParent(authorizer, mapping);
    },

    /** Require the record identifier and scope columns to be non-null text. */
    validateKeys(mapping: TableMapping): void {
        for (const name of mapping.scope === undefined
            ? [mapping.id]
            : [mapping.id, mapping.scope]) {
            const attribute = TableMapping.column(mapping.table, name).definition;
            if (attribute.kind !== "text" || attribute.nullable) {
                throw AccessError.of("DECLARATION_INVALID", `invalid object key column: ${name}`);
            }
        }
    },

    /** Require columns with the object's declared scalar type, where null reads as missing. */
    validateAttributes(mapping: TableMapping): void {
        const definition = mapping.policy.definition;
        for (const [name, expected] of Object.entries(definition.attributes)) {
            // leave the attributes of a scope type mapped onto its scope rows undecided
            const mapped = mapping.attributes[name];
            if (mapped === undefined && mapping.table === Scope.table) {
                continue;
            } else if (mapped === undefined) {
                throw AccessError.of(
                    "DECLARATION_INVALID",
                    `attribute ${name} of ${definition.name} has no column`,
                );
            }

            // compare the column's kind with the declared type
            const attribute = TableMapping.column(mapping.table, mapped).definition;
            const kind = attribute.kind;
            const type =
                kind === "integer" || kind === "real"
                    ? "number"
                    : kind === "text"
                      ? "string"
                      : kind;
            if (type !== expected) {
                throw AccessError.of(
                    "DECLARATION_INVALID",
                    `incompatible attribute column: ${name}`,
                );
            }
        }
    },

    /** Require text columns for polymorphic subjects and referenced objects. */
    validateReferenceColumns(mapping: TableMapping): void {
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

        // require each to be text
        for (const name of typed) {
            if (TableMapping.column(mapping.table, name).definition.kind !== "text") {
                throw AccessError.of("DECLARATION_INVALID", `invalid reference column: ${name}`);
            }
        }
    },

    /** Require a field relation to accept one type in a text column, unless typed columns exist. */
    validateRelations(authorizer: Authorizer, mapping: TableMapping): void {
        for (const [name, field] of Object.entries(mapping.relations)) {
            const relation = authorizer.relation(mapping.policy, name);
            if (
                (field.subject === undefined &&
                    (relation.subjects.length !== 1 || relation.open)) ||
                relation.subjects.some((subject) => subject.wildcard) ||
                TableMapping.column(mapping.table, field.column).definition.kind !== "text" ||
                field.scope === "" ||
                (field.scope === ANY_SCOPE &&
                    relation.subjects.some((subject) => TableMapping.isPrincipalType(subject)))
            ) {
                throw AccessError.of(
                    "DECLARATION_INVALID",
                    `invalid relation mapping: ${mapping.policy.name}.${name}`,
                );
            }
        }

        // require an owner mapped to a field relating one type
        const { owner } = mapping;
        if (owner !== undefined && mapping.relations[owner]?.subject !== undefined) {
            throw AccessError.of("DECLARATION_INVALID", `invalid owner mapping: ${owner}`);
        } else if (owner !== undefined && mapping.relations[owner] === undefined) {
            throw AccessError.of("DECLARATION_INVALID", `owner ${owner} is not a mapped relation`);
        }
    },

    /** Require each ancestor index to maintain this table's declared self relation. */
    validateTrees(authorizer: Authorizer, mapping: TableMapping): void {
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
                throw AccessError.of("DECLARATION_INVALID", `invalid tree mapping: ${name}`);
            }
        }
    },

    /** Require the owner parent to be a field relation of one subject type. */
    validateParent(authorizer: Authorizer, mapping: TableMapping): void {
        // refuse a parent stored as relationships
        if (mapping.parent !== undefined && !mapping.relations[mapping.parent]) {
            throw AccessError.of(
                "DECLARATION_INVALID",
                `owning parent must be a field relation: ${mapping.parent}`,
            );
        }
        // require one subject type of a parent stored in a field
        else if (mapping.parent !== undefined) {
            authorizer.onlySubject(mapping.policy, mapping.parent);
        }
    },

    /** Copy mapping selections while retaining the registered table and declaration. */
    freeze(mapping: TableMapping): TableMapping {
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
    },

    /** Decide whether a subject type is a principal, the type callers authenticate as. */
    isPrincipalType(type: SubjectType): boolean {
        return Object.values(principal).some(
            (kind) =>
                kind.definition.packageId === type.packageId && kind.definition.name === type.type,
        );
    },

    /** Read rows of a mapped type in a scope by identifier from a snapshot. */
    async read(
        snapshot: Snapshot,
        mapping: TableMapping,
        within: string,
        ids: readonly string[],
    ): Promise<Row[]> {
        // read the rows by identifier, and scopes through their reader of the universe
        const definition = mapping.policy.definition;
        const rows: Row[] =
            mapping.table === Scope.table
                ? await Scope.read(snapshot, ids)
                : await snapshot.select(
                      mapping.table,
                      [mapping.id],
                      ids.map((id) => [id]),
                  );

        return rows.filter(
            (row) =>
                (within === ANY_SCOPE || TableMapping.scope(mapping, row) === within) &&
                (mapping.isShared !== true ||
                    (row["packageId"] === definition.packageId && row["type"] === definition.name)),
        );
    },

    /** Read the scope a mapped row lives in. */
    scope(mapping: TableMapping, row: Row): string {
        return mapping.scope === undefined
            ? Scope.universe.id
            : TableMapping.text(row, mapping.scope);
    },

    /** Read a mapped row's text column, refusing a value of another kind. */
    text(row: Row, name: string): string {
        const value = row[name];
        if (typeof value !== "string") {
            throw new TypeError(`column ${name} is not text`);
        }

        return value;
    },

    /** Read the column storing a mapping's attribute, refusing an unmapped attribute. */
    attribute(mapping: TableMapping, name: string): string {
        const mapped = Object.hasOwn(mapping.attributes, name)
            ? mapping.attributes[name]
            : undefined;
        if (mapped === undefined) {
            throw AccessError.of("DECLARATION_INVALID", `attribute ${name} has no column`);
        }

        return mapped;
    },

    /** Read the ancestor index of a mapping's self relation, refusing a relation without one. */
    tree(mapping: TableMapping, relation: string): Tree {
        // require the declared index
        const trees = mapping.trees ?? {};
        const tree = Object.hasOwn(trees, relation) ? trees[relation] : undefined;
        if (tree === undefined) {
            throw AccessError.of("DECLARATION_INVALID", `${relation} has no ancestor index`);
        }

        return tree;
    },

    /** Read the field storing a mapping's relation, refusing a relation without one. */
    field(mapping: TableMapping, relation: string): TableMapping["relations"][string] {
        const field = Object.hasOwn(mapping.relations, relation)
            ? mapping.relations[relation]
            : undefined;
        if (field === undefined) {
            throw AccessError.of("DECLARATION_INVALID", `${relation} is not a field relation`);
        }

        return field;
    },

    /** Read the columns of a mapping's declared object reference. */
    reference(mapping: TableMapping, name: string): ReferenceColumns {
        // require the declared reference
        const references = mapping.references ?? {};
        const columns = Object.hasOwn(references, name) ? references[name] : undefined;
        if (columns === undefined) {
            throw AccessError.of("DECLARATION_INVALID", `unknown object reference: ${name}`);
        }

        return columns;
    },

    /** Select the scope a mapped row of a table, or of one of its aliases, lives in. */
    scopeColumn(table: Table, mapping: TableMapping): SQLWrapper {
        return mapping.scope === undefined
            ? sql`${Scope.universe.id}`
            : TableMapping.column(table, mapping.scope);
    },

    /** Resolve a mapped application column without interpolating caller SQL. */
    column(table: Table, name: string): Column {
        // accept only the table's columns
        const found = Object.hasOwn(table[TABLE].columns, name)
            ? table[TABLE].columns[name]
            : undefined;
        if (found === undefined) {
            throw AccessError.of("DECLARATION_INVALID", `unknown mapped column: ${name}`);
        }

        return found;
    },

    /** Select a table's columns under their property names. */
    columns(table: Table): SQL {
        return sql.join(
            Object.entries(table[TABLE].columns).map(
                ([property, entry]) => sql`${entry} AS ${sql.identifier(property)}`,
            ),
            sql`, `,
        );
    },
};

/** The columns that identify an object of any type: its package, type, scope and identifier. */
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

/** A relation stored in a field and used as a subject set. */
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
