import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot, asc, type DatabaseConnection } from "@destack/db";
import { Feed, Replica, type Subscription } from "@destack/sync";
import { aligned } from "@destack/schema";
import {
    accessTables,
    accessRelationship,
    CHAIN_SHAPE,
    Authorization,
    Authorizer,
    principal,
    type AccessContext,
    type TableMapping,
} from "../index.ts";
import {
    account,
    accountTable,
    item,
    mappings,
    node,
    policies,
    policyTable,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";

/** The owner of the account and the space. */
const alice: AccessContext = {
    subjects: [principal.user.reference("universe", "alice")],
    now: 1000,
    attributes: {},
};

/** A member of the account who has roles through the membership. */
const carol: AccessContext = {
    subjects: [
        principal.user.reference("universe", "carol"),
        { ...account.reference("universe", "account-1"), relation: "member" },
    ],
    now: 1000,
    attributes: {},
};

/** List the fixture's mappings with one more. */
function including(entry: TableMapping): TableMapping[] {
    return [...mappings, entry];
}

/** Build the copy of a scope's chain row an authorizer's database follows, with its subscription. */
function chainOf(authorizer: Authorizer, scope: string) {
    const subscription = authorizer.chainShape.subscription({
        name: Authorizer.chainCopy(scope),
        scope,
        below: scope,
        parameters: { local: [...authorizer.local], copied: [...authorizer.copied] },
    });

    return { replica: authorizer.chainShape.replica(subscription), subscription };
}

test("relay an account's access through the space's database into an app's, and follow its revocation", async () => {
    // open the account service's database with accounts, the cell's with spaces, and an app's with notes
    const global = await openFixture();
    const regional = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([global.close(), regional.close(), app.close()]);
    });
    const accounts = new Authorizer(
        policies,
        including({
            policy: account,
            table: accountTable,
            id: "id",
            scope: "scope",
            attributes: {},
            relations: {},
        }),
    );
    const spaces = new Authorizer(
        policies,
        including({
            policy: space,
            table: spaceTable,
            id: "id",
            scope: "account",
            attributes: {},
            relations: {},
        }),
    );
    const notes = app.authorizer;

    // create the account owned by alice, with a reader role bound to its members
    await global.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    const object = account.reference("universe", "account-1");
    const owner = new Authorization(accounts, global.database, () => alice);
    await owner.create(object, { owner: aligned(alice.subjects, 0) });
    const reader = await owner.createRole(object, {
        name: "reader",
        description: "Read every note",
        permissions: [node.permission("read")],
    });
    const binding = await owner.grant({
        object,
        role: reader.id,
        subject: aligned(carol.subjects, 1),
    });

    // copy the account into the cell's database, then create the space there
    const accountFeed = new Feed(global.database, [...accessTables, policyTable]);
    await copy(regional.database, chainOf(spaces, "account-1"), accountFeed);
    await regional.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await new Authorization(spaces, regional.database, () => alice).create(
        space.reference("account-1", "personal"),
        { owner: aligned(alice.subjects, 0) },
    );

    // relay the space and the account from the cell's database into the app's, where carol reads every note
    const spaceFeed = new Feed(regional.database, [...accessTables, policyTable]);
    const relay = async () => {
        await copy(regional.database, chainOf(spaces, "account-1"), accountFeed);
        await copy(app.database, chainOf(notes, "account-1"), spaceFeed);
        await copy(app.database, chainOf(notes, "personal"), spaceFeed);
    };
    await relay();
    const readable = async (context: AccessContext) =>
        (
            await app.database
                .select({ id: item.id })
                .from(item)
                .where(
                    notes.where(
                        node.permission("read"),
                        await notes.resolve(Snapshot.live(app.database), "personal", context),
                    ),
                )
                .orderBy(asc(item.id))
        ).map((row) => row.id);
    expect(await readable(carol)).toEqual(["a", "b", "c"]);

    // revoke the binding in the account's database and refuse deciding on unconfirmed copies
    await owner.revoke(object, binding.id);
    const revoked = await global.database.log.position();
    const impatient = new Authorizer(policies, mappings, { lag: 50 });
    await new Promise((resolve) => {
        setTimeout(resolve, 60);
    });
    await expect(
        impatient.resolve(Snapshot.live(app.database), "personal", carol),
    ).rejects.toMatchObject({
        code: "STALE",
    });

    // relay the revocation: carol reads nothing, and the app's copy reflects the account's own position
    await relay();
    expect(await readable(carol)).toEqual([]);
    const origins = await Replica.origins(app.database, CHAIN_SHAPE, ["account-1"]);
    expect(origins.get("account-1")?.position).toEqual(revoked);

    // refuse writing the account's access in the app's copy
    await expect(
        new Authorization(notes, app.database, () => alice).grant({
            object,
            relation: "member",
            subject: principal.user.reference("universe", "dave"),
        }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "access of account account-1 is written in the database keeping it",
    });

    // keep the app's own relationships, about its notes, through a fresh snapshot of the space's copy
    await copy(app.database, chainOf(notes, "personal"), spaceFeed, "snapshot");
    const own = await app.database
        .select({ type: accessRelationship.type, objectId: accessRelationship.objectId })
        .from(accessRelationship)
        .orderBy(asc(accessRelationship.type));
    expect(own.filter((row) => row.type === node.name)).toEqual([{ type: "node", objectId: "b" }]);
});

/** Apply a source's feed to a copy until it reaches the source's head. */
async function copy(
    database: DatabaseConnection,
    chain: { readonly replica: Replica; readonly subscription: Subscription },
    feed: Feed,
    from: "position" | "snapshot" = "position",
): Promise<void> {
    // read the pages up to the source's head
    const { replica, subscription } = chain;
    await replica.register(database, subscription);
    const head = await feed.database.log.position();
    const after = from === "snapshot" ? undefined : await replica.position(database);
    const pages = [];
    for await (const page of feed.subscribe(replica.queries, after, AbortSignal.timeout(5000))) {
        pages.push(page);
        if (page.complete && page.position.sequence >= head.sequence) {
            break;
        }
    }

    // apply them in order
    await Array.fromAsync(replica.apply(database, pages));
}
