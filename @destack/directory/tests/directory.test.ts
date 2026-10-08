import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { keyPair } from "../src/test/index.ts";
import { schema } from "@destack/schema";
import { defineProcedure, defineService, ServiceMount } from "@destack/service";
import { ServiceError } from "@destack/service/error";
import { expect, onTestFinished, test } from "@destack/test";
import { directoryTables, DirectoryStore, Placement, placementTable } from "../src/index.ts";
import { IdentityError, IdentityOperation } from "@destack/identity";
import { RECOVERY_MILLISECONDS } from "../src/directory/store.ts";

/** A service a machine mounts, answering a space's placement. */
const placements = defineService("placements", {
    locate: defineProcedure({ authentication: "public", permission: null, audit: false })
        .route({ method: "GET", path: "/placements/{scope}" })
        .input(schema.object({ scope: schema.string() }))
        .output(Placement),
});

test.each(TEST_DIALECTS)(
    "locate a space, take it over only on the machine its move still targets at the next epoch, and withdraw it on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const placement = { id: "space-1", scope: "account-1", machine: "machine-a", epoch: 1 };

        // place a space on its machine twice
        await directory.place(placement);
        await directory.place(placement);
        expect([await directory.locate("space-1"), await directory.locate("account-1")]).toEqual([
            placement,
            undefined,
        ]);

        // refuse an arrival without a move
        const moved = { ...placement, machine: "machine-b", epoch: 2 };
        const unmoved = {
            code: "CONFLICT",
            message: "space-1 is neither placed on machine-b at epoch 2 nor moving there",
        };
        await expect(directory.place(moved)).rejects.toMatchObject(unmoved);
        await directory.move(placement, "machine-b");
        await directory.place(placement);
        await expect(directory.place(moved)).rejects.toMatchObject(unmoved);

        // take it over on the machine its move targets
        await directory.move(placement, "machine-b");
        await directory.place(moved);
        await expect(directory.place(placement)).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is neither placed on machine-a at epoch 1 nor moving there",
        });
        await expect(directory.place({ ...moved, machine: "machine-c" })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is neither placed on machine-c at epoch 2 nor moving there",
        });
        expect(await directory.locate("space-1")).toEqual(moved);

        // withdraw the space only at the epoch and machine serving it now
        await expect(directory.withdraw(placement)).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is no longer placed on machine-a at epoch 1",
        });
        await directory.withdraw(moved);
        expect(await directory.locate("space-1")).toBeUndefined();
    },
);

