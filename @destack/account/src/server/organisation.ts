import { principal, type Relationship } from "@destack/access";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import { CallError } from "@destack/runtime/error";
import { account } from "../object/account.ts";
import { invitation } from "@destack/object";
import { member } from "../object/member.ts";
import { MemberRole, organisation } from "../object/organisation.ts";

/** Implement organisations. */
export function implementOrganisation(): typeof organisation {
    return organisation.implement({
        methods: {
            create: {
                creation: (context, object) => {
                    // relate its creating user as an owner
                    const creator = context.caller.subject;
                    const owners =
                        creator === undefined || !principal.user.is(creator)
                            ? []
                            : [{ relation: "owner", subject: creator }];

                    return {
                        relationships: owners,
                        owner: { ...object, relation: "owner" },
                    };
                },
                handle: create,
            },
            update,
            invite: async (call) => {
                // invite as the caller to the seat's relation
                const { relation, subject, purpose } = call.input;
                const [invited] = await call.mutate(call.caller, [
                    invitation.call("create", {
                        ...invitation.in(Scope.universe.id),
                        object: call.reference(),
                        relationship: { relation, subject },
                        ...(purpose === undefined ? {} : { purpose }),
                    }),
                ]);

                return invited;
            },
            accept,
        },
    });
}

/** Accept an invitation to an organisation, seating the accepting user under the invited role. */
async function accept(
    context: MethodContextOf<typeof organisation, "accept">,
): Promise<Relationship> {
    // relate the accepting user to the organisation
    const [relationship] = await context.mutate(context.caller, [
        invitation.call("accept", {
            ...invitation.in(Scope.universe.id),
            object: context.reference(),
            invitationId: context.input.invitationId,
        }),
    ]);
    const userId = schema.identifier("user").parse(relationship.subject.id);
    const role = MemberRole.parse("relation" in relationship ? relationship.relation : undefined);

    // refuse a user who has a seat already
    const organisationId = schema.identifier("organisation").parse(context.id);
    const seat = await context.read(member).findFirst({ where: { scope: organisationId, userId } });
    if (seat !== undefined) {
        throw new CallError("CONFLICT", { message: `member ${seat.id} is ${seat.status}` });
    }

    // seat the user under the invited role as the universe
    await context.mutate(context.self, [
        member.call("join", { ...member.in(organisationId), userId, role }),
    ]);

    return relationship;
}

/** Create an organisation with its account, which pays for it and stores its home. */
async function create(
    context: MethodContextOf<typeof organisation, "create">,
    next: NextOf<typeof organisation, "create">,
): Promise<ResultOf<typeof organisation, "create">> {
    // refuse any creator but a user or the universe
    const creator = context.caller.subject;
    const isUniverse = Subject.same(creator, Scope.universe);
    if (!isUniverse && !principal.user.is(creator)) {
        throw new CallError("FORBIDDEN", {
            message: "an organisation is created by a user",
        });
    }

    // write it paying with the account created next
    const { handle, residencyId, stack } = context.input;
    const payer = account.generateId();
    const row = await next(context.with({ input: { ...context.input, payer } }));

    // seat a creating user as its owner, leaving one the universe creates without members
    if (!isUniverse) {
        await context.mutate(context.self, [
            member.call("join", { ...member.in(row.id), userId: creator.id, role: "owner" }),
        ]);
    }

    // create its account as the universe
    await context.mutate(context.self, [
        account.call("create", {
            ...account.target({ scope: row.id, id: payer }),
            handle,
            name: row.name,
            kind: "shared",
            residencyId,
            ...(stack === undefined ? {} : { stack }),
        }),
    ]);

    return row;
}

/** Change an organisation, refusing a payer outside its own accounts. */
async function update(
    context: MethodContextOf<typeof organisation, "update">,
    next: NextOf<typeof organisation, "update">,
): Promise<ResultOf<typeof organisation, "update">> {
    // refuse a payer the organisation does not own
    const payer = context.input.payer;
    if (typeof payer === "string") {
        const row = await context.read(account).findFirst({ where: { id: payer } });
        if (row?.scope !== context.target.id) {
            throw new CallError("BAD_REQUEST", {
                message: `account ${payer} is no account of ${context.target.id}`,
            });
        }
    }

    return next();
}
