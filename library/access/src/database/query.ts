import { alias, sql, TABLE, type Table, type SQL, type SQLWrapper } from "@destack/db";
import { AccessError } from "../error/index.ts";
import { type ObjectType, type PermissionReference } from "../access/index.ts";
import { readOperand } from "../access/access.ts";
import { AccessModel } from "../access/model.ts";
import type { AccessOperand, AccessExpression } from "../access/expression.ts";
import type { AccessContext, Subject } from "../access/subject.ts";
import { accessGrant, accessToken } from "../stack/schema.ts";
import type { Tree } from "@destack/db/tree";
import { applies, permitsDelegation } from "../access/policy.ts";

/** SQL comparison operators for the declared expression language. */
const OPERATORS = { eq: "=", ne: "<>", lt: "<", lte: "<=", gt: ">", gte: ">=" };

/** Compile authorization into correlated predicates for existing application queries. */
export class AccessQuery {
    /** Application mappings indexed by declaring package and object type. */
    readonly #mappings = new Map<string, ObjectMapping>();

    /** Require complete mappings before accepting authorization queries. */
    constructor(
        readonly model: AccessModel,
        mappings: readonly ObjectMapping[],
    ) {
        // register each declaration once and resolve its application columns
        for (const source of mappings) {
            const mapping = freezeMapping(source);
            const definition = mapping.type.definition;
            if (
                model.type({ packageId: definition.packageId, type: definition.name }) !==
                mapping.type
            ) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    "mapping must reference the registered object declaration",
                );
            }

