import { TestDirectory } from "@destack/account/test";
import { machine } from "@destack/account/object";
import { Recipient } from "@destack/identity";
import { ResourceId } from "@destack/resource";
import { type Digest, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { createClient } from "@destack/object/client";
import { expect, refusal, test } from "@destack/test";
import * as object from "../../object/index.ts";
import stackPackage from "../../package.ts";
import { ids, openUniverse, StackFixture } from "../../test/index.ts";

/** The machine receiving the space. */
const TARGET = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-00000000000b");

test("serve a moving space's target only the blobs the space's snapshots read", async () => {
    // keep the target as a machine of the space's account and let the owner move the space there
    const fixture = await StackFixture.open();
    const now = Date.now();
    await fixture.database.insert(machine.table).values({
        id: TARGET,
        scope: ids.account,
        name: "target",
        kind: "personal",
        platform: "macOS",
        architecture: "arm64",
        model: "Mac14,2",
        createdAt: now,
        updatedAt: now,
    });
    const opened = await openUniverse();
    const placed = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
    await TestDirectory.place(opened.universe, placed);
    await TestDirectory.publish(opened.universe, TARGET, ids.account, "http://target.test/");
    const { server } = await fixture.server(opened);
    const owner = StackFixture.endpoint(server, ids.owner, { isAuthenticatedNow: true });
    await createClient(stackPackage, owner).transfer.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        targetId: TARGET,
    });

    // keep a blob a snapshot of the space reads beside one no snapshot reads
    const store = fixture.store();
    const write = (text: string) =>
        store.write(
            (async function* () {
                yield new TextEncoder().encode(text);
            })(),
        );
    const [taken, other] = [await write("taken"), await write("other")];
    await fixture.database.insert(object.snapshot.table).values({
        id: schema.identifier("snapshot").parse(object.snapshot.generateId()),
        spaceId: ids.space,
        resourceId: ResourceId.parse("database-01996ab0-0000-7000-8000-0000000000f1"),
        recipient: (await Recipient.generate()).key,
        digest: taken,
        takenAt: now,
        blobs: [taken],
        createdAt: now,
        updatedAt: now,
    });

    // read the snapshot's blob as the target, and refuse the other blob and another machine
    const read = async (caller: string, digest: Digest) => {
        const chunks: Uint8Array[] = [];
        for await (const chunk of StackFixture.handoff(server, caller).read(ids.space, digest)) {
            chunks.push(chunk);
        }

        return new TextDecoder().decode(Buffer.concat(chunks));
    };
    expect([
        await read(TARGET, taken),
        await refusal(read(TARGET, other)),
        await refusal(read(ids.machine, taken)),
    ]).toEqual([
        "taken",
        ["NOT_FOUND", `no snapshot of ${ids.space} reads blob ${other}`],
        ["NOT_FOUND", `${ids.space} is transferring to no machine the caller acts for`],
    ]);
});
