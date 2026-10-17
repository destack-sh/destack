import { principal } from "@destack/access";
import { defineServer } from "@destack/object/server";
import { CallError } from "@destack/runtime/error";
import { schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { approval, change, Change, ChangeRequest, changeRequest } from "./approval/index.ts";
import workflowPackage from "./package.ts";

/** The change runs as the workflow package serves them. */
const changes = change.implement((step, input, context) => new Change(step, input, context).run());

/** The workflow package's server, whose change requests run until approved or rejected. */
export default defineServer(workflowPackage, {
    objects: [
        approval,
        changeRequest.implement({
            methods: {
                create: async (context, next) => {
                    // ask the approvers given, else the delegator of the caller's grant: its subject, or a space's owners
                    const { subject, actors } = context.caller;
                    const given = schema
                        .looseObject({ approvers: schema.array(Subject).exactOptional() })
                        .parse(context.input).approvers;
                    const delegator = principal.space.is(subject)
                        ? { ...subject, relation: "owner" }
                        : subject;
                    const approvers = given ?? [delegator];

                    // refuse a requester asking to approve their own change, as a pull request's author cannot
                    if (given?.some((each) => Subject.same(each, subject)) === true) {
                        throw new CallError("BAD_REQUEST", {
                            message: "a requester cannot approve their own change",
                        });
                    }

                    // record the caller's subject and actors, and start the change's run
                    const row = await next(
                        context.with({
                            input: {
                                ...context.input,
                                actors: [...actors],
                                ...ObjectReference.columns("requester", subject),
                            },
                        }),
                    );
                    await new ChangeRequest(row).start(context, approvers);

                    return row;
                },

                execute: async (context) => {
                    // make the approved call, recording its output on the change
                    const output = await new ChangeRequest(context.target).execute(context);
                    await context.update({ output });

                    return output;
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
