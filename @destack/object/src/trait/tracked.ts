import {
    subjectKey,
    type AccessExpression,
    type Authorizer,
    type Permission,
} from "@destack/access";
import { and, desc, eq, TABLE, type Insert, type Table } from "@destack/db";
import { LogPosition } from "@destack/db/log";
import type { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { v7 } from "uuid";
import type { Call } from "../method/call.ts";
import { defineMethod, type Method } from "../method/method.ts";
import { Step } from "../method/step.ts";
import { Duration } from "../object/duration.ts";
import type { ObjectType } from "../object/object.ts";
import type { Procedure, ReplayShape, RowSchema, TargetShape } from "../method/procedure.ts";
import type { Gated, Trait } from "./trait.ts";

/** How long a caller's changes continue one activity after their last. */
const SESSION: Duration = { minutes: 10 };

/** The options of history tracking. */
export interface TrackedDefinition<Permissions extends string = string> extends Gated<Permissions> {
    /** The activity type among the object's attachments. */
    readonly activity: ObjectType;
    /** How long one activity continues after its last change, 10 minutes by default. */
    readonly session?: Duration;
}

/** The methods history derives. */
export type TrackedMethodMap<History> = History extends TrackedDefinition
    ? {
          readonly history: Method<"history", string, never, never, false>;
          readonly revert: Method<"revert", History["by"], never, never, true>;
      }
    : {};

/** The procedures history derives. */
export type TrackedProcedures<Object extends ObjectType> = {
    history: Procedure<schema.Object<TargetShape<Object> & PositionField>, RowSchema<Object>>;
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
        call: Call,
        before: Readonly<Record<string, unknown>> | undefined,
        after: Readonly<Record<string, unknown>>,
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

        return { history: historyMethod(reading), revert: revertMethod(options.by) };
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
        for (const object of objects.filter((served) => served.tracked !== undefined)) {
            requireKept(object, object, object.reading!.name, { objects, authorizer, followed });
        }
    },
    async record(call, before, after, from) {
        // find the caller's latest activity on the object
        const { object } = call;
        const options = object.tracked!;
        const activity = options.activity;
        const table = activity.table as Table & Record<string, never>;
        const caller = subjectKey(call.caller!);
        const [latest] = (await call.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.parentPackageId, object.policy.definition.packageId),
                    eq(table.parentType, object.name),
                    eq(table.parentId, call.id!),
                    eq(table.caller, caller),
                ),
            )
            .orderBy(desc(table.endedAt))
            .limit(1)) as Record<string, unknown>[];

        // list the fields the change wrote
        const changed = Object.keys(object.fields).filter(
            (name) => !Step.same(before?.[name], after[name]),
        );

        // continue the activity within the session window
        const session = Duration.milliseconds(options.session ?? SESSION);
        if (latest !== undefined && (latest.endedAt as number) >= call.now - session) {
            const fields = [...new Set([...(latest.fields as string[]), ...changed])];
            await call.database
                .update(table)
                .set({
                    endedAt: call.now,
                    fields,
                    changes: (latest.changes as number) + 1,
                    revision: (latest.revision as number) + 1,
                    updatedAt: call.now,
                } as Partial<Insert<Table>>)
                .where(eq(table.id, latest.id as string));
        }
        // start an activity from the position before the change
        else {
            await call.database.insert(table).values({
                id: `${activity.identity}-${v7()}`,
                scope: call.scope,
                parentPackageId: object.policy.definition.packageId,
                parentType: object.name,
                parentId: call.id!,
                caller,
                startedAt: call.now,
                endedAt: call.now,
                from,
                fields: changed,
                changes: 1,
                createdAt: call.now,
                updatedAt: call.now,
            } as Insert<Table>);
        }
    },
};

/** Read an object as it was at a position. */
function historyMethod(permission: string): Method {
    return defineMethod<Method<"history">>({
        kind: "history",
        permission,
        mutates: false,
        isPredicted: false,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/history" },
            input: shapes.target.extend({ at: LogPosition }),
            output: shapes.row,
        }),
        effect: (call) => earlier(call, call.object.permission(call.method.permission!)),
    });
}

/** Revert an object's written fields to a position. */
function revertMethod(permission: string): Method {
    return defineMethod<Method<"revert">>({
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
        effect: async (call) => {
            // write back changed fields
            const row = await earlier(call, call.object.reading!);
            const target = call.target as Record<string, unknown>;
            const changes = Object.fromEntries(
                call.object.written
                    .filter((name) => !Step.same(row[name], target[name]))
                    .map((name) => [name, row[name]]),
            );

            return Object.keys(changes).length === 0 ? target : call.revise(changes);
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
                ...Step.target(step, step.input.id),
                ...restored,
            });

            return Object.keys(restored).length === 0 || update === undefined
                ? undefined
                : [update];
        },
    });
}

/** Read the call's object at the input's position. */
async function earlier(call: Call, permission: Permission): Promise<Record<string, unknown>> {
    // read the object as it was
    const at = LogPosition.parse(call.input.at);
    const authorization = call.served();
    const snapshot = call.database.log.at(at);
    const row = await snapshot.row(call.object.table as Table, { id: call.id! });
    if (row === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no ${call.object.name} ${call.id!}` });
    }

    // check the permission as of then
    const scope = authorization.authorizer.governingScope(call.reference());
    const access = await authorization.authorizer.resolve(
        snapshot,
        scope,
        authorization.context(scope),
    );
    const decision = await authorization.authorizer.check(
        snapshot,
        permission,
        call.reference(),
        access,
    );
    if (!decision.isAllowed) {
        throw new ServiceError("NOT_FOUND", { message: `no ${call.object.name} ${call.id!}` });
    }

    return row;
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

    // follow the expression's parts
    const types = [...served.objects, ...served.objects.flatMap((candidate) => candidate.scopes)];
    for (const part of parts(object.policy.definition.permissions[permission])) {
        // follow another permission of the same object
        if (part.kind === "permission") {
            requireKept(keeper, object, part.name, served);
        }
        // require each reached type to keep history
        else if (part.kind === "through") {
            const subjects = served.authorizer.relation(object.policy, part.relation).subjects;
            const reached = types.filter((candidate) =>
                subjects.some(
                    (subject) =>
                        candidate.policy.definition.packageId === subject.packageId &&
                        candidate.policy.definition.name === subject.type,
                ),
            );
            for (const target of new Set(reached)) {
                if ((target.table as Table)[TABLE].retention !== "history") {
                    throw new TypeError(
                        `object ${keeper.name} keeps history but reads through ${target.name}, which keeps none`,
                    );
                }
                requireKept(keeper, target, part.permission, served);
            }
        }
    }
}

/** List the permissions and relations an expression reads. */
function parts(expression: AccessExpression | undefined): AccessExpression[] {
    return expression === undefined
        ? []
        : expression.kind === "union" || expression.kind === "intersection"
          ? expression.expressions.flatMap(parts)
          : expression.kind === "exclusion"
            ? [...parts(expression.include), ...parts(expression.exclude)]
            : [expression];
}
