import { type AccessExpression, type Authorizer, type Permission } from "@destack/access";
import { Duration, Identifier, present, type schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { and, desc, eq, TABLE, type Row, LogPosition } from "@destack/db";
import type { Call } from "../method/call.ts";
import type { ObjectTable } from "../object/table.ts";
import type { ActivityTable } from "../object/history.ts";
import { defineMethod, type Method } from "../method/method.ts";
import { Step } from "../method/step.ts";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { Procedure, ReplayShape, RowSchema, TargetShape } from "../method/procedure.ts";
import type { Gated, Trait } from "./trait.ts";

/** How long a caller's changes continue one activity after their last. */
const SESSION: Duration = { minutes: 10 };

/** The options of history tracking. */
export interface TrackedDefinition<Permissions extends string = string> extends Gated<Permissions> {
    /** The activity type among the object's attachments. */
    readonly activity: ObjectOf<{ table: ActivityTable }>;
    /** How long one activity continues after its last change, 10 minutes by default. */
    readonly session?: Duration;
}

/** The methods history derives. */
export type TrackedMethodMap<History> = History extends TrackedDefinition
    ? {
          readonly revert: Method<{ kind: "revert"; permission: History["by"]; mutates: true }>;
      }
    : {};

/** The procedures history derives. */
export type TrackedProcedures<Object extends ObjectType> = {
    revert: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & PositionField>,
        RowSchema<Object>
    >;
};

/** The input field naming the log position an object is read or reverted at. */
type PositionField = { readonly at: typeof LogPosition };

/** Objects keeping their history, grouped into activities. */
export const tracked: Trait<TrackedDefinition> & {
    /** Require the objects a tracked object's read permission reads through to keep history. */
    require(objects: readonly ObjectType[], authorizer: Authorizer): void;
    /** Record a served change in the caller's current or a new activity. */
    record(
        options: TrackedDefinition,
        call: Call,
        before: Row | undefined,
        after: Row,
        from: LogPosition,
    ): Promise<void>;
} = {
    key: "tracked",
    isDurable: true,
    options: (definition) => definition.tracked,
    columns: () => ({}),
    constraints: () => [],
    methods: (options, declared) => {
        // read history with the permission reading the object
        const reading = ["get", "list"]
            .map((kind) => Object.values(declared).find((method) => method.kind === kind))
            .find((method) => method !== undefined)?.permission;
        if (reading === undefined || reading === null) {
            throw new TypeError("an object keeping history needs a get or list method");
        }

        return { revert: revertMethod(options.by) };
    },
    validate: (options, object) => {
        // require the activity attachment and a valid session
        if (!object.attachments.some((attachment) => attachment.object === options.activity)) {
            throw new TypeError(
                `object ${object.name} keeps history but takes no ${options.activity.plural}`,
            );
        }
        if (options.session !== undefined) {
            Duration.require(options.session, `history session of ${object.name}`);
        }
    },
    require(objects, authorizer) {
        // follow each tracked object's read permission
        const followed = new Set<string>();
        for (const object of objects.filter((served) => served.lifecycle.tracked !== undefined)) {
            const reading = present(object.reading, `the read permission of ${object.name}`);
            requireKept(object, object, reading.name, { objects, authorizer, followed });
        }
    },
    async record(options, call, before, after, from) {
        // find the caller's latest activity on the object
        const { object } = call;
        const activity = options.activity;
        const table = activity.table;
        const caller = Subject.key(call.requireCaller());
        const [latest] = await call.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.parentPackageId, object.policy.definition.packageId),
                    eq(table.parentType, object.name),
                    eq(table.parentId, call.requireId()),
                    eq(table.caller, caller),
                ),
            )
            .orderBy(desc(table.endedAt))
            .limit(1);

        // list the fields the change wrote, comparing their JSON forms
        const encoded = object.table[TABLE].encode(after);
        const prior = before === undefined ? {} : object.table[TABLE].encode(before);
        const changed = Object.keys(object.fields).filter(
            (name) => !Step.same(prior[name], encoded[name]),
        );

        // continue the activity within the session window
        const session = Duration.milliseconds(options.session ?? SESSION);
        if (latest !== undefined && latest.endedAt >= call.now - session) {
            const fields = [...new Set([...latest.fields, ...changed])];
            await call.database
                .update(table)
                .set({
                    endedAt: call.now,
                    fields,
                    changes: latest.changes + 1,
                    revision: latest.revision + 1,
                    updatedAt: call.now,
                })
                .where(eq(table.id, latest.id));
        }
        // start an activity from the position before the change
        else {
            await call.database.insert(table).values({
                id: Identifier.create("activity"),
                scope: call.scope,
                parentPackageId: object.policy.definition.packageId,
                parentType: object.name,
                parentId: call.requireId(),
                caller,
                startedAt: call.now,
                endedAt: call.now,
                from,
                fields: changed,
                changes: 1,
                createdAt: call.now,
                updatedAt: call.now,
            });
        }
    },
};

