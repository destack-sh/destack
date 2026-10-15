import { STACKS } from "@destack/account/test";
import { expect, onTestFinished, test } from "@destack/test";
import {
    AccessContext,
    Authentication,
    Authorization,
    Authorizer,
    Caller,
    principal,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { Account, account, user, space } from "@destack/account/object";
import { type DatabaseConnection, TABLE } from "@destack/db";
import { ResourceContext } from "@destack/resource/context";
import { present, schema, Clock } from "@destack/schema";
import { Health } from "@destack/runtime/health";
import { Server } from "@destack/runtime/server";
import { CallError } from "@destack/runtime/error";
import {
    type ObjectTypeReference,
    type Page,
    replica,
    replicaPosition,
    Subject,
} from "@destack/sync";
import { createClient } from "@destack/runtime/client";
import stackPackage from "../src/package.ts";
import * as accountObject from "@destack/account/object";
import { installation } from "../src/object/index.ts";
import { serveSpace } from "../src/server/index.ts";
import { ids, StackFixture } from "../src/test/index.ts";

/** The space's notes installation. */
const notes = principal.installation.reference(ids.space, ids.notes);

/** An installation of another space. */
const other = principal.installation.reference(
    "space-01996ab0-0000-7000-8000-000000000009",
    "installation-01996ab0-0000-7000-8000-000000000006",
);

/** The person writing through the replicas. */
const alice = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000008");

test("relay the access deciding a space's objects to each of its installations alone", async () => {
    // keep and serve the space below its account
    const { snapshot } = await serveInstallation();

    // relay the space's own access and the account's replica to the installation
    expect(located(await snapshot(notes, ids.space))).toEqual([
        ["destack__access__relationship", ids.space],
        ["destack__sync__scope", ids.space],
    ]);
    expect(located(await snapshot(notes, ids.account))).toEqual([
        ["destack__sync__replica_position", ids.account],
        ["destack__sync__scope", ids.account],
    ]);

    // refuse another space's installation and a scope outside the space's chain
    await expect(snapshot(other, ids.space)).rejects.toEqual(
        new CallError("FORBIDDEN", {
            defined: true,
            message: `only ${ids.space} and its contents can follow the replicas kept for it`,
        }),
    );
    await expect(snapshot(notes, "account-elsewhere")).rejects.toEqual(
        new CallError("NOT_FOUND", {
            defined: true,
            message: `account-elsewhere does not contain ${ids.space}`,
        }),
    );
});

test("relay the user enclosing a space to the space's installation, concealing its guarded fields", async () => {
    // keep and serve the space below an account of alice's user scope with her replicated user row
    const { database, snapshot } = await serveInstallation();
    const now = Date.now();
    const key = { name: Authorizer.chainReplica(ids.space), scope: alice };
    await database.insert(replica).values({ ...key, tables: [] });
    await database
        .insert(replicaPosition)
        .values({ ...key, epoch: "epoch-global", sequence: 812, confirmedAt: now });
    await database.insert(user.table).values({
        id: alice,
        scope: "universe",
        name: "alice",
        email: "alice@example.com",
        timeZone: "Europe/Vienna",
        locale: "de-AT",
        homeId: Account.home(Account.personal(alice)),
        createdAt: now,
        updatedAt: now,
    });

    // relay her user row to the installation inside her scope
    const changes = await snapshot(notes, alice, [user.typeReference]);
    expect(
        changes.flatMap((change) =>
            change.table === user.table[TABLE].sqlName
                ? [[change.row["id"], change.concealed]]
                : [],
        ),
    ).toEqual([
        [alice, ["email", "emailVerified", "login", "timeZone", "twoFactorEnabled", "suspendedAt"]],
    ]);
});

/** Keep a space with one installation below alice's account's replicas, served to installations. */
async function serveInstallation() {
    // keep the space with one installation in the machine's database below the replicated scopes of alice and her account
    const machine = await StackFixture.empty();
    const { database } = machine;
    await createSpace(database);

    // serve the space as the request's installation at once, its replicas written by the fixture
    const options = await machine.options();
    const [served] = serveSpace(options).packages;
    const { ready: _ready, ...implementation } = present(served, "the stack package");
    const server = Server.start({
        clock: Clock.system,
        ...implementation,
        resources: new ResourceContext(),
        health: new Health("space"),
        authenticate: async (request) => authenticate(request),
        authorizeMachine: async () => {},
        drainTimeout: 1000,
    });
    onTestFinished(() => server.close());
    const client = (as: Subject) =>
        createClient(stackPackage, {
            url: "http://space.test",
            headers: { "x-installation": Subject.key(as) },
            fetch: (request) => server.fetch(request),
        });

    // read the snapshot of one scope's replica as an installation with each row's scope
    const snapshot = async (
        as: Subject,
        scope: string,
        replicated: readonly ObjectTypeReference[] = [],
    ) => {
        const pages = await client(as).replica.stream({
            name: Authorizer.chainReplica(ids.space),
            scope,
            below: ids.space,
            publication: "chain",
            parameters: {
                local: [],
                replicated: [...replicated],
                via: [],
                between: [],
                types: [],
            },
        });
        const changes: Page["changes"][number][] = [];
        for await (const page of pages) {
            changes.push(...page.changes);
            if (page.complete) {
                break;
            }
        }

        return changes;
    };

    return { database, snapshot };
}

/** Record the space with the notes installation below alice and her account's replicated scopes. */
async function createSpace(database: DatabaseConnection): Promise<void> {
    // replicate the scopes of alice and her account with the space's chain replica
    const now = Date.now();
    const access = new AccessFixture(database);
    await access.replicateScope(user.reference("universe", alice));
    await access.replicateScope(account.reference(alice, ids.account));
    const key = { name: Authorizer.chainReplica(ids.space), scope: ids.account };
    await database.insert(replica).values({ ...key, tables: [] });
    await database
        .insert(replicaPosition)
        .values({ ...key, epoch: "epoch-global", sequence: 812, confirmedAt: now });

    // record the space owned by owner
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        stack: STACKS.space,
        createdAt: now,
        updatedAt: now,
    });
    const owner = principal.user.reference("universe", ids.owner);
    await Authorization.run(
        new Authorizer([accountObject.space.policy], [accountObject.space.mapping]),
        database,
        () => AccessContext.of(Caller.of(owner), now),
        (authorization) =>
            authorization.create(accountObject.space.reference(ids.account, ids.space), {
                owner,
            }),
    );

    // install notes
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });
}

/** Authenticate a request as the installation its x-installation header names. */
function authenticate(request: Request): Authentication {
    const subject = Subject.read(
        present(request.headers.get("x-installation"), "the x-installation header"),
    );

    return {
        caller: Caller.of(subject),
        credential: {
            kind: "fixture",
            id: "fixture-1",
            audience: stackPackage.id,
            verifiedAt: Date.now(),
            expiresAt: Date.now() + 60_000,
        },
    };
}

/** List each change's table and scope in order. */
function located(changes: Page["changes"]): [string, string][] {
    return changes
        .map((change): [string, string] => [
            change.table,
            schema.string().parse(change.row["scope"]),
        ])
        .toSorted((left, right) => left.join().localeCompare(right.join()));
}