            // reject duplicate mappings before registering the table
            const key = JSON.stringify([definition.packageId, definition.name]);
            if (this.#mappings.has(key)) {
                throw new AccessError("INVALID_DECLARATION", `duplicate database mapping: ${key}`);
            }
            validateMapping(mapping);
            this.#mappings.set(key, mapping);
        }

        // require database mappings for every registered object type
        for (const definition of model.describe()) {
            this.mapping({ packageId: definition.packageId, type: definition.name });
        }
    }

    /** Restrict rows before sorting, counting, pagination, or mutation. */
    where(
        permission: PermissionReference,
        scope: string,
        context: AccessContext,
        source?: Table,
    ): SQL {
        // reject invalid request time before compiling expiry predicates
        if (!Number.isFinite(context.now)) {
            throw new AccessError("INVALID_CONTEXT", "request time must be finite");
        }

        // restrict evaluation to a valid delegation chain and its declared permission
        const mapping = this.mapping(permission);
        const table = source ?? mapping.table;
        if (!permitsDelegation(permission, { scope }, context)) {
            return sql`false`;
        }

        // allocate aliases within this predicate and evaluate the represented subject
        const aliases = { next: 0 };
        const expression = this.model.expression(permission);
        const predicates = [this.#compile(expression, mapping, table, context, aliases)];

        // intersect represented-user authority, actor authority and each delegation's selection
        for (const delegation of context.delegations ?? []) {
            predicates.push(
                this.#compile(
                    expression,
                    mapping,
                    table,
                    { ...context, subjects: [delegation.actor] },
                    aliases,
                ),
            );

            // constrain delegated permission to its selected objects
            const selections = delegation.permissions.filter(
                (entry) =>
                    entry.packageId === permission.packageId &&
                    entry.type === permission.type &&
                    entry.name === permission.name &&
                    entry.scope === scope,
            );
            if (!selections.some((entry) => entry.objectId === undefined)) {
                const ids = selections.map((entry) => sql`${entry.objectId}`);
                predicates.push(sql`${column(table, mapping.id)} IN (${sql.join(ids, sql`, `)})`);
            }
        }

        // apply host restrictions after resolving all participating subjects
        for (const policy of this.model.policies) {
            if (applies(policy, permission, scope)) {
                const condition = this.#compile(policy.condition, mapping, table, context, aliases);
                predicates.push(policy.effect === "restrict" ? condition : sql`NOT (${condition})`);
            }
        }

        return sql`(${column(table, mapping.scope)} = ${scope} AND (${sql.join(predicates, sql` AND `)}))`;
    }

    /** Return the application mapping for object checks and transactional mutations. */
    mapping(reference: Pick<PermissionReference, "packageId" | "type">): ObjectMapping {
        const mapping = this.#mappings.get(JSON.stringify([reference.packageId, reference.type]));
        if (!mapping) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `missing database mapping: ${reference.type}`,
            );
        }

        return mapping;
    }

    /** Translate one expression using bound values and generated SQL aliases. */
    #compile(
        expression: AccessExpression,
        mapping: ObjectMapping,
        source: Table,
        context: AccessContext,
        aliases: { next: number },
    ): SQL {
        switch (expression.kind) {
            case "union":
            case "intersection": {
                const children = expression.expressions.map((child) =>
                    this.#compile(child, mapping, source, context, aliases),
                );
                const separator = expression.kind === "union" ? sql` OR ` : sql` AND `;

                return sql`(${sql.join(children, separator)})`;
            }
            case "exclusion": {
                const include = this.#compile(
                    expression.include,
                    mapping,
                    source,
                    context,
                    aliases,
                );
                const exclude = this.#compile(
                    expression.exclude,
                    mapping,
                    source,
                    context,
                    aliases,
                );

                return sql`(${include} AND NOT ${exclude})`;
            }
            case "permission":
                return this.#compile(
                    this.model.expression(mapping.type.permission(expression.name)),
                    mapping,
                    source,
                    context,
                    aliases,
                );
            case "compare": {
                const left = operand(expression.left, mapping, source, context);
                const right = operand(expression.right, mapping, source, context);

                return sql`coalesce((${left} ${sql.raw(OPERATORS[expression.operator])} ${right}), false)`;
            }
            case "relation":
                return this.#relation(expression.name, mapping, source, context, aliases);
            case "through":
                return this.#through(expression, mapping, source, context, aliases);
        }
    }

    /** Match direct subjects or persisted grants for a declared relation. */
    #relation(
        name: string,
        mapping: ObjectMapping,
        source: Table,
        context: AccessContext,
        aliases: { next: number },
    ): SQL {
        // match application ownership against subjects of the declared authority
        const definition = mapping.type.definition.relations[name];
        if (definition.kind === "subject") {
            const subject = mapping.subjects[name];
            const ids = context.subjects
                .filter(
                    (candidate) =>
                        candidate.kind === subject.kind &&
                        candidate.authority === subject.authority,
                )
                .map((candidate) => sql`${candidate.id}`);

            if (ids.length === 0) {
                return sql`false`;
            }

            // require directly stored bearer subjects to retain a current credential too
            const match = sql`coalesce(${column(source, subject.column)} IN (${sql.join(ids, sql`, `)}), false)`;
            if (subject.kind === "share-token") {
                const scope = column(source, mapping.scope);
                const id = column(source, subject.column);

                return sql`(${match} AND ${scope} = ${subject.authority} AND ${activeToken(id, scope, context.now, aliases.next++)})`;
            }

            return match;
        }

        // match persisted grant subjects within their declared authorities
        const grant = alias(accessGrant, `access_grant_${aliases.next++}`);
        const subjects = context.subjects.filter(
            (subject) => definition.kind === "grant" && definition.subjects.includes(subject.kind),
        );
        const matches = subjects.map(
            (subject) =>
                sql`(${grant.subjectKind} = ${subject.kind} AND ${grant.subjectAuthority} = ${subject.authority} AND ${grant.subjectId} = ${subject.id})`,
        );

        // include public grants only when the relation permits them
        if (definition.kind === "grant" && definition.subjects.includes("everyone")) {
            matches.push(sql`${grant.subjectKind} = 'everyone'`);
        }
        if (matches.length === 0) {
            return sql`false`;
        }

        // require bearer grants to reference a current token in the same scope
        const active = activeToken(grant.subjectId, grant.scope, context.now, aliases.next++);

        // correlate an active grant to the protected row and requested relation
        return sql`EXISTS (
            SELECT 1 FROM ${from(grant)}
            WHERE ${grant.packageId} = ${mapping.type.definition.packageId}
                AND ${grant.type} = ${mapping.type.definition.name}
                AND ${grant.scope} = ${column(source, mapping.scope)}
                AND ${grant.objectId} = ${column(source, mapping.id)}
                AND ${grant.relation} = ${name}
                AND ${grant.createdAt} <= ${context.now}
                AND ${grant.revokedAt} IS NULL
                AND (${grant.expiresAt} IS NULL OR ${grant.expiresAt} > ${context.now})
                AND (${sql.join(matches, sql` OR `)})
                AND (${grant.subjectKind} <> 'share-token' OR (${grant.subjectAuthority} = ${grant.scope} AND ${active}))
        )`;
    }

    /** Correlate a parent permission through a foreign key or an ancestor index. */
    #through(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: ObjectMapping,
        source: Table,
        context: AccessContext,
        aliases: { next: number },
    ): SQL {
        // resolve the declared target type before compiling its permission
        const relation = mapping.type.definition.relations[expression.relation];
        if (relation.kind !== "object") {
            throw new AccessError(
                "INVALID_DECLARATION",
                "object traversal requires an object relation",
            );
        }
        const target = this.mapping({
            packageId: mapping.type.definition.packageId,
            type: relation.type,
        });

        // compile the target permission against a distinct table alias
        const parent = alias(target.table, `access_parent_${aliases.next++}`);
        const predicate = this.#compile(
            this.model.expression(target.type.permission(expression.permission)),
            target,
            parent,
            context,
            aliases,
        );

        // correlate direct parents within the source object's scope
        const scope = column(source, mapping.scope);
        const parentId = column(source, mapping.objects[expression.relation]);
        const targetId = column(parent, target.id);
        const targetScope = column(parent, target.scope);
        if (!expression.transitive) {
            return sql`EXISTS (SELECT 1 FROM ${from(parent)} WHERE ${targetScope} = ${scope} AND ${targetId} = ${parentId} AND ${predicate})`;
        }

        // require an explicitly configured ancestor index for transitive traversal
        const tree = mapping.trees?.[expression.relation];
        if (!tree) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `missing tree mapping: ${expression.relation}`,
            );
        }

        // join the indexed ancestors to their protected application rows
        const ancestor = alias(tree.ancestors, `access_ancestor_${aliases.next++}`);

        return sql`EXISTS (
            SELECT 1 FROM ${from(ancestor)} JOIN ${from(parent)}
                ON ${targetId} = ${ancestor.ancestor} AND ${targetScope} = ${ancestor.scope}
            WHERE ${ancestor.scope} = ${scope}
                AND ${ancestor.descendant} = ${column(source, mapping.id)}
                AND ${ancestor.depth} > 0
                AND ${predicate}
        )`;
    }
}

