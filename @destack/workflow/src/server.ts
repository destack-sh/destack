import { defineServer } from "@destack/object/server";
import { CallError } from "@destack/runtime/error";
import { schema } from "@destack/schema";
import { Call, Subject } from "@destack/sync";
import { approval, change, ChangeRequest, changeRequest } from "./approval/index.ts";
import workflowPackage from "./package.ts";
import { LIVE_STATES, RunRecord } from "./run/run.ts";

/** The change runs as the workflow package serves them. */
const changes = change.implement((step, input, context) => ChangeRequest.run(step, input, context));

/** The workflow package's server, whose change requests run until approved or rejected. */
export default defineServer(workflowPackage, {
    objects: [
        approval,
        changeRequest.implement({
            methods: {
                create: async (context, next) => {
                    // refuse a requester asking to approve their own change, as a pull request's author cannot
                    const { approvers } = schema
                        .looseObject({ approvers: schema.array(Subject) })
                        .parse(context.input);
                    if (approvers.some((each) => Subject.same(each, context.caller.subject))) {
                        throw new CallError("BAD_REQUEST", {
                            message: "a requester cannot approve their own change",
                        });
                    }

                    // start the change's run as the requester, keyed by the change request
                    const row = await next();
                    await context.mutate(context.self, [
                        change.start(
                            { id: row.id, approvers },
                            { key: row.id, actor: context.caller.subject },
                        ),
                    ]);

                    return row;
                },

                apply: async (context, next) => {
                    // make the held call as the requester in the transaction applying the change
                    await context.mutate(context.caller, [Call.parse(context.target.call)]);

                    return next();
                },

                withdraw: async (context, next) => {
                    // cancel the change's live run
                    const live = await context.read(change.run).findMany({
                        where: { key: context.target.id, state: { in: [...LIVE_STATES] } },
                    });
                    await context.mutate(
                        context.caller,
                        live.map((row) =>
                            change.run.call("cancel", change.run.target(RunRecord.parse(row))),
                        ),
                    );

                    return next();
                },
            },
        }),
        ...changes.objects,
    ],
    controllers: changes.controllers,
});
