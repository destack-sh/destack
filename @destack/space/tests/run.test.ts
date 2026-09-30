import { expect, test } from "@destack/test";
import type { QueryPage } from "@destack/sync";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { v7 } from "uuid";
import * as object from "../src/object/index.ts";
import { installation } from "../src/object/index.ts";
import { pin } from "./fixture/run.ts";
import { ids, openSpace, serveSpace } from "./fixture/space.ts";

/** Keep the notes installation in a space its owner owns, and serve the space. */
async function open() {
    const database = await openSpace();
    const now = Date.now();
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

    return { database, client: await serveSpace(database) };
}

test("list and follow an installation's runs as its readers, and refuse users recording or changing them", async () => {
    // keep one finished run
    const { database, client } = await open();
    const now = Date.now();
    await database.insert(object.run.table).values({
        id: ids.run,
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        installation: ids.notes,
        call: pin,
        at: now,
        cause: "send",
        state: "succeeded",
        concurrency: "allow",
        attempts: 1,
        startedAt: now,
        finishedAt: now,
        traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
    });

    // list the run as the owner, who reads the installation, and find no space as a stranger
    const listed = async (user: string) =>
        (await client(user).run.list({ spaceId: ids.space })).items.map((item) => [
            item.id,
            item.state,
            item.traceId,
        ]);
    expect(await listed("owner")).toEqual([
        [ids.run, "succeeded", "4bf92f3577b34da6a3ce929d0e0e4736"],
    ]);
    await expect(listed("stranger")).rejects.toEqual(
        new ServiceError("NOT_FOUND", { defined: true, message: `no scope ${ids.space}` }),
    );

    // follow the run in the owner's session
    const controller = new AbortController();
    const pages = await client("owner").replica.sync(
        { scope: ids.space, queries: { runs: { object: "run" } } },
        { signal: controller.signal },
    );
    const { value } = await (pages as AsyncIterable<QueryPage>)[Symbol.asyncIterator]().next();
    controller.abort();
    expect((value as QueryPage).changes.map((change) => [change.table, change.row.id])).toEqual([
        ["destack__space__run", ids.run],
    ]);

    // refuse the owner sending a call or writing runs as the system does
    await expect(
        client("owner").run.send({
            spaceId: ids.space,
            requestId: RequestId.create(),
            installation: ids.notes,
            call: pin,
            at: now,
            cause: "send",
        }),
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: send" });
    const { outcomes } = await client("owner").replica.push({
        scope: ids.space,
        mutations: [
            {
                id: v7(),
                calls: [
                    {
                        method: "run.update",
                        release: object.run.package.version,
                        input: { spaceId: ids.space, id: ids.run, state: "failed" },
                    },
                ],
            },
        ],
    });
    expect(outcomes.map((entry) => entry.outcome)).toEqual([
        { error: { code: "BAD_REQUEST", message: "no mutating method run.update", status: 400 } },
    ]);
});

test("record the calls an installation sends, once per webhook delivery and watch change, running each now unless it names a time", async () => {
    const { client } = await open();
    const notes = client(ids.notes);
    const now = Date.now();
    const send = (fields: Readonly<Record<string, unknown>>) =>
        notes.run.send({
            spaceId: ids.space,
            requestId: RequestId.create(),
            installation: ids.notes,
            call: pin,
            cause: "send",
            ...fields,
        });

    // send a call later, and record a webhook's delivery and a watch's change twice each
    const later = await send({ at: now + 60_000 });
    const webhook = {
        cause: "webhook",
        packageId: ids.package,
        trigger: "pushes",
        deliveryId: "delivery-1",
    };
    const delivered = [await send(webhook), await send(webhook)];
    const watch = {
        cause: "watch",
        packageId: ids.package,
        trigger: "pinned",
        epoch: "epoch-1",
        sequence: 7,
    };
    const changed = [await send(watch), await send(watch)];

    expect([
        [later.cause, later.at - now, later.state, later.concurrency],
        [delivered[0]!.id === delivered[1]!.id, delivered[0]!.concurrency],
        [changed[0]!.id === changed[1]!.id, changed[0]!.concurrency],
    ]).toEqual([
        ["send", 60_000, "pending", "allow"],
        [true, "allow"],
        [true, "queue"],
    ]);

    // refuse recording a schedule's occurrence, which only the cell records, and a cause with another's fields
    const refused = (fields: Readonly<Record<string, unknown>>) =>
        send(fields).then(
            () => "recorded",
            (error: { code: string; message: string }) => [error.code, error.message],
        );
    expect([
        await refused({ cause: "schedule" }),
        await refused({ deliveryId: "delivery-2" }),
        await refused({ ...webhook, deliveryId: undefined }),
    ]).toEqual([
        ["BAD_REQUEST", "a schedule's runs are the cell's to record"],
        ["BAD_REQUEST", "a send run does not take these fields: deliveryId"],
        ["BAD_REQUEST", "a webhook run does not take these fields: deliveryId"],
    ]);
});

test("cancel a waiting run as the space's owner, and refuse cancelling a finished one", async () => {
    const { client } = await open();
    const sent = await client(ids.notes).run.send({
        spaceId: ids.space,
        requestId: RequestId.create(),
        installation: ids.notes,
        call: pin,
        at: Date.now() + 60_000,
        cause: "send",
    });

    // cancel the waiting run, then refuse cancelling it again
    const cancel = (revision: number) =>
        client("owner").run.cancel({
            spaceId: ids.space,
            requestId: RequestId.create(),
            id: sent.id,
            revision,
        });
    const cancelled = await cancel(sent.revision);
    expect([cancelled.state, cancelled.error]).toEqual([
        "skipped",
        { code: "CANCELLED", message: "the run was cancelled" },
    ]);
    await expect(cancel(cancelled.revision)).rejects.toMatchObject({
        code: "CONFLICT",
        message: `run ${sent.id} is skipped`,
    });
});

test("refuse a created schedule with an unknown calendar or time zone, or occurring more than once a minute", async () => {
    const { client } = await open();
    const create = (timing: Readonly<Record<string, unknown>>) =>
        client(ids.notes)
            .schedule.create({
                spaceId: ids.space,
                requestId: RequestId.create(),
                installation: ids.notes,
                name: "digest",
                timing,
                call: pin,
                concurrency: "forbid",
                deadline: 60_000,
            } as never)
            .then(
                () => "created",
                (error: { code: string }) => error.code,
            );

    expect([
        await create({ timing: "cron", cron: "61 * * * *", timezone: "Europe/Vienna" }),
        await create({ timing: "cron", cron: "0 9 * * *", timezone: "Mars/Olympus" }),
        await create({ timing: "interval", interval: 1_000, startsAt: 0 }),
        await create({ timing: "cron", cron: "0 9 * * *", timezone: "Europe/Vienna" }),
    ]).toEqual(["BAD_REQUEST", "BAD_REQUEST", "BAD_REQUEST", "created"]);
});
