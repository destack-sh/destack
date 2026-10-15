import { TestDirectory } from "@destack/account/test";
import { IdentityKeystore, identityKey } from "@destack/account/identity";
import { expect, test } from "@destack/test";
import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import * as accountObject from "@destack/account/object";
import { account } from "@destack/account/object";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { Identifier, Clock } from "@destack/schema";
import { Scope } from "@destack/sync";
import { RequestId } from "@destack/runtime/request";
import * as object from "../../object/index.ts";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import {
    ids,
    keepSpace,
    openUniverse,
    placeSpace,
    StackFixture,
    subjectCaller,
} from "../../test/index.ts";

test("delete a space at its home once its installations are gone", async () => {
    // keep a space at home in the universe, placed here with its placement replicated, its identity started, and an installation in it
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const owner = principal.user.reference("universe", ids.owner);
    await new AccessFixture(database).bindOwner(account.reference("universe", ids.account), owner);
    const opened = await openUniverse();
    const { identities } = opened;
    const keystore = new IdentityKeystore(
        await LocalKeyring.open(new MemoryKeychain(), ids.cloud),
        identities,
        Clock.system,
    );
    const now = Date.now();
    const spaceId = Identifier.create("space");
    const placement = { id: spaceId, scope: ids.account, machine: ids.cloud, epoch: 1 };
    await TestDirectory.place(opened.universe, placement);
    await placeSpace(database, spaceId);
    await keepSpace(opened.universe, spaceId, "home");
    const home = await StackFixture.home(opened.universe);
    await fixture.replicate(home);
    const access = new AccessFixture(database);
    const reference = accountObject.space.reference(ids.account, spaceId);
    await access.replicateScope(reference);
    await access.bindOwner(reference, principal.space.reference(Scope.universe.id, spaceId));
    await keystore.generate(database, spaceId);
    await fixture.serve({ ...opened, keystore });
    await database.insert(object.installation.table).values({
        id: ids.notes,
        scope: spaceId,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });

    // request the deletion at home and keep the space while the installation it deletes remains
    await home.mutate(subjectCaller(owner, true), {
        id: RequestId.create(Date.now()),
        calls: [
            accountObject.space.call(
                "delete",
                accountObject.space.target({ scope: ids.account, id: spaceId }),
            ),
        ],
    });
    await fixture.replicate(home);
    const deleting = async () => {
        const [row] = await database
            .select({ deletedAt: object.installation.table.deletedAt })
            .from(object.installation.table)
            .where(eq(object.installation.table.id, ids.notes));

        return row?.deletedAt !== null;
    };
    await database.log.until(deleting, AbortSignal.timeout(5000));
    expect(await placed(opened.universe, spaceId)).toEqual([
        { id: spaceId, scope: ids.account, machine: ids.cloud, epoch: 1 },
    ]);

    // remove the space at home and its replica here once the installation is gone
    await database
        .delete(object.installation.table)
        .where(eq(object.installation.table.id, ids.notes));
    const spaces = (kept: DatabaseConnection) =>
        kept
            .select({ id: accountObject.space.table.id })
            .from(accountObject.space.table)
            .where(eq(accountObject.space.table.id, spaceId));
    await database.log.until(
        async () => (await spaces(database)).length === 0,
        AbortSignal.timeout(5000),
    );
    const keys = await database
        .select({ scope: identityKey.scope })
        .from(identityKey)
        .where(eq(identityKey.scope, spaceId));
    expect([await placed(opened.universe, spaceId), keys, await spaces(opened.universe)]).toEqual([
        [],
        [],
        [],
    ]);
});

/** Read a space's placement in the universe, none once withdrawn. */
function placed(universe: DatabaseConnection, spaceId: Identifier<"space">) {
    return accountObject.Placement.find(Snapshot.live(universe), [spaceId]);
}
