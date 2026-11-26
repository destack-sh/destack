import type { MethodContextOf, NextOf, ObjectReconciliation, ResultOf } from "@destack/object";
import { accessRelationship } from "@destack/access";
import type { Call } from "@destack/sync";
import type { Select } from "@destack/db";
import type { Snapshot } from "@destack/db/database";
import { type DigestCrypto, type Identifier, present, schema } from "@destack/schema";
import { SpaceAddress } from "../address/address.ts";
import { account } from "../object/account.ts";
import { machine } from "../object/machine.ts";
import { Placement, placement } from "../object/placement.ts";
import type { Entitlement } from "./entitlement.ts";
import { decideSchedule } from "./placement.ts";
import { Space, space } from "../object/space.ts";
import type { PlatformDefinition } from "./universe.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** Implement spaces, born with the platform's space stack unless given one. */
export function implementSpace(
    stacks: PlatformDefinition["stacks"],
    entitlement: (accountId: Identifier<"account">) => Entitlement,
): typeof space {
    return space.implement({
        methods: {
            create: {
                // make the space the owner of itself
                relate: (_call, created) =>
                    Space.creation({
                        id: schema.identifier("space").parse(created.id),
                        scope: account.identifier(created.scope),
                    }),
                handle: (context, next) => create(context, next, stacks, entitlement),
            },
            update: { handle: update },
            address,
        },
        controllers: [
            {
                // TODO #Performance: select only pending spaces, once space keeps its machine as Kubernetes' spec.nodeName
                name: "place",
                // place a live space without a placement once a machine admits its account
                when: { deletedAt: { isNull: true } },
                reads: [
                    // place again once a managed machine of the account's region comes or changes
                    {
                        table: machine.table,
                        fields: ["status", "deletedAt", "kind", "regionId"],
                        path: [
                            {
                                table: account.table,
                                cardinality: "many",
                                on: { kind: "key", column: "regionId", parent: "regionId" },
                            },
                        ],
                        on: { scope: "id" },
                    },
                    // place again once a machine admits a tenant
                    {
                        table: accessRelationship,
                        where: { relation: "tenant", objectType: machine.typeReference.type },
                        fields: ["subjectId"],
                        path: [
                            {
                                table: machine.table,
                                cardinality: "one",
                                on: { kind: "key", column: "id", parent: "objectId" },
                            },
                            {
                                table: account.table,
                                cardinality: "many",
                                on: { kind: "key", column: "regionId", parent: "regionId" },
                            },
                        ],
                        on: { scope: "id" },
                    },
                ],
                reconcile: (reconciliation) => place(reconciliation, entitlement),
            },
        ],
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

/** Create a space with its stack, placed at the first epoch once a machine takes it. */
async function create(
    context: MethodContextOf<typeof space, "create">,
    next: NextOf<typeof space, "create">,
    stacks: PlatformDefinition["stacks"],
    entitlement: (accountId: Identifier<"account">) => Entitlement,
): Promise<ResultOf<typeof space, "create">> {
    // create the space with its stack, the space stack by default
    const accountId = schema.identifier("account").parse(context.scope);
    const spaceId = space.identifier(context.id);
    const { machine: chosen } = context.input;
    const requested = chosen === undefined ? undefined : machine.identifier(chosen);
    const { crypto } = context.environment;
    const target = await pickMachine(context, crypto, accountId, spaceId, requested);
    const stack = context.input.stack ?? stacks.space;
    const created = await next(context.with({ input: { ...context.input, stack } }));

    // place it in the same transaction, else leave it pending until a machine admits its account
    if (target !== undefined) {
        await context.mutate(context.self, [
            await placementCall(
                context,
                crypto,
                { spaceId, scope: accountId, machineId: target },
                entitlement,
            ),
        ]);
    }

    return created;
}

/** Update a space, deciding its placement again when its tenancy changes. */
async function update(
    context: MethodContextOf<typeof space, "update">,
    next: NextOf<typeof space, "update">,
): Promise<ResultOf<typeof space, "update">> {
    // update the space
    const updated = await next(context);
    if (updated.tenancy === context.target.tenancy) {
        return updated;
    }

    // decide its placement again for the new tenancy
    const { scope, id } = context.target;
    await context.mutate(context.self, [placement.call("decide", placement.target({ scope, id }))]);

    return updated;
}

/** Read the machine a new space of an account runs on, none while no machine admits it. */
async function pickMachine(
    reader: Pick<Snapshot, "read">,
    crypto: DigestCrypto,
    accountId: Identifier<"account">,
    spaceId: Identifier<"space">,
    machineId: Identifier<"machine"> | undefined,
): Promise<Identifier<"machine"> | undefined> {
    // place it on the given machine the account may use
    if (machineId !== undefined) {
        return Placement.requireMachine(reader, { scope: accountId }, machineId);
    }

    // place it on the candidate scoring highest for it
    const owner = await reader.read(account.table).findFirst({ where: { id: accountId } });
    if (owner === undefined) {
        throw AccountError.of("ACCOUNT_NOT_FOUND", "account not found");
    }
    const shared = await Placement.candidates(reader, owner);
    const picked = await Placement.pick(spaceId, shared, crypto);

    return picked === undefined ? undefined : schema.identifier("machine").parse(picked);
}

/** Place each live space without a placement on a candidate, leaving the rest waiting. */
async function place(
    reconciliation: ObjectReconciliation<typeof space>,
    entitlement: (accountId: Identifier<"account">) => Entitlement,
): Promise<void> {
    // read which spaces have no placement yet
    const reader = reconciliation.database.snapshot();
    const placed = await reader.read(placement.table).findMany({
        where: { spaceId: { in: reconciliation.rows.map((row) => row.id) } },
    });
    const isPlaced = new Set<string>(placed.map((entry) => entry.spaceId));
    const pending = reconciliation.rows.filter((row) => !isPlaced.has(row.id));

    // place each on the machine that scores highest for it
    const { crypto } = reconciliation.store.environment;
    const calls: Call[] = [];
    for (const row of pending) {
        const accountId = schema.identifier("account").parse(row.scope);
        const spaceId = space.identifier(row.id);
        const target = await pickMachine(reader, crypto, accountId, spaceId, undefined);
        if (target !== undefined) {
            const taken = { spaceId, scope: accountId, machineId: target };
            calls.push(await placementCall(reader, crypto, taken, entitlement));
        }
    }
    if (calls.length > 0) {
        await reconciliation.mutate(calls);
    }
}

/** Read the call placing a space on a machine at the first epoch, decided for that machine. */
async function placementCall(
    reader: Pick<Snapshot, "read">,
    crypto: DigestCrypto,
    placed: Pick<Select<typeof placement.table>, "spaceId" | "scope" | "machineId">,
    entitlement: (accountId: Identifier<"account">) => Entitlement,
): Promise<Call> {
    const decided = await decideSchedule(reader, placed, crypto, entitlement);

    return placement.call("create", {
        ...placement.in(placed.scope),
        spaceId: placed.spaceId,
        machineId: placed.machineId,
        epoch: 1,
        schedule: decided,
    });
}
