import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { defineProcedure, defineService, ServiceMount } from "@destack/service";
import { expect, onTestFinished, test } from "@destack/test";
import { directoryTables, DirectoryStore, Zone, zoneTable } from "../src/index.ts";

/** A service a cell mounts, answering a zone's location. */
const zones = defineService("zones", {
    locate: defineProcedure({ authentication: "public", permission: null, audit: false })
        .route({ method: "GET", path: "/zones/{scope}" })
        .input(schema.object({ scope: schema.string() }))
        .output(Zone),
});

test.each(TEST_DIALECTS)(
    "locate a zone, move it only to a later epoch, and withdraw it on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const zone = { id: "space-1", scope: "account-1", cell: "host-1", epoch: 1 };

        // place a zone in its host, again without change, and find nothing for other scopes
        await directory.place(zone);
        await directory.place(zone);
        expect([await directory.locate("space-1"), await directory.locate("account-1")]).toEqual([
            zone,
            undefined,
        ]);

        // move the zone to a region at the next epoch, and refuse the stale host and a rival at that epoch
        const moved = { ...zone, cell: "region-1", epoch: 2 };
        await directory.place(moved);
        const refused = {
            code: "CONFLICT",
            message: "space-1 is placed at a later epoch or in another cell",
        };
        await expect(directory.place(zone)).rejects.toMatchObject(refused);
        await expect(directory.place({ ...moved, cell: "region-2" })).rejects.toMatchObject(
            refused,
        );
        expect(await directory.locate("space-1")).toEqual(moved);

        // withdraw the zone only at the epoch and cell serving it now
        await expect(directory.withdraw(zone)).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is no longer placed in host-1 at epoch 1",
        });
        await directory.withdraw(moved);
        expect(await directory.locate("space-1")).toBeUndefined();
    },
);

test.each(TEST_DIALECTS)(
    "mark a zone moving to a cell until it takes it, and serve cells' endpoints on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const zone = { id: "space-1", scope: "account-1", cell: "host-1", epoch: 1 };
        await directory.place(zone);

        // mark the zone moving to the target, and clear the mark once the target takes it
        const target = async () => {
            const [zoneRow] = await storage.database
                .select({ target: zoneTable.target })
                .from(zoneTable);
            if (zoneRow === undefined) {
                throw new TypeError("zone table has no row");
            }

            return zoneRow.target;
        };
        const seen = [await target()];
        await directory.move(zone, "host-2");
        seen.push(await target());
        await directory.place({ ...zone, cell: "host-2", epoch: 2 });
        seen.push(await target());

        // refuse moving a zone its cell no longer serves at the epoch
        await expect(directory.move(zone, "host-3")).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is no longer placed in host-1 at epoch 1",
        });

        // publish a cell's endpoint, and find none of an unknown cell
        await directory.publish("host-2", "account-1", "https://host-2.test/");
        expect([seen, await directory.cell("host-2"), await directory.cell("host-9")]).toEqual([
            [null, "host-2", null],
            { id: "host-2", scope: "account-1", endpoint: "https://host-2.test/" },
            undefined,
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "list the zones a scope contains in identity order on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const placed = [
            { id: "space-2", scope: "account-1", cell: "host-1", epoch: 1 },
            { id: "space-1", scope: "account-1", cell: "region-1", epoch: 1 },
            { id: "space-3", scope: "account-2", cell: "host-1", epoch: 1 },
        ];
        for (const zone of placed) {
            await directory.place(zone);
        }

        expect([await directory.list("account-1"), await directory.list("account-9")]).toEqual([
            [placed[1], placed[0]],
            [],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "reserve, confirm and replace claims, refusing a name another object owns, and release the rest on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const index = "package-1/place/name";
        const claim = (key: string, objectId: string) => ({
            index,
            key,
            objectId,
            scope: "account-1",
        });

        // reserve a name, return it as taken to another object, and confirm it once the write committed
        expect([
            await directory.claim([claim("notes", "space-1")], "request-1"),
            await directory.claim([claim("notes", "space-2")], "request-2"),
        ]).toEqual([[], [claim("notes", "space-2")]]);
        const reserved = await directory.owner(index, "notes");
        await directory.confirm("request-1", [
            { indexes: [index], objectId: "space-1", claims: [claim("notes", "space-1")] },
        ]);

        // refuse another object's replacement, then rename the object, releasing the old name
        const taken = await directory.replace(
            { indexes: [index], objectId: "space-2", claims: [claim("notes", "space-2")] },
            "request-3",
        );
        await directory.replace(
            { indexes: [index], objectId: "space-1", claims: [claim("archive", "space-1")] },
            "request-4",
        );
        expect([
            reserved,
            taken,
            await directory.owner(index, "notes"),
            await directory.owner(index, "archive"),
        ]).toEqual([
            undefined,
            [claim("notes", "space-2")],
            undefined,
            { objectId: "space-1", scope: "account-1" },
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "send a service's requests to a cell's endpoint and resend them once to the cell a scope moved to on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        await directory.publish("host-1", "account-1", "https://host-1.test/");
        await directory.publish("host-2", "account-1", "https://host-2.test");

        // answer MOVED at the first cell, and the zone at the second, where each mounts the service
        const mount = ServiceMount.path(zones.package.id);
        const zone = { id: "space-1", scope: "account-1", cell: "host-2", epoch: 2 };
        const sent: string[] = [];
        const client = directory.client(zones, "host-1", async (request) => {
            sent.push(`${request.method} ${request.url}`);
            const moved = {
                defined: true,
                code: "MOVED",
                status: 421,
                message: "space-1 moves to host-2",
                data: { scope: "space-1", cell: "host-2" },
            };

            return request.url.startsWith("https://host-1.test")
                ? Response.json(moved, { status: 421 })
                : Response.json(zone);
        });

        // follow the move once, then stay at the new cell
        expect([
            await client.locate({ scope: "space-1" }),
            await client.locate({ scope: "space-1" }),
            sent,
        ]).toEqual([
            zone,
            zone,
            [
                `GET https://host-1.test${mount}/zones/space-1`,
                `GET https://host-2.test${mount}/zones/space-1`,
                `GET https://host-2.test${mount}/zones/space-1`,
            ],
        ]);
    },
);