test.each(TEST_DIALECTS)(
    "mark a space moving to a machine until it takes it, and serve machines' endpoints on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const placement = { id: "space-1", scope: "account-1", machine: "machine-a", epoch: 1 };
        await directory.place(placement);

        // mark the space moving to the target
        const target = async () => {
            const [placed] = await storage.database
                .select({ target: placementTable.target })
                .from(placementTable);
            if (placed === undefined) {
                throw new TypeError("placement table has no row");
            }

            return placed.target;
        };
        const seen = [await target()];
        await directory.move(placement, "machine-b");
        seen.push(await target());
        await directory.place({ ...placement, machine: "machine-b", epoch: 2 });
        seen.push(await target());

        // refuse moving a space its machine no longer serves at the epoch
        await expect(directory.move(placement, "machine-c")).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is no longer placed on machine-a at epoch 1",
        });

        // publish a machine's endpoint
        await directory.publish(
            "machine-b",
            "account-1",
            "https://machine-b.test/",
            "publication-1",
        );
        expect([
            seen,
            await directory.endpoint("machine-b"),
            await directory.endpoint("machine-z"),
        ]).toEqual([
            [null, "machine-b", null],
            { machine: "machine-b", scope: "account-1", url: "https://machine-b.test/" },
            undefined,
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "withdraw an endpoint only under the publication holding it, keeping a later tunnel's on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);

        // publish through a first tunnel, then through a second as the machine reconnects
        await directory.publish("machine-a", "account-1", "https://relay-1.test/", "tunnel-1");
        await directory.publish("machine-a", "account-1", "https://relay-2.test/", "tunnel-2");

        // keep the second tunnel's endpoint as the first one's withdrawal arrives late
        await directory.unpublish("machine-a", "tunnel-1");
        const kept = await directory.endpoint("machine-a");
        await directory.unpublish("machine-a", "tunnel-2");
        expect([kept, await directory.endpoint("machine-a")]).toEqual([
            { machine: "machine-a", scope: "account-1", url: "https://relay-2.test/" },
            undefined,
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "list the placements of an account's spaces in identity order on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const placed = [
            { id: "space-2", scope: "account-1", machine: "machine-a", epoch: 1 },
            { id: "space-1", scope: "account-1", machine: "machine-b", epoch: 1 },
            { id: "space-3", scope: "account-2", machine: "machine-a", epoch: 1 },
        ];
        for (const placement of placed) {
            await directory.place(placement);
        }

        expect([await directory.list("account-1"), await directory.list("account-9")]).toEqual([
            [placed[1], placed[0]],
            [],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "reserve, confirm and replace claims, refusing a name another object holds, resolving active names alone, and release the rest on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const index = "package-1/place/name";
        const claim = (key: string, objectId: string) => ({
            index,
            key,
            packageId: "package-1",
            objectId,
            scope: "account-1",
        });

        // reserve a name and confirm it once the write committed
        expect([
            await directory.claim([claim("notes", "space-1")], "request-1"),
            await directory.claim([claim("notes", "space-2")], "request-2"),
        ]).toEqual([[], [claim("notes", "space-2")]]);
        const reserved = await directory.owner(index, "notes");
        const owned = (key: string, state: "active" | "held") => ({
            indexes: [index],
            objectId: "space-1",
            claims: [claim(key, "space-1")],
            state,
        });
        await directory.confirm("request-1", [owned("notes", "active")]);
        const active = await directory.owner(index, "notes");

        // hold the name while its object sits in the trash, still refusing it to another object
        await directory.replace(owned("notes", "held"), "request-3");
        const trashed = await directory.owner(index, "notes");
        const taken = await directory.replace(
            {
                indexes: [index],
                objectId: "space-2",
                claims: [claim("notes", "space-2")],
                state: "active",
            },
            "request-4",
        );

        // restore and rename the object
        await directory.replace(owned("notes", "active"), "request-5");
        const restored = await directory.owner(index, "notes");
        await directory.replace(owned("archive", "active"), "request-6");
        expect([
            reserved,
            active,
            trashed,
            taken,
            restored,
            await directory.owner(index, "notes"),
            await directory.owner(index, "archive"),
        ]).toEqual([
            undefined,
            { objectId: "space-1", scope: "account-1" },
            undefined,
            [claim("notes", "space-2")],
            { objectId: "space-1", scope: "account-1" },
            undefined,
            { objectId: "space-1", scope: "account-1" },
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "send a service's requests to a machine's endpoint and resend them once to the machine a scope moved to on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        await directory.publish(
            "machine-a",
            "account-1",
            "https://machine-a.test/",
            "publication-1",
        );
        await directory.publish(
            "machine-b",
            "account-1",
            "https://machine-b.test",
            "publication-1",
        );

        // answer misdirected at the first machine and the placement at the second
        const mount = ServiceMount.path(placements.package.id);
        const placement = { id: "space-1", scope: "account-1", machine: "machine-b", epoch: 2 };
        const sent: string[] = [];
        const client = directory.machineClient(placements, "machine-a", async (request) => {
            sent.push(`${request.method} ${request.url}`);
            const moved = {
                defined: true,
                code: "MISDIRECTED_REQUEST",
                status: 421,
                message: "space-1 moves to machine-b",
                data: { scope: "space-1", machine: "machine-b" },
            };

            return request.url.startsWith("https://machine-a.test")
                ? Response.json(moved, { status: 421 })
                : Response.json(placement);
        });

        // follow the move once and stay at the new machine
        expect([
            await client.locate({ scope: "space-1" }),
            await client.locate({ scope: "space-1" }),
            sent,
        ]).toEqual([
            placement,
            placement,
            [
                `GET https://machine-a.test${mount}/placements/space-1`,
                `GET https://machine-b.test${mount}/placements/space-1`,
                `GET https://machine-b.test${mount}/placements/space-1`,
            ],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "send a service's requests below the mount of its space and of an installation of its space at the machine serving the space on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        await directory.publish(
            "machine-a",
            "account-1",
            "https://machine-a.test/",
            "publication-1",
        );
        await directory.place({
            id: "space-1",
            scope: "account-1",
            machine: "machine-a",
            epoch: 1,
        });

        // call the service of the space and of its finance installation
        const sent: string[] = [];
        const fetch = async (request: Request) => {
            sent.push(`${request.method} ${request.url}`);

            return Response.json({
                id: "space-1",
                scope: "account-1",
                machine: "machine-a",
                epoch: 1,
            });
        };
        await directory.spaceClient(placements, "space-1", fetch).locate({ scope: "space-1" });
        await directory
            .installationClient(placements, "space-1", "finance", fetch)
            .locate({ scope: "space-1" });

        // mount the service below the space's path and below the installation's
        const mount = ServiceMount.path(placements.package.id);
        expect(sent).toEqual([
            `GET https://machine-a.test/spaces/space-1${mount}/placements/space-1`,
            `GET https://machine-a.test/spaces/space-1/installations/finance${mount}/placements/space-1`,
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse a malformed identity operation, start a space's identity from its machine, chain operations by rotation keys, and let a higher-priority key nullify a lower one's within the recovery window on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        let now = Date.now();
        const directory = new DirectoryStore(storage.database, { clock: () => now });
        const placement = { id: "space-1", scope: "account-1", machine: "machine-a", epoch: 1 };
        await directory.place(placement);
        const signing = await keyPair();
        const machine = await keyPair();
        const owner = await keyPair();
        const intruder = await keyPair();

        // refuse a malformed operation
        await expect(directory.apply("not.an.operation", placement)).rejects.toEqual(
            new IdentityError("INVALID_OPERATION", "the operation is no identity operation"),
        );

        // start the identity only from the serving machine
        const first = await IdentityOperation.sign(
            {
                subject: "space-1",
                previous: null,
                signingKeys: [signing.key],
                rotationKeys: [machine.key],
            },
            machine.privateKey,
        );
        const unserved = new ServiceError("FORBIDDEN", {
            message: "only the machine serving space-1 starts its identity",
        });
        await expect(directory.apply(first)).rejects.toEqual(unserved);
        await expect(
            directory.apply(first, { ...placement, machine: "machine-b" }),
        ).rejects.toEqual(unserved);
        await directory.apply(first, placement);
        expect(await directory.identity("space-1")).toEqual({
            signingKeys: [signing.key],
            rotationKeys: [machine.key],
            digest: await IdentityOperation.digest(first),
        });

        // add the owner's key above the machine's
        const previous = await IdentityOperation.digest(first);
        const owned = { subject: "space-1", previous, signingKeys: [signing.key] };
        const rotationKeys = [owner.key, machine.key];
        await expect(
            directory.apply(
                await IdentityOperation.sign({ ...owned, rotationKeys }, intruder.privateKey),
            ),
        ).rejects.toEqual(
            new ServiceError("UNAUTHORIZED", {
                message: "no rotation key of the identity signed the operation",
            }),
        );
        const second = await IdentityOperation.sign({ ...owned, rotationKeys }, machine.privateKey);
        await directory.apply(second);

        // let the machine's key replace the signing key
        const unrecoverable = new ServiceError("CONFLICT", {
            message: "the operation cannot nullify the later operations of space-1",
        });
        const after = {
            subject: "space-1",
            previous: await IdentityOperation.digest(second),
            rotationKeys,
        };
        const stolen = await IdentityOperation.sign(
            { ...after, signingKeys: [intruder.key] },
            machine.privateKey,
        );
        await directory.apply(stolen);
        expect((await directory.identity("space-1"))?.signingKeys).toEqual([intruder.key]);
        const recovered = await IdentityOperation.sign(
            { ...after, signingKeys: [signing.key] },
            owner.privateKey,
        );
        await directory.apply(recovered);
        await expect(
            directory.apply(
                await IdentityOperation.sign(
                    { ...after, signingKeys: [intruder.key] },
                    machine.privateKey,
                ),
            ),
        ).rejects.toEqual(unrecoverable);
        expect([
            await directory.identity("space-1"),
            await directory.operations("space-1"),
        ]).toEqual([
            {
                signingKeys: [signing.key],
                rotationKeys,
                digest: await IdentityOperation.digest(recovered),
            },
            [first, second, stolen, recovered],
        ]);

        // refuse the owner's key nullifying the machine's next operation once the recovery window passed
        const settled = {
            subject: "space-1",
            previous: await IdentityOperation.digest(recovered),
            rotationKeys,
        };
        await directory.apply(
            await IdentityOperation.sign(
                { ...settled, signingKeys: [intruder.key] },
                machine.privateKey,
            ),
        );
        now += RECOVERY_MILLISECONDS + 1000;
        await expect(
            directory.apply(
                await IdentityOperation.sign(
                    { ...settled, signingKeys: [signing.key] },
                    owner.privateKey,
                ),
            ),
        ).rejects.toEqual(unrecoverable);
    },
);
