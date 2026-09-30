import { spaceTables } from "../src/stack/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { Authorization, Authorizer, COPY_NAME, principal } from "@destack/access";
import { copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import { eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { identifier, schema } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { replica, type QueryPage } from "@destack/sync";
import { connect } from "../src/client/index.ts";
import * as object from "../src/object/index.ts";
import { implementService } from "../src/server/index.ts";
import { installation, space } from "../src/object/index.ts";
import { spaceOptions } from "./fixture/space.ts";

/** The identities the scenario names. */
const ids = {
    account: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    region: identifier("region").parse("region-01996ab0-0000-7000-8000-000000000003"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    notes: identifier("installation").parse("installation-01996ab0-0000-7000-8000-000000000005"),
    other: identifier("installation").parse("installation-01996ab0-0000-7000-8000-000000000006"),
    host: identifier("host").parse("host-01996ab0-0000-7000-8000-000000000007"),
};

test("relay the access deciding a space's objects to each of its installations alone", async () => {
    // keep the space in the regional database, below its account's copied scope, with one installation
    const test = await TestDatabase.create("sqlite", spaceTables, { isMigrated: true });
    onTestFinished(() => test.close());
    const database = test.database;
    const now = Date.now();
    await copyScope(database, account.reference("universe", ids.account));
    await database.insert(replica).values({
        name: COPY_NAME,
        scope: ids.account,
        epoch: "epoch-global",
        sequence: 812,
        confirmedAt: now,
    });
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    const owner = principal.user.reference("universe", "owner");
    await new Authorization(
        new Authorizer([object.space.policy], [object.space.mapping]),
        database,
        () => ({ subjects: [owner], now, attributes: {} }),
    ).create(object.space.reference(ids.account, ids.space), { owner });
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        alias: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    } as never);

    // serve the space as the request's installation with space names in a global key index
    await using server = Server.start({
        ...implementService(await spaceOptions(database)),
        audience: ids.package,
        resources: new ResourceContext(),
        health: new Health("space"),
        authenticate: async (request) => {
            const subject = principal.installation.reference(
                ids.space,
                request.headers.get("x-installation")!,
            );

            return new Caller({
                credential: { kind: "fixture", id: "fixture-1" },
                audience: ids.package,
                subject,
                subjects: [subject],
                verifiedAt: Date.now(),
                expiresAt: Date.now() + 60_000,
            });
        },
        authorizeHost: async () => {},
        drainTimeout: 1000,
    });
    const client = (as: string) =>
        connect({
            url: "http://space.test",
            headers: { "x-installation": as },
            fetch: (request) => server.fetch(request),
        });

    // read the snapshot of one scope's copy as an installation with each row's scope
    const snapshot = async (as: string, scope: string) => {
        const pages = await client(as).replica.stream({
            name: COPY_NAME,
            scope,
            below: ids.space,
            access: true,
            held: [],
            copied: [],
            rows: [],
        });
        const rows: [string, string][] = [];
        for await (const page of pages as AsyncIterable<QueryPage>) {
            rows.push(
                ...page.changes.map((change): [string, string] => [
                    change.table,
                    schema.string().parse(change.row.scope),
                ]),
            );
            if (page.complete) {
                break;
            }
        }

        return rows.sort((left, right) => left.join().localeCompare(right.join()));
    };

    // relay the space's own access and the account's copy to the installation
    expect(await snapshot(ids.notes, ids.space)).toEqual([
        ["destack__access__relationship", ids.space],
        ["destack__access__role", ids.space],
        ["destack__sync__scope", ids.space],
    ]);
    expect(await snapshot(ids.notes, ids.account)).toEqual([
        ["destack__sync__replica", ids.account],
        ["destack__sync__scope", ids.account],
    ]);

    // refuse relaying a copy without a position
    await database
        .update(replica)
        .set({ epoch: null, sequence: null })
        .where(eq(replica.scope, ids.account));
    await expect(snapshot(ids.notes, ids.account)).rejects.toEqual(
        new ServiceError("SERVICE_UNAVAILABLE", {
            message: `copy of ${ids.account} holds no position yet`,
        }),
    );

    // refuse another installation, and a scope outside the space's chain
    await expect(snapshot(ids.other, ids.space)).rejects.toEqual(
        new ServiceError("FORBIDDEN", {
            defined: true,
            message: "permission denied: replicate",
        }),
    );
    await expect(snapshot(ids.notes, "account-elsewhere")).rejects.toEqual(
        new ServiceError("NOT_FOUND", {
            defined: true,
            message: `account-elsewhere does not contain ${ids.space}`,
        }),
    );
});