/** Revert an object's written fields to a position. */
function revertMethod(permission: string): Method {
    return defineMethod<{ kind: "revert" }>({
        kind: "revert",
        permission,
        mutates: true,
        isPredicted: false,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/revert" },
            input: shapes.target.extend({ ...shapes.replay, at: LogPosition }),
            output: shapes.row,
        }),
        handler: async (call: Call<ObjectTable>) => {
            // write back the changed fields the history keeps, sensitive ones being unlogged
            const row = await earlier(
                call,
                present(call.object.reading, `the read permission of ${call.object.name}`),
            );
            const target: Row = call.requireTarget();
            const kept = call.object.table[TABLE].encode(row);
            const current = call.object.table[TABLE].encode(target);
            const changes = Object.fromEntries(
                call.object.written.flatMap((name) => {
                    const value = row[name];

                    return value === undefined || Step.same(kept[name], current[name])
                        ? []
                        : [[name, value] as const];
                }),
            );

            return Object.keys(changes).length === 0
                ? target
                : call.update(call.object.table[TABLE].values(changes));
        },
        inverse: (step) => {
            // restore reverted fields unchanged since
            const current = step.current;
            if (current === undefined) {
                return undefined;
            }
            const restored = Object.fromEntries(
                step.object.written
                    .filter((name) => !Step.same(step.before?.[name], step.after?.[name]))
                    .filter((name) => Step.same(current[name], step.after?.[name]))
                    .map((name) => [name, step.before?.[name] ?? null]),
            );
            const update = Step.call(step, "update", {
                ...Step.target(step),
                ...restored,
            });

            return Object.keys(restored).length === 0 || update === undefined
                ? undefined
                : [update];
        },
    });
}

/** Read the call's object at the input's position, where the caller may read it then and now. */
function earlier(call: Call, permission: Permission): Promise<Row> {
    const snapshot = call.database.log.at(LogPosition.parse(call.input["at"]));

    return call.requireAuthorization().readIn(call, call.requireId(), snapshot, permission);
}

/** Require the objects a permission reads through to keep their history. */
function requireKept(
    keeper: ObjectType,
    object: ObjectType,
    permission: string,
    served: {
        readonly objects: readonly ObjectType[];
        readonly authorizer: Authorizer;
        readonly followed: Set<string>;
    },
): void {
    // follow each object's permission once
    const key = `${object.name}.${permission}`;
    if (served.followed.has(key)) {
        return;
    }
    served.followed.add(key);

    // follow the expression's terms
    const types = [...served.objects, ...served.objects.flatMap((candidate) => candidate.scopes)];
    for (const term of terms(object.policy.definition.permissions[permission])) {
        // follow another permission of the same object
        if (term.kind === "permission") {
            requireKept(keeper, object, term.name, served);
        }
        // require each related type to keep history
        else if (term.kind === "through") {
            const subjects = served.authorizer.relation(object.policy, term.relation).subjects;
            const related = types.filter((candidate) =>
                subjects.some(
                    (subject) =>
                        candidate.policy.definition.packageId === subject.packageId &&
                        candidate.policy.definition.name === subject.type,
                ),
            );
            for (const target of new Set(related)) {
                if (target.table[TABLE].retention !== "history") {
                    throw new TypeError(
                        `object ${keeper.name} keeps history but reads through ${target.name}, which keeps none`,
                    );
                }
                requireKept(keeper, target, term.permission, served);
            }
        }
    }
}

/** List the permissions and relations an expression reads. */
function terms(expression: AccessExpression | undefined): AccessExpression[] {
    // read nothing of an absent expression
    if (expression === undefined) {
        return [];
    }
    // read each branch of a union or intersection
    else if (expression.kind === "union" || expression.kind === "intersection") {
        return expression.expressions.flatMap(terms);
    }
    // read both sides of an exclusion
    else if (expression.kind === "exclusion") {
        return [...terms(expression.include), ...terms(expression.exclude)];
    }
    // read a term itself
    else {
        return [expression];
    }
}
