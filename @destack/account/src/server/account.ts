import { type MethodContextOf, type NextOf, type ResultOf } from "@destack/object";
import { Account, HOME_SPACE, account } from "../object/account.ts";
import { space } from "../object/space.ts";
import type { PlatformDefinition } from "./universe.ts";

/** Implement accounts. */
export function implementAccount(stacks: PlatformDefinition["stacks"]): typeof account {
    return account.implement({
        methods: {
            create: {
                relate: (context, object) => Account.creation(context, object),
                handle: (context, next) => create(context, next, stacks),
            },
        },
    });
}

/** Create an account with its home space running its stack. */
async function create(
    context: MethodContextOf<typeof account, "create">,
    next: NextOf<typeof account, "create">,
    stacks: PlatformDefinition["stacks"],
): Promise<ResultOf<typeof account, "create">> {
    // create the account
    const { stack: given, ...input } = context.input;
    const created = await next(context.with({ input }));

    // create its home with its stack, the homes' by default
    const stack = given ?? stacks.home;
    await context.mutate(context.self, [
        space.call("create", {
            ...space.target({ scope: created.id, id: Account.home(created.id) }),
            name: HOME_SPACE,
            stack,
        }),
    ]);

    return created;
}
