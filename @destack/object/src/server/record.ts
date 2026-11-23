import { isPrincipal, PermissionReference, type Creation, Manager } from "@destack/access";
import type { Access, Reader } from "@destack/access/authorizer";
import {
    Condition,
    DatabaseError,
    type InsertValue,
    type OpenCondition,
    Row,
    type Select,
    Table,
    TABLE,
} from "@destack/db";
import { canonicalize, type JsonObject, nth, present, schema, zip } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { conceal, CallError } from "../error/call.ts";
import { ObjectError } from "../error/error.ts";
import { MethodContext, RecordedRow } from "../method/context.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";
import { Method, type MethodConfiguration } from "../method/method.ts";
import { EventLog } from "../query/event.ts";
import { Numbered } from "../trait/numbered.ts";
import { Orderable } from "../trait/orderable.ts";
import { list } from "../query/list.ts";
import { ObjectRecord, BatchItem, MATCH, UPDATED_ITEM } from "../method/record.ts";

/** The record verbs a store runs for the built-in method kinds: create, update, delete and list. */
export const RecordExecution = {
    /** Require the receiving parent and the create permission, over grants read once. */
    async authorizeCreate(
        context: MethodContext,
        execution: StoreExecution,
        grants?: Reader,
    ): Promise<void> {
        // require the receiving parent and the creation
        const reader = grants ?? (await execution.authorization.reader(context.scope));
        const parent = context.object.parent && context.parent();
        if (parent) {
            await RecordExecution.requireReceiving(context, execution, parent, reader);
        }
        await RecordExecution.requireCreatable(context, execution, reader);
    },

    /** Require the add permission on a created object's parent. */
    async requireReceiving(
        context: MethodContext,
        execution: StoreExecution,
        parent: ObjectReference,
        reader?: Reader,
    ): Promise<void> {
        // require a nested object, and an attachment host type
        const declared = context.object.parent;
        if (declared === undefined) {
            throw new TypeError(`object ${context.object.name} is not nested in any parent`);
        }
        const { authorization } = execution;
        const authorizer = authorization.authorizer;
        if (declared.object === "any") {
            const hosts = authorizer.relation(context.object.policy, "parent").subjects;
            if (
                !hosts.some(
                    (host) => host.packageId === parent.packageId && host.type === parent.type,
                )
            ) {
                throw ObjectError.of(
                    "PARENT_TYPE_INVALID",
                    `parent ${parent.type} is invalid: it cannot have '${context.object.plural}'`,
                );
            }

            // require the parent to exist
            const mapping = authorizer.requireMapping(parent);
            const [found] = await authorization.transaction
                .view()
                .read(mapping.table)
                .findMany({
                    where: {
                        [mapping.id]: parent.id,
                        ...(mapping.scope === undefined ? {} : { [mapping.scope]: parent.scope }),
                    },
                });
            if (found === undefined) {
                throw CallError.http("NOT_FOUND", {
                    message: `${parent.type} '${parent.id}' not found`,
                });
            }
        }

        // hide an unreadable parent unless the caller may add under it, else require the permission
        const add = authorizer.policy(parent).permission(declared.add);
        if (!(await authorization.check(add, parent, reader)).isAllowed) {
            const host =
                declared.object === "any"
                    ? context.objects.find((object) => object.policy.is(parent))
                    : declared.object;
            const reading = host?.reading;
            if (reading === undefined || !(await authorization.check(reading, parent)).isAllowed) {
                throw CallError.http("NOT_FOUND", {
                    message: `${parent.type} '${parent.id}' not found`,
                });
            }
            await authorization.require(add, parent);
        }

        // require the permitted parent ready for children
        await RecordExecution.requireParentReady(context, parent.id);
    },

    /** Require each item's receiving parent and create permission, reading the grants once. */
    async authorizeItems(context: MethodContext, execution: StoreExecution): Promise<void> {
        const reader = await execution.authorization.reader(context.scope);
        await RecordExecution.eachItem(context, (item) =>
            RecordExecution.authorizeCreate(item, execution, reader),
        );
    },

    /** Insert a created object under an unused identifier and require guarded writes. */
    async insertAuthorized(
        context: MethodContext,
        execution: StoreExecution,
        insert: (created: MethodContext<MethodConfiguration, unknown, string>) => Promise<unknown>,
    ): Promise<{ readonly id: string; readonly row: unknown }> {
        const { id, created, row } = await RecordExecution.insertCreated(
            context,
            execution,
            insert,
        );
        await execution.authorization.requireWritable(created, id, execution);

        return { id, row };
    },

    /** Create one object per item as a separate creation does, returning their identifiers. */
    async createItems(context: MethodContext, execution: Execution): Promise<{ ids: string[] }> {
        // append an events type's items to the event log at the home
        const { object } = context;
        if (object.storage === "events" && !execution.isPredicted) {
            const rows = await RecordExecution.eachItem(context, async (item) =>
                ObjectRecord.createdRow(
                    object.table,
                    await RecordExecution.createdValues(item, execution),
                ),
            );
            await EventLog.append(context, execution, rows);

            return { ids: rows.map((row) => schema.string().parse(row["id"])) };
        }

        // create independent items in one write at the home and require each to be writable
        const isSequential = object.ordering !== undefined || object.numbering !== undefined;
        if (!isSequential && !execution.isPredicted) {
            const ids = await RecordExecution.createTogether(context, execution);
            await RecordExecution.eachItem(context, (item, itemIndex) =>
                execution.authorization.requireWritable(item, nth(ids, itemIndex), execution),
            );

            return { ids };
        }

        // create one item at a time otherwise
        const ids = await RecordExecution.eachItem(context, async (item) => {
            // insert a prediction under the identifier its call carries, whose home records the access
            if (execution.isPredicted) {
                const id = present(item.id, `the identifier of a predicted ${item.object.name}`);
                await RecordExecution.createObject(item.identify(id), execution);

                return id;
            }
            // insert under an unused identifier and record its access at the home
            else {
                const { id } = await RecordExecution.insertAuthorized(item, execution, (created) =>
                    RecordExecution.createObject(created, execution),
                );

                return id;
            }
        });

        return { ids };
    },

    /** Insert every item's row in one write, reporting the item a duplicate or creation refuses. */
    async createTogether(context: MethodContext, execution: StoreExecution): Promise<string[]> {
        // build each item's row under an unused identifier
        const created = await RecordExecution.eachItem(context, async (item) => {
            const id = await RecordExecution.createdId(item);
            const row = ObjectRecord.createdRow(
                context.object.table,
                await RecordExecution.createdValues(item.with({ id }), execution),
            );

            return { item: item.with({ id }), id, row };
        });

        // write the rows and record each creation's access within one savepoint
        const table = context.object.table;
        try {
            await execution.transaction.savepoint(async () => {
                await Execution.write(
                    execution,
                    context,
                    ...created.map(({ row }) => ({ table, before: null, after: row })),
                );
                for (const [itemIndex, { item, id }] of created.entries()) {
                    await RecordExecution.recordCreation(item, id, execution).catch(
                        (error: unknown) => {
                            throw ObjectRecord.itemFailure(context.object, itemIndex, error);
                        },
                    );
                }
            });
        } catch (error) {
            // report the first item duplicating a stored row or an earlier item, as the caller sees it
            const itemIndex = DatabaseError.is(error, "ROW_EXISTS")
                ? await RecordExecution.duplicateIndex(context, execution, created, error)
                : undefined;
            const failing = itemIndex === undefined ? undefined : created[itemIndex];
            if (itemIndex === undefined || failing === undefined) {
                throw error;
            }
            await RecordExecution.requireCreatable(failing.item, execution);
            throw ObjectRecord.itemFailure(context.object, itemIndex, error);
        }

        return created.map(({ id }) => id);
    },

    /** Find the first created row repeating a stored or an earlier row's duplicate key. */
    async duplicateIndex(
        context: MethodContext,
        execution: StoreExecution,
        created: readonly { readonly row: Row }[],
        error: unknown,
    ): Promise<number | undefined> {
        // read each row's value of the duplicate key's columns, by field
        const violated = DatabaseError.is(error, "ROW_EXISTS") ? error.data : null;
        const keyFields = present(violated, "the duplicate's unique key").columns;
        const keys = created.map(({ row }) =>
            Object.fromEntries(keyFields.map((field) => [field, row[field]])),
        );

        // pass a key of structured values, which no equality condition reads
        const matched = keys.map((key) => MATCH.safeParse(key));
        if (matched.some((entry) => !entry.success)) {
            return undefined;
        }

        // find the first row repeating a stored or an earlier key
        const conditions = matched.flatMap((entry) =>
            entry.success ? [Condition.equal(entry.data)] : [],
        );
        const stored = await execution.transaction
            .read(context.object.table)
            .findMany({ where: { OR: conditions } });
        const taken = new Set(
            stored.map((found) => {
                const row: Row = found;

                return canonicalize(keyFields.map((field) => row[field]));
            }),
        );
        const itemIndex = keys.findIndex((key) => {
            // take a key seen before, and remember this one
            const value = canonicalize(keyFields.map((field) => key[field]));
            const isTaken = taken.has(value);
            taken.add(value);

            return isTaken;
        });

        return itemIndex < 0 ? undefined : itemIndex;
    },

    /** Run a call per item of a bulk creation in order, prefixing a failure with its index. */
    async eachItem<Result>(
        context: MethodContext,
        run: (item: MethodContext, itemIndex: number) => Promise<Result>,
    ): Promise<Result[]> {
        const results: Result[] = [];
        for (const [itemIndex, { id, ...written }] of BatchItem.array()
            .parse(context.input["items"])
            .entries()) {
            // run the item as a separate creation's call
            const item = context.with({
                input: written,
                ...(id === undefined ? {} : { id: schema.string().parse(id) }),
            });
            try {
                results.push(await run(item, itemIndex));
            } catch (error) {
                throw ObjectRecord.itemFailure(context.object, itemIndex, error);
            }
        }

        return results;
    },

    /** Update every matching object the caller may change in one write, returning the count. */
    async updateMatching(
        context: MethodContext,
        execution: Execution,
        written: readonly string[],
    ): Promise<{ count: number }> {
        // stamp the written fields over each match, refusing a managed object
        const table = context.object.table;
        const values = ObjectRecord.writtenValues(table, context.input, written);
        const rows = await RecordExecution.matchingRows(context, execution);
        const changes = rows.map((row) => {
            const each = context.at(row);
            RecordExecution.requireUnmanagedFields(each, execution, written);

            return {
                table,
                before: row,
                after: Table.values(table, Row.parse({ ...row, ...each.revised(values) })),
            };
        });

        // write every match at once
        await Execution.write(execution, context, ...changes);

        return { count: changes.length };
    },

    /** Update each listed object at its revision in one write, all or none. */
    async updateItems(
        context: MethodContext,
        execution: Execution,
        written: readonly string[],
    ): Promise<{ count: number }> {
        // read the listed objects in the call's scope, filtering the permitted ones on the server
        const table = context.object.table;
        const items = BatchItem.array().parse(context.input["items"]);
        const named = items.map((item) => UPDATED_ITEM.parse(item));
        const listed: OpenCondition = {
            scope: context.scope,
            id: { in: named.map((item) => item.id) },
        };
        const rows = await Execution.permitted(
            execution,
            context.object,
            context.permission(),
            await execution.transaction.read(table).findMany({ where: listed }),
        );
        const targets = new Map(rows.map((row) => [RecordedRow.parse(row).id, row]));

        // stamp each item's written fields at its revision, as an update of one object does
        const changes = zip(items, named).map(([item, { id, revision }]) => {
            // refuse a missing or unpermitted object
            const target = targets.get(id);
            if (target === undefined) {
                throw CallError.http("NOT_FOUND", {
                    message: `${context.object.name} not found`,
                });
            }
            // refuse a stale revision
            else if (RecordedRow.parse(target).revision !== revision) {
                throw ObjectError.of(
                    "REVISION_CONFLICT",
                    `cannot update ${context.object.name}: its revision changed`,
                );
            }

            // write the item's fields over its target, refusing a managed object
            const each = context.at(target);
            RecordExecution.requireUnmanagedFields(each, execution, written);
            const values = ObjectRecord.writtenValues(table, item, written);

            return {
                table,
                before: target,
                after: Table.values(table, Row.parse({ ...target, ...each.revised(values) })),
            };
        });

        // write every item at once
        await Execution.write(execution, context, ...changes);

        return { count: changes.length };
    },

    /** Delete every matching object the caller may delete, each as its deletion does. */
    async deleteMatching(context: MethodContext, execution: Execution): Promise<{ count: number }> {
        // take each match as its target
        const rows = await RecordExecution.matchingRows(context, execution);
        const targets = rows.map((row) => context.at(row));
        for (const target of targets) {
            RecordExecution.requireUnmanaged(target, execution);
        }

        // delete or trash the read matches in one write
        const table = context.object.table;
        await Execution.write(
            execution,
            context,
            ...targets.map((target) => ({ table, ...RecordExecution.deletion(target) })),
        );

        return { count: rows.length };
    },

    /** Read a loaded target's deletion: its trashed row under a trash, else its removal. */
    deletion(target: MethodContext<{ readonly table: Table }, unknown>): {
        readonly before: Row;
        readonly after: Row | null;
    } {
        // remove an object deleting at once
        const before: Row = target.target;
        if (!target.object.defersDeletion()) {
            return { before, after: null };
        }

        // trash it at the next revision, recording who asked
        const trashed = target.revised(RecordExecution.trashed(target));

        return {
            before,
            after: Table.values(target.object.table, Row.parse({ ...before, ...trashed })),
        };
    },

    /** Read the columns trashing a call's target: the time, and the subject asking. */
    trashed(context: MethodContext) {
        return {
            deletedAt: context.now,
            ...ObjectReference.columns("deletedBy", context.caller.subject),
        };
    },

    /** Read the listed objects a bulk call matches, the permitted ones on a server. */
    async matchingRows(context: MethodContext, execution: Execution) {
        // match the values in the call's scope, leaving out objects deleting or archived, as lists do
        const table = context.object.table;
        const matched = MATCH.parse(context.input["where"]);
        const isDeleting = Object.hasOwn(table[TABLE].columns, "deletedAt");
        const isArchivable = context.object.lifecycle.archivable !== undefined;
        const where = Condition.equal({
            ...matched,
            ...(isDeleting ? { deletedAt: null } : {}),
            ...(isArchivable ? { archivedAt: null } : {}),
            scope: context.scope,
        });

        // read the matches, filtering the permitted ones at the home
        const rows = await Execution.permitted(
            execution,
            context.object,
            context.permission(),
            await execution.transaction.read(table).findMany({ where }),
        );

        // refuse matching a predefined row
        rows.forEach((row) => context.object.refusePredefined(RecordedRow.parse(row).id));

        return rows;
    },

    /** Insert a created object. */
    async createObject(
        context: MethodContext<MethodConfiguration, unknown, string>,
        execution: Execution,
    ): Promise<Select<Table>> {
        // write the created row and record its number, then record its access
        const row = ObjectRecord.createdRow(
            context.object.table,
            await RecordExecution.createdValues(context, execution),
        );
        await Execution.write(execution, context, {
            table: context.object.table,
            before: null,
            after: row,
        });
        if (context.object.numbering !== undefined && !execution.isPredicted) {
            await Numbered.raise(context.object, row, context, execution);
        }
        await RecordExecution.recordCreation(context, context.id, execution);

        return row;
    },

    /** Record a created object's access: a new scope's chain and its first relationships. */
    async recordCreation(context: MethodContext, id: string, execution: Execution): Promise<void> {
        const creating = RecordExecution.readCreation(context);
        const isScope = context.object.policy.definition.scope === true;
        if (!execution.isPredicted && (isScope || creating !== undefined)) {
            const object = context.object.reference(context.scope, id);
            await execution.authorization.create(object, (await creating?.(context, object)) ?? {});
        }
    },

    /** Build the columns a created object is inserted with. */
    async createdValues(context: MethodContext, execution: Execution): Promise<InsertValue<Table>> {
        // fill caller fields with a calling principal of the declared kinds, a reference by its identifier column
        const { object } = context;
        const caller = context.caller.subject;
        const isServerOnly = !Method.isRouted(context.method);
        const callers: Record<string, string | null> = {};
        for (const [name, declared] of Object.entries(object.fields)) {
            const kinds = declared.principals;
            if (!declared.isCallerFilled || isServerOnly || declared.reference === "none") {
                continue;
            } else if (
                caller === undefined ||
                !(kinds === undefined ? isPrincipal(caller) : kinds.some((kind) => kind.is(caller)))
            ) {
                const named = kinds?.map((kind) => kind.name).join(" or ") ?? "principal";
                throw ObjectError.of(
                    "CREATOR_KIND_DENIED",
                    `${object.name} is forbidden: only a '${named}' can create it`,
                );
            }
            Object.assign(
                callers,
                declared.reference === "object"
                    ? ObjectReference.columns(declared.relationAt(name), caller)
                    : { [name]: caller.id },
            );
        }

        // place an ordered object after its last sibling unless the caller placed it
        const decoded = Table.decode(object.table, context.input);
        const parent = object.parent === undefined ? {} : context.parentColumns();
        const position =
            object.ordering === undefined || context.input["position"] !== undefined
                ? undefined
                : await Orderable.place(
                      object,
                      { ...decoded, ...parent, scope: context.scope },
                      "last",
                      context,
                  );

        // require the writer
        if (object.storage === "ephemeral" && context.writerId === undefined) {
            throw ObjectError.of(
                "WRITER_REQUIRED",
                `writer is required: only a writer can create ephemeral ${object.name} objects`,
            );
        }

        // assemble the columns, stamping the times and the author where the table records them
        const columns = object.table[TABLE].columns;
        const isTimed = Object.hasOwn(columns, "createdAt");
        const isAuthored = Object.hasOwn(columns, "createdById");
        const manager = Execution.manager(execution);
        const managed = manager === null || !object.isDeclarable() ? {} : Manager.values(manager);

        // number a numbered object, unless its creator assigned the number or a later method issues it
        const numbered =
            object.numbering === undefined
                ? {}
                : await Numbered.created(
                      object,
                      { ...decoded, ...parent, scope: context.scope },
                      context,
                      execution,
                  );

        return {
            ...decoded,
            ...parent,
            ...(position === undefined ? {} : { position }),
            id: context.id,
            scope: context.scope,
            ...numbered,
            ...(object.storage === "ephemeral" ? { writerId: context.writerId } : {}),
            ...callers,
            ...(isTimed ? { createdAt: context.now, updatedAt: context.now } : {}),
            ...(isAuthored
                ? {
                      ...ObjectReference.columns("createdBy", caller),
                      ...ObjectReference.columns("updatedBy", caller),
                  }
                : {}),
            ...managed,
        };
    },

    /** Read the object that already exists under a creation's unique key, as written. */
    async existing(
        context: MethodContext,
        execution: StoreExecution,
        key: string | undefined,
    ): Promise<Row | undefined> {
        // find nothing without a key
        if (key === undefined) {
            return undefined;
        }

        // read the key's columns: the identifier, or a unique key's
        const declared = context.object.definition.constraints?.[key];
        const isUnique =
            declared?.kind === "unique" || (declared?.kind === "index" && declared.unique === true);
        if (key !== "id" && (declared === undefined || !isUnique || !("on" in declared))) {
            throw new TypeError(`${context.object.name} has no unique key '${key}'`);
        }
        const columns = declared !== undefined && "on" in declared ? declared.on : ["id"];

        // match the written values on those columns, the identifier the caller gives among them
        const written = Table.values(
            context.object.table,
            Row.parse(await RecordExecution.createdValues(context, execution)),
        );
        const values = context.id === undefined ? written : { ...written, id: context.id };
        if (columns.includes("id") && context.id === undefined) {
            throw new TypeError(
                `answering a ${context.object.name} on an id conflict needs the caller's id`,
            );
        }
        const matched = Object.fromEntries(columns.map((column) => [column, values[column]]));
        const where = Condition.equal(MATCH.parse(matched));
        const [row] = await context.read(context.object.table).findMany({ where, limit: 1 });
        if (row === undefined) {
            return undefined;
        }

        // answer it to a caller who could create it as written
        const grants = await execution.authorization.reader(context.scope);
        await RecordExecution.requirePermitted(context, execution, row, grants, "concealed");

        return row;
    },

    /** Insert a created object under an unused identifier within a savepoint. */
    async insertCreated(
        context: MethodContext,
        execution: StoreExecution,
        insert: (created: MethodContext<MethodConfiguration, unknown, string>) => Promise<unknown>,
    ): Promise<{
        readonly id: string;
        readonly created: MethodContext<MethodConfiguration, unknown, string>;
        readonly row: unknown;
    }> {
        // insert within a savepoint
        try {
            return await execution.transaction.savepoint(async () => {
                // insert under an unused identifier
                const id = await RecordExecution.createdId(context);
                const created = context.identify(id);

                return { id, created, row: await insert(created) };
            });
        } catch (error) {
            // report conflicts only to authorized callers
            const isConflict =
                CallError.of(error)?.code === "CONFLICT" ||
                DatabaseError.is(error, "ROW_EXISTS") ||
                DatabaseError.is(error, "REFERENCE_BLOCKED");
            if (isConflict) {
                await RecordExecution.requireCreatable(context, execution);
            }
            throw error;
        }
    },

    /** Require the create permission on the object as written. */
    async requireCreatable(
        context: MethodContext,
        execution: StoreExecution,
        reader?: Reader,
    ): Promise<void> {
        // build the row as it will be written, with column defaults
        const id = context.id ?? context.object.generateId(execution.store.environment);
        const definition = context.object.table[TABLE];
        const values = await RecordExecution.createdValues(context.with({ id }), execution);
        const row = definition.defaults(Table.values(context.object.table, Row.parse(values)));

        // grant the creation's first relationships, then check the permission on the row
        const { authorization } = execution;
        const grants = reader ?? (await authorization.reader(context.scope));
        const created = context.object.reference(context.scope, id);
        const initial =
            (await RecordExecution.readCreation(context)?.(context.with({ id }), created)) ?? {};
        const { authorizer } = authorization;
        grants.creating(
            created,
            row,
            authorizer.initialRelationships(
                created,
                initial,
                context.now,
                authorization.environment,
            ),
        );
        await RecordExecution.requirePermitted(context, execution, row, grants, "concealed");
    },

    /** Require a scoped method's permission in the call's scope, as on a new object there. */
    async authorizeScoped(
        context: MethodContext,
        execution: StoreExecution,
        selected: ((input: JsonObject) => Row) | undefined,
    ): Promise<void> {
        // read the selected columns, else the input's, and a nested type's parent
        const definition = context.object.table[TABLE];
        const input: JsonObject = context.input;
        const named =
            selected?.(input) ??
            Object.fromEntries(
                Object.keys(definition.columns)
                    .filter((column) => input[column] !== undefined)
                    .map((column) => [column, input[column]]),
            );
        const parent = context.parentColumns();

        // check the permission on a new object with those columns, under an unused identifier
        const id = context.object.generateId(execution.store.environment);
        const row = definition.defaults({ ...named, id, scope: context.scope, ...parent });
        await RecordExecution.requirePermitted(
            context,
            execution,
            row,
            await execution.authorization.reader(context.scope),
            "open",
        );
    },

    /** Require the call's permission on a row, challenging for step-up authentication. */
    async requirePermitted(
        context: MethodContext,
        execution: StoreExecution,
        row: Row,
        grants: Reader,
        refusal: "open" | "concealed",
    ): Promise<void> {
        // check the permission on the row
        const { authorization } = execution;
        const { authorizer, snapshot } = authorization;
        const permission = context.permission();
        const allows = async (access: Access) =>
            (await authorizer.checkRows(snapshot, permission, access, [row], grants)).permitted.has(
                0,
            );
        const access = await authorization.in(context.scope);
        if (await allows(access)) {
            return;
        }

        // refuse a gate's denial: a step-up or a suspended scope
        const refused = await authorizer.refusal(snapshot, permission, access, allows);
        if (refused !== undefined) {
            throw refused;
        }

        // refuse openly, or hide an object the caller cannot read
        const reading = context.object.reading;
        const isReadable =
            refusal === "open" ||
            (reading !== undefined &&
                (await authorizer.checkRows(snapshot, reading, access, [row])).permitted.has(0));
        const denial = ObjectError.of(
            "PERMISSION_DENIED",
            PermissionReference.forbidden(permission),
        );
        if (!isReadable) {
            throw conceal(denial, `${context.object.name} not found`);
        }

        // refuse a reader waiting for the approvals a grant or a role asks, else deny it
        const target = context.object.reference(context.scope, schema.string().parse(row["id"]));
        throw (await authorization.approvalRefusal(permission, target, access, allows)) ?? denial;
    },

    /** Delete an object, or request deletion when a trash or controller finishes it. */
    async deleteObject(
        context: MethodContext<{ readonly table: Table }, unknown>,
    ): Promise<Record<string, never>> {
        // refuse deleting an object twice
        const { object } = context;
        const target: Row = context.target;
        if (target["deletedAt"] !== undefined && target["deletedAt"] !== null) {
            throw ObjectError.of(
                "OBJECT_BLOCKED",
                `cannot delete ${object.name}: it is already deleted`,
            );
        }

        // request deletion when a trash or controller finishes it, moving the children nested to the trash with it
        if (object.defersDeletion()) {
            const trashed = RecordExecution.trashed(context);
            await context.update(trashed);
            await RecordExecution.trashChildren(
                context,
                schema.string().parse(target["id"]),
                "delete",
            );

            return {};
        }

        // delete at the loaded revision
        await context.remove();

        return {};
    },

    /** Trash a deleted object's live children with it, or restore the ones it trashed. */
    async trashChildren(
        context: MethodContext<{ readonly table: Table }, unknown>,
        parentId: string,
        verb: "delete" | "restore",
    ): Promise<void> {
        // find the types moving to the trash with this one
        const types = context.objects.filter(
            (type) =>
                type.parent?.delete === "trash" &&
                type.parent.object !== "any" &&
                type.parent.object.same(context.object),
        );
        for (const type of types) {
            // read its live children, or those the parent's deletion moved to the trash
            const where =
                verb === "delete"
                    ? { parentId, deletedAt: { isNull: true } }
                    : { parentId, isTrashedWithParent: true };
            const children = await context.read<Table>(type.table).findMany({ where });
            if (children.length === 0) {
                continue;
            }

            // run each child's verb as the parent's type, which the child's refusals still guard
            const calls = children.map((child) =>
                type.call(
                    verb,
                    type.target({ scope: context.scope, id: RecordedRow.parse(child).id }),
                ),
            );
            await context.mutate(context.self, calls);

            // mark them trashed with the parent, or no longer
            const written = await context.read<Table>(type.table).findMany({
                where: { id: { in: children.map((child) => RecordedRow.parse(child).id) } },
            });
            for (const child of written) {
                await context
                    .with({ object: type })
                    .at(child)
                    .update({ isTrashedWithParent: verb === "delete" });
            }
        }
    },

    /** Refuse a child under a missing parent, or one in the trash or archive. */
    async requireParentReady(context: MethodContext, parentId: string | null): Promise<void> {
        // check a child of a parent type with a trash or archive
        const declared = context.object.parent;
        if (
            declared === undefined ||
            declared.object === "any" ||
            (!declared.object.defersDeletion() &&
                declared.object.lifecycle.archivable === undefined) ||
            parentId === null
        ) {
            return;
        }
        const [parent] = await context
            .read<Table>(declared.object.table)
            .findMany({ where: { id: parentId } });

        // refuse a missing parent, one in the trash or being deleted, and one archived
        const isDeleted = declared.object.defersDeletion() && parent?.["deletedAt"] !== null;
        if (parent === undefined) {
            throw CallError.http("NOT_FOUND", {
                message: `${declared.object.name} '${parentId}' not found`,
            });
        } else if (isDeleted && declared.object.lifecycle.recoverable !== undefined) {
            throw ObjectError.of(
                "OBJECT_BLOCKED",
                `cannot add to ${declared.object.name} '${parentId}': it is in the trash`,
            );
        } else if (isDeleted) {
            throw ObjectError.of(
                "OBJECT_BLOCKED",
                `cannot add to ${declared.object.name} '${parentId}': it is being deleted`,
            );
        } else if (
            declared.object.lifecycle.archivable !== undefined &&
            parent["archivedAt"] !== null
        ) {
            throw ObjectError.of(
                "OBJECT_BLOCKED",
                `cannot add to ${declared.object.name} '${parentId}': it is archived`,
            );
        }
    },

    /** Take a caller's identifier, which the insert refuses once taken, or generate one. */
    async createdId(context: MethodContext): Promise<string> {
        const { object, id: chosen } = context;

        return chosen === undefined ? context.generateId(object) : object.idSchema.parse(chosen);
    },

    /** Refuse changing a managed object as a whole, such as deleting it, but as its manager. */
    requireUnmanaged(context: MethodContext, execution: Execution): void {
        // let the record's manager through
        const manager = Execution.manager(execution);
        if (context.isDeclarable() && context.target !== undefined) {
            Manager.requireChangeable(context.target, manager, context.object.name);
        }
    },

    /** Refuse changing a managed object's declared fields from anyone but its manager. */
    requireUnmanagedFields(
        context: MethodContext,
        execution: Execution,
        changed: readonly string[],
    ): void {
        // leave the fields no declaration sets to their own permissions
        if (!context.isDeclarable()) {
            return;
        }
        const declared = context.object.declaredFields();
        if (changed.some((name) => declared.has(name))) {
            RecordExecution.requireUnmanaged(context, execution);
        }
    },

    /** Read how a call creates its object's first relationships. */
    readCreation(
        context: MethodContext,
    ):
        | ((context: MethodContext, object: ObjectReference) => Creation | Promise<Creation>)
        | undefined {
        const hasOwnerRelation =
            context.object.roles !== undefined && context.object.policy.definition.scope === true;

        return (
            context.method.relate ??
            (hasOwnerRelation ? ObjectRecord.relateCreator("owner") : undefined)
        );
    },

    /** List a page of a scope's objects or aggregate groups. */
    list,

    /** Append rows of an events type to the event log. */
    append: (...rows: Parameters<typeof EventLog.append>) => EventLog.append(...rows),
};
