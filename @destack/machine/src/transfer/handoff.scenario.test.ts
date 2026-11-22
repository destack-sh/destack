import { Recipient } from "@destack/identity";
import { ResourceId } from "@destack/resource";
import { type Digest, present, schema } from "@destack/schema";
import { eq } from "@destack/db";
import { MAIN_DATABASE } from "@destack/stack/declare";
import { RequestId } from "@destack/sync";
import { createClient } from "@destack/object/client";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import * as object from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { TestBuildStore } from "@destack/build/test";
import { BuildImage } from "@destack/package/manifest";
import { fixtureBuild, ids, TestSpace } from "../test/index.ts";

test("serve a moving space's target only the blobs the space's rows reference and its machine stores", async () => {
    // enroll the target as a machine of the space's account and let the owner move the space there
    const fixture = await TestSpace.open();
    const now = fixture.machine.clock.now();
    const target = await fixture.universe.enroll(fixture.owner, ids.account, "http://target.test/");
    await fixture.serve();
    await fixture.simulation.idle();
    const owner = fixture.as(fixture.owner.subject).stepUp().endpoint(stackPackage);
    await createClient(stackPackage, owner, fixture.machine.environment).transfer.create({
        spaceId: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        targetId: target.id,
    });

    // write a blob a snapshot of the space reads, and a stray no row references once its write's lease ends
    const { blobs } = await fixture.space;
    const write = (text: string) =>
        blobs.write(
            (async function* () {
                yield new TextEncoder().encode(text);
            })(),
        );
    const [taken, other] = [await write("taken"), await write("other")];
    await fixture.database.transaction(
        async (transaction) =>
            await transaction.insert(object.snapshot.table).values({
                id: schema
                    .identifier("snapshot")
                    .parse(object.snapshot.generateId(fixture.environment)),
                spaceId: ids.space,
                resourceId: ResourceId.parse("database-01996ab0-0000-7000-8000-0000000000f1"),
                recipient: (await Recipient.generate(fixture.environment)).key,
                digest: taken,
                takenAt: now,
                blobs: [taken],
                createdAt: now,
                updatedAt: now,
            }),
    );

    // list and read the snapshot's blob as the target, and refuse the other blob and another machine
    const read = async (caller: string, digest: Digest) => {
        const chunks: Uint8Array[] = [];
        for await (const blob of fixture.handoff(fixture.as(caller)).batch(ids.space, [digest])) {
            for await (const chunk of blob.body) {
                chunks.push(chunk);
            }
        }

        return new TextDecoder().decode(Buffer.concat(chunks));
    };
    await fixture.simulation.jump(
        fixture.simulation.clock.now() + (await fixture.space).blobs.leaseMilliseconds + 1,
    );
    const listed = await fixture.handoff(fixture.as(target.id)).blobs(ids.space);
    const [main] = await fixture.database
        .select({ states: object.database.table.states })
        .from(object.database.table)
        .where(eq(object.database.table.name, MAIN_DATABASE));
    const revisions = await fixture.database
        .select({ declarations: object.installationRevision.table.declarations })
        .from(object.installationRevision.table);
    const referenced = [
        ...present(main, "the main database").states,
        ...revisions.map((revision) => revision.declarations),
    ];
    expect([
        listed.toSorted(),
        await read(target.id, taken),
        await refusal(read(target.id, other)),
        await refusal(read(ids.machine, taken)),
    ]).toEqual([
        [taken, ...referenced].toSorted(),
        "taken",
        [
            "NOT_FOUND",
            "BLOB_NOT_FOUND",
            `blob '${other}' referenced by a row of '${ids.space}' not found`,
        ],
        [
            "NOT_FOUND",
            "TRANSFER_TO_CALLER_NOT_FOUND",
            "transfer of this space to a machine the caller acts for not found",
        ],
    ]);
});

test("copy up a shipped release a moving space's rows reference, for a target that ships none", async () => {
    // ship notes' release in the machine's layer
    const host = testHost();
    const directory = await host.files.makeTemporaryDirectory("destack-layer-");
    const build = await fixtureBuild(ids.package, "2026.9.0", host.crypto);
    const shipped = await TestBuildStore.layer([build], directory, host);
    const image = present(shipped.images[0], "the shipped image");

    // move the space with a snapshot row referencing the shipped image
    const fixture = await TestSpace.open({ layer: shipped.layer });
    const now = fixture.machine.clock.now();
    const target = await fixture.universe.enroll(fixture.owner, ids.account, "http://target.test/");
    await fixture.serve();
    await fixture.simulation.idle();
    const owner = fixture.as(fixture.owner.subject).stepUp().endpoint(stackPackage);
    await createClient(stackPackage, owner, fixture.machine.environment).transfer.create({
        spaceId: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        targetId: target.id,
    });
    await fixture.database.transaction(
        async (transaction) =>
            await transaction.insert(object.snapshot.table).values({
                id: schema
                    .identifier("snapshot")
                    .parse(object.snapshot.generateId(fixture.environment)),
                spaceId: ids.space,
                resourceId: ResourceId.parse("database-01996ab0-0000-7000-8000-0000000000f2"),
                recipient: (await Recipient.generate(fixture.environment)).key,
                digest: image,
                takenAt: now,
                blobs: [image],
                createdAt: now,
                updatedAt: now,
            }),
    );

    // list the image to the target, served from the space's own blobs after the copy-up
    const handoff = fixture.handoff(fixture.as(target.id));
    const listed = await handoff.blobs(ids.space);
    const chunks: Uint8Array[] = [];
    for await (const blob of handoff.batch(ids.space, [image])) {
        for await (const chunk of blob.body) {
            chunks.push(chunk);
        }
    }
    const served = BuildImage.parse(JSON.parse(new TextDecoder().decode(Buffer.concat(chunks))));
    const [main] = await fixture.database
        .select({ states: object.database.table.states })
        .from(object.database.table)
        .where(eq(object.database.table.name, MAIN_DATABASE));
    const revisions = await fixture.database
        .select({ declarations: object.installationRevision.table.declarations })
        .from(object.installationRevision.table);
    const referenced = [
        ...present(main, "the main database").states,
        ...revisions.map((revision) => revision.declarations),
    ];
    expect({
        listed: listed.toSorted(),
        root: served.root,
        missing: await (await fixture.space).blobs.missing([image]),
    }).toEqual({
        listed: [image, ...referenced].toSorted(),
        root: shipped.layer.image(image)?.root,
        missing: [],
    });
});
