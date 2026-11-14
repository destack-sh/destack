import { type Authorizer } from "@destack/access";
import { Duration, Identifier, present } from "@destack/schema";
import { ObjectReference, type ReferenceColumns } from "@destack/sync";
import { TABLE, type Row, LogPosition, Table } from "@destack/db";
import { type MethodContext, RecordedRow } from "../method/context.ts";
import { Step } from "../method/step.ts";
import type { ObjectType } from "../object/object.ts";

import type { TrackedDefinition } from "../trait/tracked.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";

/** The history of tracked objects: its required reads and the activities changes record. */
export const TrackedHistory = {
    /** List each tracked object whose read permission reads through objects without history. */
    refusals(
        objects: readonly ObjectType[],
        authorizer: Authorizer,
    ): { readonly object: ObjectType; readonly error: TypeError }[] {
        // follow each tracked object's read permission on its own walk
        return objects
            .filter((served) => served.lifecycle.tracked !== undefined)
            .flatMap((object) => {
                const reading = present(object.reading, `the read permission of ${object.name}`);
                const error = TrackedHistory.refusal(object, object, reading.name, {
                    objects,
                    authorizer,
                    followed: new Set(),
                });

                return error === undefined ? [] : [{ object, error }];
            });
    },

    /** Record a served change in the caller's current or a new activity. */
    async record(
        options: TrackedDefinition,
        context: MethodContext,
        execution: StoreExecution,
        before: Row | undefined,
        after: Row,
        from: LogPosition,
    ): Promise<void> {
        // find the caller's latest activity on the object
        const { object } = context;
        const { id } = RecordedRow.parse(after);
        const table = options.activity.table;
        const caller = ObjectReference.columns(
            "caller",
            present(context.caller.subject, "the author of a tracked change"),
        );
        const latest = await TrackedHistory.latestActivity(options, context, id, execution, caller);

        // list the fields the change wrote, comparing their JSON forms
        const encoded = object.table[TABLE].encode(after);
        const prior = before === undefined ? {} : object.table[TABLE].encode(before);
        const changed = Object.keys(object.fields).filter(
            (name) => !Step.same(prior[name], encoded[name]),
        );

        // continue the activity within the session window
        const session = Duration.milliseconds(options.session ?? SESSION);
        if (latest !== undefined && latest.endedAt >= context.now - session) {
            const fields = [...new Set([...latest.fields, ...changed])];
            await Execution.write(execution, context, {
                table,
                before: latest,
                after: {
                    ...latest,
                    endedAt: context.now,
                    fields,
                    changes: latest.changes + 1,
                    revision: latest.revision + 1,
                    updatedAt: context.now,
                },
            });
        }
        // start an activity from the position before the change
        else {
            await Execution.write(execution, context, {
                table,
                before: null,
                after: Table.complete(table, {
                    id: Identifier.create("activity", execution.store.environment),
                    scope: context.scope,
                    parentPackageId: object.policy.definition.packageId,
                    parentType: object.name,
                    parentId: id,
                    ...caller,
                    startedAt: context.now,
                    endedAt: context.now,
                    from,
                    fields: changed,
                    changes: 1,
                    createdAt: context.now,
                    updatedAt: context.now,
                }),
            });
        }
    },

    /** Find the first object a permission reads through whose record decides it without history. */
    refusal(
        recorder: ObjectType,
        object: ObjectType,
        permission: string,
        served: {
            readonly objects: readonly ObjectType[];
            readonly authorizer: Authorizer;
            readonly followed: Set<string>;
        },
    ): TypeError | undefined {
        // follow each object's permission once
        const key = `${object.name}.${permission}`;
        if (served.followed.has(key)) {
            return undefined;
        }
        served.followed.add(key);

        // follow the expression's terms
        const types = [
            ...served.objects,
            ...served.objects.flatMap((candidate) => candidate.scopes),
        ];
        for (const term of object.terms(permission)) {
            // follow another permission of the same object
            if (term.kind === "permission") {
                const refused = TrackedHistory.refusal(recorder, object, term.name, served);
                if (refused !== undefined) {
                    return refused;
                }
            }
            // refuse a related type whose record decides the permission without history, then follow it
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
                    const refused =
                        target.isDecidedByRecord(term.permission) &&
                        target.table[TABLE].retention !== "history"
                            ? new TypeError(
                                  `object '${recorder.name}' records history but reads through '${target.name}', which has no history`,
                              )
                            : TrackedHistory.refusal(recorder, target, term.permission, served);
                    if (refused !== undefined) {
                        return refused;
                    }
                }
            }
        }

        return undefined;
    },

    /** Read a caller's latest activity on a call's object, absent before its first. */
    async latestActivity(
        options: TrackedDefinition,
        context: MethodContext,
        id: string,
        execution: StoreExecution,
        caller: ReferenceColumns<"caller">,
    ) {
        // read the newest activity of the caller on the object
        const { object } = context;
        const table = options.activity.table;
        const [latest] = await execution.transaction.read(table).findMany({
            where: {
                parentPackageId: object.policy.definition.packageId,
                parentType: object.name,
                parentId: id,
                ...caller,
            },
            orderBy: { endedAt: "desc" },
            limit: 1,
        });

        return latest;
    },
};

/** How long a caller's changes continue one activity after their last. */
const SESSION: Duration = { minutes: 10 };
