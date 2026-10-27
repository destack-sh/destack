import { Recipient } from "@destack/identity";
import { ResourceId } from "@destack/resource";
import { type Digest, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { createClient } from "@destack/object/client";
import { expect, refusal, test } from "@destack/test";
import * as object from "../../object/index.ts";
import stackPackage from "../../package.ts";
import { ids, TestSpace } from "../../test/index.ts";

test("serve a moving space's target only the blobs the space's snapshots read", async () => {
    // enroll the target as a machine of the space's account and let the owner move the space there
    const fixture = await TestSpace.open();
    const now = fixture.machine.clock.now();
    const target = await fixture.universe.enroll(fixture.owner, ids.account, "http://target.test/");
    const served = await fixture.serve();
    await fixture.settle(served);
    const owner = served.as(fixture.owner.subject).stepUp().endpoint(stackPackage);
    await createClient(stackPackage, owner, fixture.machine.environment).transfer.create({
        spaceId: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        targetId: target.id,
    });

    // write a blob a snapshot of the space reads and one no snapshot reads
    const store = fixture.store();
    const write = (text: string) =>
        store.write(
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

    // read the snapshot's blob as the target, and refuse the other blob and another machine
    const read = async (caller: string, digest: Digest) => {
        const chunks: Uint8Array[] = [];
        for await (const chunk of fixture
            .handoff(served, served.as(caller))
            .read(ids.space, digest)) {
            chunks.push(chunk);
        }

        return new TextDecoder().decode(Buffer.concat(chunks));
    };
    expect([
        await read(target.id, taken),
        await refusal(read(target.id, other)),
        await refusal(read(ids.machine, taken)),
    ]).toEqual([
        "taken",
        ["NOT_FOUND", "BLOB_NOT_FOUND", `no snapshot of ${ids.space} references blob ${other}`],
        [
            "NOT_FOUND",
            "TRANSFER_TO_CALLER_MISSING",
            `${ids.space} is not transferring to a machine the caller acts for`,
        ],
    ]);
});
