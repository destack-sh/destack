import { Scope } from "@destack/sync";
import type { MethodContext, MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { type Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/object";
import { SpaceAddress } from "../address/address.ts";
import { account } from "../object/account.ts";
import { machine } from "../object/machine.ts";
import { Placement, placement } from "../object/placement.ts";
import { Space, space } from "../object/space.ts";
import type { SpaceKind, StackSource } from "../object/stack-source.ts";

/** Implement spaces, born with the platform's space stack unless given one. */
export function implementSpace(stacks: Readonly<Record<SpaceKind, StackSource>>): typeof space {
    return space.implement({
        methods: {
            create: {
                // make the space the owner of itself, written at its home as it is born
                creation: (_call, created) =>
                    Space.creation({ id: schema.identifier("space").parse(created.id) }),
                handle: (context, next) => create(context, next, stacks),
            },
            address,
        },
    });
}

/** Read a space's address from its name and its account's handle. */
async function address(
    context: MethodContextOf<typeof space, "address">,
): Promise<ResultOf<typeof space, "address">> {
    const { target } = context;
    const owner = present(
        await context
            .read(account)
            .findFirst({ where: { id: schema.identifier("account").parse(target.scope) } }),
        `the account of ${target.id}`,
    );

    return { address: SpaceAddress.of({ space: target.name, handle: owner.handle }) };
}

/** Create a space with its stack, placed at the first epoch. */
async function create(
    context: MethodContextOf<typeof space, "create">,
    next: NextOf<typeof space, "create">,
    stacks: Readonly<Record<SpaceKind, StackSource>>,
): Promise<ResultOf<typeof space, "create">> {
    // create the space with its stack, the space stack by default
    const accountId = schema.identifier("account").parse(context.scope);
    const spaceId = space.identifier(context.id);
    const target = await pickMachine(context, accountId, spaceId, context.input.machine);
    const stack = context.input.stack ?? stacks.space;
    const created = await next(context.with({ input: { ...context.input, stack } }));

    // place it at the first epoch in the same transaction
    await context.mutate(context.self, [
        placement.call("create", {
            ...placement.in(accountId),
            id: context.id,
            machineId: target,
            epoch: 1,
        }),
    ]);

    return created;
}

/** Read the machine a new space of an account runs on. */
async function pickMachine(
    context: Pick<MethodContext, "read">,
    accountId: Identifier<"account">,
    spaceId: Identifier<"space">,
    machineId: Identifier<"machine"> | undefined,
): Promise<Identifier<"machine">> {
    // place it on the given machine the account may use
    if (machineId !== undefined) {
        return Placement.requireMachine(context, { scope: accountId }, machineId);
    }

    // place it on the enabled shared machine of the account's residency scoring highest for it
    const owner = await context.read(account).findFirst({ where: { id: accountId } });
    if (owner === undefined) {
        throw new CallError("NOT_FOUND", { message: `no account ${accountId}` });
    }
    const shared = await context.read(machine).findMany({
        where: {
            scope: Scope.universe.id,
            kind: "cloud",
            residencyId: owner.residencyId,
            status: "enabled",
            revokedAt: { isNull: true },
        },
    });
    const picked = await Placement.pick(
        spaceId,
        shared.map((row) => row.id),
    );
    if (picked === undefined) {
        throw new CallError("UNAVAILABLE", {
            message: `no cloud machine serves the residency ${owner.residencyId}`,
        });
    }

    return schema.identifier("machine").parse(picked);
}
