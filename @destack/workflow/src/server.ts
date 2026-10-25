import { implementPackage } from "@destack/object/server";
import { ObjectReference, Subject } from "@destack/sync";
import {
    approval,
    ApprovalQuorum,
    change,
    Change,
    ChangeRequest,
    changeRequest,
} from "./approval/index.ts";
import workflowPackage from "./package.ts";
import type {} from "@destack/package/import-meta";
import { WorkflowError } from "./error/index.ts";

/** The change runs as the workflow package serves them. */
const changes = change.implement((step, input, context) => new Change(step, input, context).run());

/** The workflow package's server, whose change requests run until approved or rejected. */
export default implementPackage(workflowPackage, {
    objects: [
        approval,
        changeRequest.implement({
            methods: {
                create: async (context, next) => {
                    // refuse the acting principal asking itself, as a pull request's author cannot
                    const { subject, actors } = context.caller;
                    const acting = actors.at(-1) ?? subject;
                    const given = context.input.approvers;
                    if (given?.some((each) => Subject.same(each, acting)) === true) {
                        throw WorkflowError.of(
                            "SELF_APPROVAL",
                            "a requester cannot approve their own change",
                        );
                    }

                    // ask the approvers given, or those the caller's kind of request asks
                    const approvers = given ?? ChangeRequest.approvers(context.caller);
                    const count = context.input.count ?? 1;
                    ApprovalQuorum.require(count, approvers.length);

                    // record the caller's subject and actors, and start the change's run
                    const row = await next(
                        context.with({
                            input: {
                                ...context.input,
                                count,
                                actors: [...actors],
                                ...ObjectReference.columns("requester", subject),
                            },
                        }),
                    );
                    await new ChangeRequest(row).start(context, approvers);

                    return row;
                },

                apply: async (context, next) => {
                    // make the approved call under the approving grant, then apply the change with its output
                    const output = await new ChangeRequest(context.target).execute(context);
                    const applied = await next();

                    return await context.at(applied).update({ output });
                },

                withdraw: async (context, next) => {
                    // cancel the change's live run
                    await new ChangeRequest(context.target).cancel(context);

                    return next();
                },
            },
        }),
        ...changes.objects,
    ],
    controllers: changes.controllers,
});
