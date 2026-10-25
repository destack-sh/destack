import type { Call } from "@destack/sync";
import { Relationship } from "@destack/access";
import type { Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import { Account, HOME_SPACE, account } from "../object/account.ts";
import { organization } from "../object/organization.ts";
import { relationship } from "@destack/object";
import { Residency } from "../object/residency.ts";
import { StoreRelationship } from "./relationship.ts";
import { space } from "../object/space.ts";
import type { SpaceKind, StackSource } from "../object/stack-source.ts";

/** Implement accounts. */
export function implementAccount(stacks: Readonly<Record<SpaceKind, StackSource>>): typeof account {
    return account.implement({
        methods: {
            create: {
                creation: (context, object) => Account.creation(context, object),
                handle: (context, next) => create(context, next, stacks),
            },
            purge: { handle: (context, next) => purge(context, next) },
        },
    });
}

/** Create an account with its home space running its stack. */
async function create(
    context: MethodContextOf<typeof account, "create">,
    next: NextOf<typeof account, "create">,
    stacks: Readonly<Record<SpaceKind, StackSource>>,
): Promise<ResultOf<typeof account, "create">> {
    // read the owner
    const owner = await Scope.object(context, context.scope);
    const isUser = !organization.is(owner);

    // create the account
    const { stack: given, ...input } = context.input;
    const created = await next(context.with({ input }));

    // create its home with its stack, a person's home's or an account's home's by default
    const stack = given ?? (created.kind === "personal" ? stacks.person : stacks.account);
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
    const wanted = new Set(accounts.map((row) => row.residencyId));
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
                subject: Residency.servers(residencyId),
            }),
        ),
    ];
}