/** Map access declarations to authoritative columns in an application table. */
export interface ObjectMapping {
    /** The registered protected object type. */
    readonly type: ObjectType;
    /** The application table containing the protected objects. */
    readonly table: Table;
    /** The column identifying an object within its scope. */
    readonly id: string;
    /** The column selecting the object's authority scope. */
    readonly scope: string;
    /** Application columns supplying declared scalar attributes. */
    readonly attributes: Readonly<Record<string, string>>;
    /** Direct ownership stored in application columns. */
    readonly subjects: Readonly<
        Record<
            string,
            {
                /** The column containing the subject identifier. */
                readonly column: string;
                /** The kind of subject stored in this column. */
                readonly kind: Subject["kind"];
                /** The authority issuing these subject identifiers. */
                readonly authority: string;
            }
        >
    >;
    /** Nullable foreign-key columns implementing object relationships. */
    readonly objects: Readonly<Record<string, string>>;
    /** Engine-maintained ancestor indexes for explicitly transitive relationships. */
    readonly trees?: Readonly<Record<string, Tree>>;
}

/** Check mapped columns against their declared attribute and relation types. */
function validateMapping(mapping: ObjectMapping): void {
    // resolve the record identifier and scope before inspecting its attributes
    const definition = mapping.type.definition;
    for (const name of [mapping.id, mapping.scope]) {
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

    // resolve ownership and object references stored in application columns
    for (const [name, relation] of Object.entries(definition.relations)) {
        if (relation.kind === "subject") {
            const subject = mapping.subjects[name];
            if (!subject || !relation.subjects.includes(subject.kind)) {
                throw new AccessError("INVALID_DECLARATION", `missing subject mapping: ${name}`);
            }
            if (
                column(mapping.table, subject.column).definition.kind !== "text" ||
                !subject.authority
            ) {
                throw new AccessError("INVALID_DECLARATION", `invalid subject mapping: ${name}`);
            }
        } else if (relation.kind === "object") {
            if (column(mapping.table, mapping.objects[name]).definition.kind !== "text") {
                throw new AccessError("INVALID_DECLARATION", `invalid object mapping: ${name}`);
            }
        }
    }

    // require each ancestor index to maintain this table's declared relationship
    for (const [name, tree] of Object.entries(mapping.trees ?? {})) {
        const relation = definition.relations[name];
        if (
            !Object.hasOwn(definition.relations, name) ||
            relation.kind !== "object" ||
            relation.type !== definition.name ||
            tree.definition.table !== mapping.table ||
            tree.definition.parent !== mapping.objects[name] ||
            tree.definition.id !== mapping.id ||
            tree.definition.scope !== mapping.scope
        ) {
            throw new AccessError("INVALID_DECLARATION", `invalid tree mapping: ${name}`);
        }
    }
}

/** Copy mapping selections while retaining the registered table and declaration. */
function freezeMapping(mapping: ObjectMapping): ObjectMapping {
    // copy nested subject selections so callers cannot change a prepared query
    const subjects = Object.fromEntries(
        Object.entries(mapping.subjects).map(([name, subject]) => [
            name,
            Object.freeze({ ...subject }),
        ]),
    );

    return Object.freeze({
        ...mapping,
        attributes: Object.freeze({ ...mapping.attributes }),
        subjects: Object.freeze(subjects),
        objects: Object.freeze({ ...mapping.objects }),
        trees: mapping.trees === undefined ? undefined : Object.freeze({ ...mapping.trees }),
    });
}

/** Declare a source table and its alias inside a correlated SQL expression. */
function from(table: Table): SQL {
    return table[TABLE].source ? sql`${table[TABLE].source} AS ${table}` : sql`${table}`;
}

/** Require a bearer credential to remain active in the protected object's scope. */
function activeToken(id: SQLWrapper, scope: SQLWrapper, now: number, index: number): SQL {
    const token = alias(accessToken, `access_token_${index}`);

    return sql`EXISTS (
        SELECT 1 FROM ${from(token)}
        WHERE ${token.id} = ${id} AND ${token.scope} = ${scope}
            AND ${token.revokedAt} IS NULL AND ${token.createdAt} <= ${now}
            AND (${token.expiresAt} IS NULL OR ${token.expiresAt} > ${now})
    )`;
}

/** Resolve a mapped application column without interpolating caller SQL. */
function column(table: Table, name: string) {
    if (!Object.hasOwn(table[TABLE].columns, name)) {
        throw new AccessError("INVALID_DECLARATION", `unknown mapped column: ${name}`);
    }

    return table[TABLE].columns[name];
}

/** Bind trusted request values or reference declared application columns. */
function operand(
    value: AccessOperand,
    mapping: ObjectMapping,
    source: Table,
    context: AccessContext,
): SQLWrapper {
    if (value.kind === "object") {
        return column(source, mapping.attributes[value.name]);
    }

    return sql`${readOperand(value, {}, context)}`;
}
