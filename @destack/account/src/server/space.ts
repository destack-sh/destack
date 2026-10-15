import { Scope } from "@destack/sync";
import type { Call, CallOf, NextOf, ResultOf } from "@destack/object";
import { type Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { SpaceAddress } from "../address/address.ts";
import { account } from "../object/account.ts";
import { machine } from "../object/machine.ts";
import { Placement, placement } from "../object/placement.ts";
import { Space, space } from "../object/space.ts";
import type { PlatformDeclaration } from "./platform.ts";

/** Implement spaces, born with the platform's space stack unless given one. */
export function implementSpace(stacks: PlatformDeclaration["stacks"]): typeof space {
    return space.implement({
        methods: {
            create: {
                // let the space own itself, written at its home as it is born
                creation: (_call, created) =>
                    Space.creation({ id: schema.identifier("space").parse(created.id) }),
                handle: (call, next) => create(call, next, stacks),
            },
            address,
        },
    });
}

/** Read a space's address from its name and its account's handle. */
async function address(
    call: CallOf<typeof space, "address">,
): Promise<ResultOf<typeof space, "address">> {
    const { target } = call;
    const owner = present(
        await call
            .read(account)
            .findFirst({ where: { id: schema.identifier("account").parse(target.scope) } }),
        `the account of ${target.id}`,
    );

    return { address: SpaceAddress.of({ space: target.name, handle: owner.handle }) };
}

/** Create a space with its stack, placed at the first epoch. */
async function create(
    call: CallOf<typeof space, "create">,
    next: NextOf<typeof space, "create">,
    stacks: PlatformDeclaration["stacks"],
): Promise<ResultOf<typeof space, "create">> {
    // create the space with its stack, the space stack by default
    const accountId = schema.identifier("account").parse(call.scope);
    const target = await pickMachine(call, accountId, call.requireId(), call.input.machine);
    const stack = call.input.stack ?? stacks.space;
    const created = await next(call.with({ input: { ...call.input, stack } }));

    // place it at the first epoch in the same transaction
    await call.mutate(call.self, [
        placement.call("create", {
            ...placement.in(accountId),
            id: call.requireId(),
            machineId: target,
            epoch: 1,
        }),
    ]);

    return created;
}

/** Read the machine a new space of an account runs on. */
async function pickMachine(
    call: Pick<Call, "read">,
    accountId: Identifier<"account">,
    spaceId: string,
    machineId: Identifier<"machine"> | undefined,
): Promise<Identifier<"machine">> {
    // place it on the given machine the account may use
    if (machineId !== undefined) {
        return Placement.requireAdmitted(call, { scope: accountId }, machineId);
    }

    // place it on the enabled shared machine of the account's residency scoring highest for it
    const owner = await call.read(account).findFirst({ where: { id: accountId } });
    if (owner === undefined) {
        throw new CallError("NOT_FOUND", { message: `no account ${accountId}` });
    }
    const shared = await call.read(machine).findMany({
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
