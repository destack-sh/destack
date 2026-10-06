import { accessRelationship, principal, Relationship } from "@destack/access";
import { and, eq, isNotNull } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import * as spaceObject from "@destack/space/object";
import { expect, refusal, single, test } from "@destack/test";
import { v7 } from "uuid";
import { secret, secretVersion, vault } from "../object/index.ts";
import { VaultFixture } from "../test/index.ts";
import { RemoteVault } from "./remote.ts";

test.each(TEST_DIALECTS)(
    "keep a credential for a user through the account service by reference, refuse other objects, and destroy it as its custodian on %s",
    async (dialect) => {
        // lend the member's vault role to the account service's installation
        await using fixture = await VaultFixture.open(dialect);
        const { client, database, spaceId, vaultId } = fixture;
        const member = fixture.caller.claims;
        const service = principal.installation.reference(
            fixture.accountId,
            schema.identifier("installation").parse(`installation-${v7()}`),
        );
        const lent = Relationship.encode(
            {
                id: `relationship-${v7()}`,
                object: spaceObject.space.reference(fixture.accountId, spaceId),
                role: fixture.roleId,
                subject: service,
                createdAt: Date.now(),
                expiresAt: null,
                conditions: { onBehalfOf: member.subject },
            },
            spaceId,
        );
        await database.insert(accessRelationship).values(lent);

        // reach the vault for the member through the service
        const delegated = new Authentication({
            ...member,
            delegates: [{ subject: service, authority: "lent" }],
        });
        const itself = new Authentication({ ...member, subject: service, subjects: [service] });
        const secrets = new RemoteVault(async (_spaceId, subject) => {
            fixture.caller = subject === undefined ? itself : delegated;

            return client;
        });

        // keep one secret with one version however often the credential is written
        const id = schema.identifier("secret").parse(`secret-${v7()}`);
        const written = {
            vault: vault.reference(spaceId, vaultId),
            id,
            name: "connection",
            value: "oauth-token",
        };
        const kept = await secrets.write(written, member.subject);
        expect(await secrets.write(written, member.subject)).toEqual(kept);
        expect([
            kept,
            (await database.select().from(secret.table)).map((row) => row.id),
            (await database.select().from(secretVersion.table)).map((row) => row.number),
            await secrets.read(kept, member.subject),
            await secrets.parent(kept),
        ]).toEqual([
            secret.reference(spaceId, id),
            [id],
            [1],
            "oauth-token",
            vault.reference(spaceId, vaultId),
        ]);

        // refuse a reference to anything but a secret of the vault service
        const space = spaceObject.space.reference(fixture.accountId, spaceId);
        expect([
            await refusal(secrets.read(space, member.subject)),
            await refusal(secrets.write({ ...written, vault: space }, member.subject)),
        ]).toEqual([
            ["BAD_REQUEST", `space ${spaceId} is no secret of the vault service`],
            ["BAD_REQUEST", `space ${spaceId} is no vault of the vault service`],
        ]);

        // relate the service writing as a delegate as the secret's custodian
        const custodians = await database
            .select({ subjectId: accessRelationship.subjectId })
            .from(accessRelationship)
            .where(
                and(
                    eq(accessRelationship.objectId, id),
                    eq(accessRelationship.relation, "custodian"),
                ),
            );
        expect(custodians).toEqual([{ subjectId: service.id }]);

        // destroy the secret as the service twice
        await database.delete(accessRelationship).where(eq(accessRelationship.id, lent.id));
        await secrets.destroy(kept);
        await secrets.destroy(kept);
        await secrets.destroy(secret.reference(spaceId, `secret-${v7()}`));
        const purged = single(await database.select().from(secret.table));
        expect([
            purged.purgedAt,
            await database
                .select({ id: secretVersion.table.id })
                .from(secretVersion.table)
                .where(isNotNull(secretVersion.table.ciphertext)),
        ]).toEqual([expect.any(Number), []]);
    },
);
