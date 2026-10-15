import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import { Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { CallError } from "@destack/runtime/error";
import { Installation, installation, installationRevision } from "../../object/index.ts";
import { ids, StackFixture } from "../../test/index.ts";

test("install an application into a space as its owner, and refuse a stranger whether its name is taken or free", async () => {
    const client = await (await StackFixture.open()).serve();
    const create = (user: string, name = "notes") =>
        client(user).installation.create({
            spaceId: ids.space,
            requestId: RequestId.create(Date.now()),
            packageId: ids.package,
            role: "application",
            selection: { kind: "release", version: "2026.9.0" },
            name,
        });

    // install the package as an application under its name
    const installed = await create(ids.owner);
    expect([installed.role, installed.name, installed.packageId, installed.status]).toEqual([
        "application",
        "notes",
        ids.package,
        "enabled",
    ]);

    // refuse a caller with no role in the space alike for a free and a taken name
    for (const name of ["tasks", "notes"]) {
        await expect(create("stranger", name)).rejects.toEqual(
            new CallError("NOT_FOUND", { defined: true, message: `no scope ${ids.space}` }),
        );
    }
});

test("find and list only the enabled installations not being deleted, as a machine opens their views", async () => {
    // keep an enabled, a suspended and a deleting installation following a revision each
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const now = Date.now();
    const keep = async (name: string, state: Partial<Installation>) => {
        const id = Identifier.create("installation");
        const revisionId = Identifier.create("installation-revision");
        await database.insert(installation.table).values({
            id,
            scope: ids.space,
            packageId: Identifier.create("package"),
            role: "application",
            name,
            selection: { kind: "release", version: "2026.9.0" },
            createdAt: now,
            updatedAt: now,
        });
        await database.insert(installationRevision.table).values({
            id: revisionId,
            scope: ids.space,
            installationId: id,
            build: { kind: "release", version: "2026.9.0", manifest: "e".repeat(64) },
            views: {},
            settings: [],
            catalogs: [],
            digest: "e".repeat(64),
            createdAt: now,
            updatedAt: now,
        });
        await database
            .update(installation.table)
            .set({ revisionId, ...state })
            .where(eq(installation.table.id, id));
    };
    await keep("notes", {});
    await keep("tasks", { status: "suspended" });
    await keep("board", { deletedAt: now });

    // open only the enabled one by its name and among the space's installations
    const found = async (name: string) =>
        (await Installation.find(database, ids.space, name))?.name;
    const listed = await Installation.list(database, ids.space);
    expect([
        await found("notes"),
        await found("tasks"),
        await found("board"),
        listed.map((entry) => entry.installation.name),
    ]).toEqual(["notes", undefined, undefined, ["notes"]]);
});

test("roll an installation back to the revision applied before, a reused one too, and refuse one without", async () => {
    // keep an application with three revisions
    const fixture = await StackFixture.open();
    const client = await fixture.serve();
    const owner = client(ids.owner);
    const installed = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "application",
        selection: { kind: "release", version: "2026.9.2" },
        name: "notes",
    });
    const revise = async (version: string) => {
        const id = Identifier.create("installation-revision");
        const now = Date.now();
        await fixture.database.insert(installationRevision.table).values({
            id,
            scope: ids.space,
            installationId: installed.id,
            build: await fixture.release(ids.package, version),
            views: {},
            settings: [],
            catalogs: [],
            digest: version.replaceAll(".", "").padEnd(64, "0"),
            createdAt: now,
            updatedAt: now,
        });

        return id;
    };
    const first = await revise("2026.9.0");
    const second = await revise("2026.9.1");
    const third = await revise("2026.9.2");
    const read = async () => {
        const [row] = await fixture.database
            .select()
            .from(installation.table)
            .where(eq(installation.table.id, installed.id));

        return present(row, "the installation");
    };
    const apply = async (revisionId: Identifier<"installation-revision">) => {
        const fields = Installation.applied(await read(), revisionId);
        await fixture.database
            .update(installation.table)
            .set({ revisionId, ...fields })
            .where(eq(installation.table.id, installed.id));
    };
    const rollBack = () =>
        owner.installation.rollBack({
            spaceId: ids.space,
            id: installed.id,
            requestId: RequestId.create(Date.now()),
        });

    // refuse a rollback before a second revision applied
    await apply(first);
    const refused = await rollBack().then(
        () => "done",
        (error: unknown) => error,
    );

    // follow the revision applied before, with its release pinned
    await apply(second);
    await apply(third);
    const back = await rollBack();
    const { revisionId, selection } = await read();

    // follow a newer revision again after the older one is applied again
    await apply(second);
    await apply(first);
    const reused = await rollBack();

    expect({ refused, back, followed: { revisionId, selection }, reused }).toEqual({
        refused: new CallError("CONFLICT", {
            defined: true,
            message: "installation notes has no revision before its applied one",
        }),
        back: { revisionId: second },
        followed: { revisionId: second, selection: { kind: "release", version: "2026.9.1" } },
        reused: { revisionId: second },
    });
});
