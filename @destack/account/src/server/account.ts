import { Relationship } from "@destack/access";
import type { Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import type { CallOf, NextOf, ObjectCall, ResultOf } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import { Account, HOME_SPACE, account } from "../object/account.ts";
import { organisation } from "../object/organisation.ts";
import { relationship } from "../object/relationship.ts";
import { Residency } from "../object/residency.ts";
import { space } from "../object/space.ts";
import type { PlatformDeclaration } from "./platform.ts";

/** Implement accounts. */
export function implementAccount(stacks: PlatformDeclaration["stacks"]): typeof account {
    return account.implement({
        methods: {
            create: {
                creation: (call, object) => Account.creation(call, object),
                handle: (call, next) => create(call, next, stacks),
            },
        },
    });
}

/** Create an account with its owner's residency and its home space carrying its stack. */
async function create(
    call: CallOf<typeof account, "create">,
    next: NextOf<typeof account, "create">,
    stacks: PlatformDeclaration["stacks"],
): Promise<ResultOf<typeof account, "create">> {
    // read the owner
    const owner = await Scope.object(call, call.scope);
    const isUser = !organisation.is(owner);

    // create the account in its residency
    const residencyId = await Account.residency(call, call.scope, call.input.residencyId);
    const { stack: given, ...input } = call.input;
    const created = await next(call.with({ input: { ...input, residencyId } }));

    // create its home with its stack as the universe, a personal home's by default
    const stack = given ?? (created.kind === "personal" ? stacks.person : undefined);
    await call.mutate(call.self, [
        space.call("create", {
            ...space.target({ scope: created.id, id: Account.home(created.id) }),
            name: HOME_SPACE,
            ...(stack === undefined ? {} : { stack }),
        }),
    ]);

    // relate an owning organisation to the servers of the account's residency
    if (!isUser) {
        const organisationId = schema.identifier("organisation").parse(owner.id);
        await call.mutate(call.self, await residentCalls(call, organisationId));
    }

    return created;
}

/** List the calls relating an organisation to the servers of its accounts' residencies. */
export async function residentCalls(
    reader: Pick<Snapshot, "read">,
    organisationId: Identifier<"organisation">,
): Promise<ObjectCall[]> {
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
