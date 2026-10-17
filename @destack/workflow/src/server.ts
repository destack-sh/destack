import { AccessContext, Grant, principal } from "@destack/access";
import type { Table } from "@destack/db";
import { defineServer } from "@destack/object/server";
import { CallError } from "@destack/runtime/error";
import { schema } from "@destack/schema";
import { Call, ObjectReference, Subject } from "@destack/sync";
import { approval, change, ChangeRequest, changeRequest } from "./approval/index.ts";
import workflowPackage from "./package.ts";

/** The change runs as the workflow package serves them. */
const changes = change.implement((step, input, context) => ChangeRequest.run(step, input, context));

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
                    await ChangeRequest.start(context, row.id, approvers);

                    return row;
                },

                apply: async (context, next) => {
                    // make the call as its principal through its actors' grants, as the approved change
                    const delegator = await ChangeRequest.principal(context, context.target);
                    const reader = {
                        read: <Read extends Table>(table: Read) => context.read(table),
                    };
                    const recorded = context.target.actors;
                    const actors = recorded.length === 0 ? [context.self.subject] : recorded;
                    const acting = await Grant.chain(
                        reader,
                        { subject: delegator, actors },
                        context.now,
                    );
                    await context.mutate(
                        AccessContext.of(acting, context.now, { approval: context.target.id }),
                        [Call.parse(context.target.call)],
                    );

                    return next();
                },

                withdraw: async (context, next) => {
                    // cancel the change's live run
                    await ChangeRequest.cancel(context, context.target.id);

                    return next();
                },
            },
        }),
        ...changes.objects,
    ],
    controllers: changes.controllers,
});
