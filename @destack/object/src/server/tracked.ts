import { type AccessExpression, type Authorizer } from "@destack/access";
import { Duration, Identifier, present } from "@destack/schema";
import { Subject } from "@destack/sync";
import { and, desc, eq, TABLE, type Row, LogPosition } from "@destack/db";
import type { Call } from "../method/call.ts";
import { Step } from "../method/step.ts";
import type { ObjectType } from "../object/object.ts";

import type { TrackedDefinition } from "../trait/tracked.ts";
/** The history of tracked objects: the reads it requires and the activities a served change records. */
export const TrackedHistory = {
    /** Require the objects a tracked object's read permission reads through to keep history. */
    require(objects: readonly ObjectType[], authorizer: Authorizer): void {
        // follow each tracked object's read permission
        const followed = new Set<string>();
        for (const object of objects.filter((served) => served.lifecycle.tracked !== undefined)) {
            const reading = present(object.reading, `the read permission of ${object.name}`);
            requireKept(object, object, reading.name, { objects, authorizer, followed });
        }
    },

    /** Record a served change in the caller's current or a new activity. */
    async record(
        options: TrackedDefinition,
        call: Call,
        before: Row | undefined,
        after: Row,
        from: LogPosition,
    ): Promise<void> {
        // find the caller's latest activity on the object
        const { object } = call;
        const table = options.activity.table;
        const caller = Subject.key(call.requireCaller());
        const latest = await latestActivity(options, call, caller);

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

/** How long a caller's changes continue one activity after their last. */
const SESSION: Duration = { minutes: 10 };

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

/** Read a caller's latest activity on a call's object, absent before its first. */
async function latestActivity(options: TrackedDefinition, call: Call, caller: string) {
    // read the newest activity of the caller on the object
    const { object } = call;
    const table = options.activity.table;
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

    return latest;
}
