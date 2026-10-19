import { binding, capture, deployment, installation } from "@destack/stack/object";
import { vault } from "@destack/vault/object";
import { VAULT_PROVIDER } from "@destack/vault/declare";
import { credentialVault } from "../stack/index.ts";
import forgePackage from "../package.ts";
import {
    AccessContext,
    anyone,
    Authorization,
    Caller,
    principal,
    Relationship,
} from "@destack/access";
import {
    account,
    key,
    machine,
    placement,
    residency,
    type ResidencyCode,
    residentShape,
    spaceShape,
    Space,
    space,
} from "@destack/account/object";
import { callTables } from "@destack/object/test";
import { connection, integration } from "@destack/connection";
import { type DatabaseConnection, defineDatabase, eq, Transaction } from "@destack/db";
import type { ObjectStore, ObjectStoreOptions } from "@destack/object/server";
import { Identifier, present, schema } from "@destack/schema";
import { type ObjectReference, type Publisher, Scope, type Subject, Uplink } from "@destack/sync";
import type { AccountFixture } from "@destack/account/test";
import { vaultTables } from "@destack/vault/stack";
import { issuer, packageObject, reference, release, repository, tag } from "../object/index.ts";
import { claimTable, Follow } from "@destack/object";

/** The residency whose home the fixture forges are installed in. */
export const RESIDENCY: ResidencyCode = "eu";

/** The tables of the residency home's main database that the forge uses, with its replicas. */
export const homeDatabase = defineDatabase({
    name: "main",
    tables: [
        ...[repository, reference, packageObject, release, tag, issuer].flatMap(
            (type) => type.tables,
        ),
        ...vaultTables,
        ...callTables,
    ],
    replicated: [
        account.table,
        residency.table,
        space.table,
        integration.table,
        connection.table,
        machine.table,
        key.table,
        placement.table,
        claimTable,
        installation.table,
        binding.table,
        capture.table,
        deployment.table,
    ],
});

/** The residency home's space row, as its machine replicates it, with the machine serving it. */
export type ResidencyHome = Pick<Space, "id" | "scope"> & {
    /** The cloud machine serving the home, following the account package with its own key. */
    readonly machine: { readonly id: Identifier<"machine">; readonly publisher: Publisher };
};

/** Read the residency home's space row and serving machine from the account package. */
export async function readHome(accounts: AccountFixture, id: string): Promise<ResidencyHome> {
    // read the row
    const spaceId = schema.identifier("space").parse(id);
    const [row] = await accounts.database
        .select({
            id: space.table.id,
            scope: space.table.scope,
        })
        .from(space.table)
        .where(eq(space.table.id, spaceId));

    // follow the account package as the machine the home is placed on
    const identity = await accounts.serving(spaceId);
    const client = accounts.machine(identity);
    const serving = { id: identity.machineId, publisher: Uplink.of(() => client.store) };

    return { ...present(row, `the home ${id}`), machine: serving };
}

/** Give birth to the home below its account's replicated chain, as its machine's placement does. */
export async function bearHome(objects: ObjectStore, home: ResidencyHome): Promise<void> {
    // read the account's ancestors from its replicated chain
    const [parent] = await objects.database
        .select({ ancestors: Scope.table.ancestors })
        .from(Scope.table)
        .where(eq(Scope.table.scope, home.scope));
    const ancestors = [home.scope, ...present(parent, `the chain of ${home.scope}`).ancestors];

    // record the space and its first access here, its home, the space owning itself
    const now = objects.clock.now();
    await Transaction.run(objects.database, async (transaction) => {
        // receive the space as the store's principal
        const authorization = new Authorization(objects.authorizer, transaction, () =>
            AccessContext.of(Caller.of(objects.principal), now),
        );
        await authorization.receive(
            space.reference(home.scope, home.id),
            ancestors,
            Space.creation({ id: home.id }),
        );

        // let everyone find the home as the space owning itself, as the residency stack declares
        const owner = principal.space.reference(Scope.universe.id, home.id);
        const owning = new Authorization(objects.authorizer, transaction, () =>
            AccessContext.of(Caller.of(owner), now),
        );
        await Relationship.grant(
            {
                object: space.reference(home.scope, home.id),
                relation: "guest",
                subject: anyone.reference("*", "*"),
            },
            owning,
        );
    });
}

/** Provision the forge's credential vault in its home, bound to it, as the residency stack does. */
export async function provisionVault(
    database: DatabaseConnection,
    home: string,
    forge: Subject,
): Promise<ObjectReference> {
    // store the vault in the home
    const now = Date.now();
    const id = vault.identifier(Identifier.create("vault"));
    await database.insert(vault.table).values({
        id,
        scope: home,
        name: credentialVault.name,
        definitionPackageId: forgePackage.id,
        definitionVersion: forgePackage.version,
        definitionName: credentialVault.name,
        spec: {},
        provider: VAULT_PROVIDER,
        reference: vault.package.name,
        createdAt: now,
        updatedAt: now,
    });

    // bind the forge as the vault's consumer
    const bound = vault.reference(home, id);
    await Relationship.insert(
        database,
        {
            id: Identifier.create("relationship"),
            object: bound,
            relation: "consumer",
            subject: forge,
            createdAt: now,
            expiresAt: null,
        },
        home,
    );

    return bound;
}

/** Replicate the home's chain, the residency's accounts and the machine's rows, as its stack. */
export function homeReplicas(
    home: ResidencyHome,
    code: ResidencyCode,
    publisher: Publisher,
): Pick<ObjectStoreOptions, "chain" | "publisher"> & {
    readonly follows: readonly Follow[];
} {
    return {
        follows: [
            // replicate the home's own row and every account of the residency from the universe
            Follow.of(spaceShape, {
                keys: [{ key: { scope: Scope.universe.id, space: home.id } }],
            }),
            Follow.of(residentShape, {
                keys: [{ key: { scope: Scope.universe.id, residency: code } }],
            }),
        ],
        // store the chain above the home, born here
        chain: async () => ({ kind: "home", parent: home.scope }),
        publisher: () => publisher,
    };
}
