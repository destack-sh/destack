import { type Call } from "@destack/sync";
import { type MethodContextOf, type NextOf, type ResultOf } from "@destack/object";
import { Account, HOME_SPACE, account } from "../object/account.ts";
import { space } from "../object/space.ts";
import { Entitlement, entitlement } from "../object/entitlement.ts";
import { AccountError } from "../error/error.ts";
import type { PlatformDefinition } from "./universe.ts";

/** Implement accounts. */
export function implementAccount(stacks: PlatformDefinition["stacks"]): typeof account {
    return account.implement({
        methods: {
            create: {
                relate: (context, object) => Account.creation(context, object),
                handle: (context, next) => create(context, next, stacks),
            },
            entitle,
        },
    });
}

/** Set an account's entitlements to its allowances, writing only the features that changed. */
async function entitle(
    context: MethodContextOf<typeof account, "entitle">,
): Promise<ResultOf<typeof account, "entitle">> {
    // read the stored entitlements beside the allowances keyed by feature, refusing a repeated feature
    const accountId = context.target.id;
    const stored = await context.read(entitlement).findMany({ where: { scope: accountId } });
    const { allowances } = context.input;
    const allowed = new Map(allowances.map((each) => [Entitlement.key(each), each]));
    if (allowed.size !== allowances.length) {
        throw AccountError.of(
            "FEATURE_INVALID",
            "allowance is invalid: the account's allowances repeat a feature",
        );
    }

    // compare each stored entitlement with its allowance
    const calls: Call[] = stored.flatMap((row): Call[] => {
        const wanted = allowed.get(Entitlement.key(row));
        // delete one no allowance keeps
        if (wanted === undefined) {
            return [entitlement.call("delete", entitlement.target(row))];
        }
        // update a changed one
        else if (!Entitlement.isSame(row, wanted)) {
            const { kind, value, limit, resetAt, state, resource } = wanted;

            return [
                entitlement.call("update", {
                    ...entitlement.target(row),
                    kind,
                    value,
                    limit,
                    resetAt,
                    state,
                    resource,
                }),
            ];
        }
        // keep an unchanged one
        else {
            return [];
        }
    });

    // create the ones not stored yet
    const kept = new Set(stored.map((row) => Entitlement.key(row)));
    for (const [key, wanted] of allowed) {
        if (!kept.has(key)) {
            calls.push(entitlement.call("create", { ...wanted, ...entitlement.in(accountId) }));
        }
    }

    // write the changes as the account's home
    if (calls.length > 0) {
        await context.mutate(context.self, calls);
    }

    return context.target;
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
