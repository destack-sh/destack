import { principal } from "@destack/access";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { Scope, Subject } from "@destack/sync";
import { account } from "../object/account.ts";
import { organisation } from "../object/organisation.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

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
        },
    });
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
        throw AccountError.of("USER_REQUIRED", "only a user can create an organisation");
    }

    // write it paying with the account created next
    const { handle, residencyId, stack } = context.input;
    const payer = account.generateId();
    const row = await next(context.with({ input: { ...context.input, payer } }));

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

/** Change an organisation, refusing a payer outside its accounts. */
async function update(
    context: MethodContextOf<typeof organisation, "update">,
    next: NextOf<typeof organisation, "update">,
): Promise<ResultOf<typeof organisation, "update">> {
    // refuse a payer outside the organisation's accounts
    const payer = context.input.payer;
    if (typeof payer === "string") {
        const row = await context.read(account).findFirst({ where: { id: payer } });
        if (row?.scope !== context.target.id) {
            throw AccountError.of(
                "FOREIGN_PAYER",
                `account ${payer} is not an account of ${context.target.id}`,
            );
        }
    }

    return next();
}
