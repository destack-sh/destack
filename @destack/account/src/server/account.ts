import type { Call } from "@destack/sync";
import { Relationship } from "@destack/access";
import type { Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import { Account, HOME_SPACE, account } from "../object/account.ts";
import { organization } from "../object/organization.ts";
import { relationship } from "@destack/object";
import { Residency, residency } from "../object/residency.ts";
import { StoreRelationship } from "./relationship.ts";
import { space } from "../object/space.ts";
import { Entitlement, entitlement } from "../object/entitlement.ts";
import { AccountError } from "../error/error.ts";
import type { PlatformDefinition } from "./universe.ts";

/** Implement accounts. */
export function implementAccount(stacks: PlatformDefinition["stacks"]): typeof account {
    return account.implement({
        methods: {
            create: {
                creation: (context, object) => Account.creation(context, object),
                handle: (context, next) => create(context, next, stacks),
            },
            purge: { handle: (context, next) => purge(context, next) },
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
        throw AccountError.of("FEATURE_REPEATED", "the account's allowances repeat a feature");
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
    // read the owner
    const owner = await Scope.object(context, context.scope);
    const isUser = !organization.is(owner);

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

    // relate the account's installations to its residency as residents
    await StoreRelationship.touch(context, Account.resident(created));

    // relate an owner organization to the servers of the account's residency
    if (!isUser) {
        const organizationId = schema.identifier("organization").parse(owner.id);
        await context.mutate(context.self, await residentCalls(context, organizationId));
    }

    return created;
}

/** Purge an account, unrelating its installations from its residency as residents. */
async function purge(
    context: MethodContextOf<typeof account, "purge">,
    next: NextOf<typeof account, "purge">,
): Promise<ResultOf<typeof account, "purge">> {
    // unrelate the account's installations from its residency as residents
    await StoreRelationship.delete(context, Account.resident(context.target));

    return next(context);
}

/** List the calls relating an organization to the servers of its accounts' residencies. */
export async function residentCalls(
    reader: Pick<Snapshot, "read">,
    organizationId: Identifier<"organization">,
): Promise<Call[]> {
    // read the residencies of its accounts and the ones it relates now
    const accounts = await reader
        .read(account.table)
        .findMany({ where: { scope: organizationId } });
    const wanted = new Set<string>(accounts.map((row) => row.residencyId));
    const object = organization.reference(Scope.universe.id, organizationId);
    const related = await Relationship.readOn(reader, object, { relation: "resident" });

    // revoke the residencies it left, and grant the ones it lacks
    const current = new Set(related.map((entry) => entry.subjectId));
    const left = related.filter((entry) => !wanted.has(entry.subjectId));
    const lacking = [...wanted].filter((code) => !current.has(code));

    return [
        ...left.map((row) =>
            relationship.call("delete", { ...relationship.in(row.scope), id: row.id }),
        ),
        ...lacking.map((residencyId) =>
            relationship.call("create", {
                ...relationship.in(Scope.universe.id),
                object,
                relation: "resident",
                subject: Residency.servers(residency.identifier(residencyId)),
            }),
        ),
    ];
}
