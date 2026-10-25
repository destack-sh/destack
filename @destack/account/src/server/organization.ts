import { Caller, principal } from "@destack/access";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { Subject } from "@destack/sync";
import { account } from "../object/account.ts";
import { organization } from "../object/organization.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** Implement organizations. */
export function implementOrganization(): typeof organization {
    return organization.implement({
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
        },
    });
}

/** Create an organization with its account, which pays for it and stores its home. */
async function create(
    context: MethodContextOf<typeof organization, "create">,
    next: NextOf<typeof organization, "create">,
): Promise<ResultOf<typeof organization, "create">> {
    // refuse any creator but a user or the account package acting as itself
    const creator = context.caller.subject;
    const isSelf = Subject.same(creator, Caller.principal(context.self));
    if (!isSelf && !principal.user.is(creator)) {
        throw AccountError.of("USER_REQUIRED", "only a user can create an organization");
    }

    // write it paying with the account created next
    const { handle, residencyId, stack } = context.input;
    const payer = account.generateId(context.environment);
    const row = await next(context.with({ input: { ...context.input, payer } }));

    // create its account as the store
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

/** Change an organization, refusing a payer outside its accounts. */
async function update(
    context: MethodContextOf<typeof organization, "update">,
    next: NextOf<typeof organization, "update">,
): Promise<ResultOf<typeof organization, "update">> {
    // refuse a payer outside the organization's accounts
    const payer = context.input.payer;
    if (typeof payer === "string") {
        const row = await context.read(account).findFirst({ where: { id: payer } });
        if (row?.scope !== context.target.id) {
            throw AccountError.of(
                "PAYER_FOREIGN",
                `account ${payer} is not an account of ${context.target.id}`,
            );
        }
    }

    return next();
}
