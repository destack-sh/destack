import type { Call } from "@destack/sync";
import { Relationship } from "@destack/access";
import type { Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import { Account, HOME_SPACE, account } from "../object/account.ts";
import { organisation } from "../object/organisation.ts";
import { relationship } from "@destack/object";
import { Residency } from "../object/residency.ts";
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
        },
    });
}

/** Create an account with its owner's residency and its home space running its stack. */
async function create(
    context: MethodContextOf<typeof account, "create">,
    next: NextOf<typeof account, "create">,
    stacks: Readonly<Record<SpaceKind, StackSource>>,
): Promise<ResultOf<typeof account, "create">> {
    // read the owner
    const owner = await Scope.object(context, context.scope);
    const isUser = !organisation.is(owner);

    // create the account in its residency
    const residencyId = await Account.residency(context, context.scope, context.input.residencyId);
    const { stack: given, ...input } = context.input;
    const created = await next(context.with({ input: { ...input, residencyId } }));

    // create its home with its stack as the universe, a personal home's by default
    const stack = given ?? (created.kind === "personal" ? stacks.person : undefined);
    await context.mutate(context.self, [
        space.call("create", {
            ...space.target({ scope: created.id, id: Account.home(created.id) }),
            name: HOME_SPACE,
            ...(stack === undefined ? {} : { stack }),
        }),
    ]);

    // relate an owner organisation to the servers of the account's residency
    if (!isUser) {
        const organisationId = schema.identifier("organisation").parse(owner.id);
        await context.mutate(context.self, await residentCalls(context, organisationId));
    }

    return created;
}

/** List the calls relating an organisation to the servers of its accounts' residencies. */
export async function residentCalls(
    reader: Pick<Snapshot, "read">,
    organisationId: Identifier<"organisation">,
): Promise<Call[]> {
    // read the residencies of its accounts and the ones it relates now
    const accounts = await reader
        .read(account.table)
        .findMany({ where: { scope: organisationId } });
    const wanted = new Set(accounts.map((row) => row.residencyId));
    const object = organisation.reference(Scope.universe.id, organisationId);
    const related = await Relationship.readOn(reader, object, { relation: "resident" });

    // revoke the residencies it left, and grant the ones it lacks
    const current = new Set(related.map((entry) => entry.subjectId));
    const left = related.filter((entry) => !wanted.has(entry.subjectId));
    const lacking = [...wanted].filter((code) => !current.has(code));

    return [
        ...left.map((row) =>
            relationship.call("delete", {
                ...relationship.in(Scope.universe.id),
                object,
                relationshipId: row.id,
            }),
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
