import { Plan } from "@destack/resource";
import { expect, test } from "@destack/test";
import { identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { ServiceError } from "@destack/service/error";
import { eq } from "@destack/db";
import { openRelease } from "../src/server/index.ts";
import { installation, installationRevision, space } from "../src/object/index.ts";
import { PackageId } from "@destack/package";
import { ids, openBuild, openSpace, serveSpace } from "./fixture/space.ts";

/** The checkout the stack follows. */
const selection = {
    kind: "checkout" as const,
    host: ids.host,
    checkout: identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    directory: "." as const,
};

test("submit a stack build to the space's stack installation, apply it, and open the views its installations' revisions declare", async () => {
    const database = await openSpace();
    const owner = (await serveSpace(database))("owner");

    // create the space's stack installation, following a checkout
    const stack = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        packageId: ids.package,
        role: "stack",
        selection,
        export: "personal",
        parameters: {},
        alias: "stack",
    });
    const submission = {
        spaceId: ids.space,
        id: stack.id,
        selection,
        build: { ...selection, manifest: "a".repeat(64) },
        evaluation: {
            export: "personal",
            parameters: {},
            definition: {
                roles: { editor: { description: "Edit notes", permissions: [] } },
                installations: {
                    notes: {
                        package: { id: ids.package, name: "@example/notes", version: "2026.9.0" },
                        status: "enabled",
                        alias: "notes",
                        bindings: {},
                        compute: {},
                        tags: {},
                    },
                },
            },
        },
    };

    // submit the build, and let the stack controller apply its safe plan without approval
    await owner.installation.submit({ ...submission, requestId: RequestId.create() });
    await expect
        .poll(async () => {
            const applied = await owner.installation.get({ spaceId: ids.space, id: stack.id });

            return [applied.appliedRevisionId === null, applied.conditions.ready?.reason];
        })
        .toEqual([false, "Applied"]);
    expect(await owner.installation.plan({ spaceId: ids.space, id: stack.id })).toEqual({
        steps: [],
    });

    // open a view the installed release's revision declares, and refuse opening one it does not
    const [notes] = (await owner.installation.list({ spaceId: ids.space })).items.filter(
        (entry) => entry.alias === "notes",
    );
    expect(
        await owner.installation.open({
            spaceId: ids.space,
            id: notes!.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes!.id,
        revisionId: notes!.revisionId,
        build: { kind: "release", version: "2026.9.0" },
        view: "home",
        definition: { entrypoint: "view.js", permissions: [], output: "browser" },
        path: "/",
    });
    await expect(
        owner.installation.open({ spaceId: ids.space, id: notes!.id, view: "settings", path: "/" }),
    ).rejects.toEqual(
        new ServiceError("NOT_FOUND", {
            defined: true,
            message: "notes declares no view settings",
        }),
    );
});

test("apply an earlier build submitted again after a later one, approving each plan that waits", async () => {
    const database = await openSpace();
    const owner = (await serveSpace(database))("owner");
    const stack = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        packageId: ids.package,
        role: "stack",
        selection,
        export: "personal",
        parameters: {},
        alias: "stack",
    });

    // submit a build declaring one role, then one declaring another, then the first again
    const submit = async (role: string, manifest: string) => {
        const submission = {
            spaceId: ids.space,
            id: stack.id,
            selection,
            build: { ...selection, manifest: manifest.repeat(64) },
            evaluation: {
                export: "personal",
                parameters: {},
                definition: { roles: { [role]: { description: role, permissions: [] } } },
            },
        };
        await owner.installation.submit({ ...submission, requestId: RequestId.create() });

        // approve a plan waiting for approval, and wait for the stack controller to apply the revision
        const read = () => owner.installation.get({ spaceId: ids.space, id: stack.id });
        const isApplied = async () => {
            const applied = await read();

            return applied.revisionId === applied.appliedRevisionId;
        };
        await expect
            .poll(async () => (await isApplied()) || (await read()).plan !== null)
            .toBe(true);
        if (!(await isApplied())) {
            const waiting = await owner.installation.plan({ spaceId: ids.space, id: stack.id });
            await owner.installation.approve({
                spaceId: ids.space,
                id: stack.id,
                requestId: RequestId.create(),
                plan: await Plan.digest(waiting),
            });
        }
        await expect.poll(isApplied).toBe(true);

        return (await owner.installation.get({ spaceId: ids.space, id: stack.id })).revisionId;
    };
    const first = await submit("editor", "a");
    await submit("viewer", "b");
    await submit("editor", "a");

    // apply the first build's revision once more
    await expect
        .poll(async () => {
            const applied = await owner.installation.get({ spaceId: ids.space, id: stack.id });

            return [
                applied.revisionId === applied.appliedRevisionId,
                applied.conditions.ready?.reason,
            ];
        })
        .toEqual([true, "Applied"]);
    const applied = await owner.installation.get({ spaceId: ids.space, id: stack.id });
    expect(applied.revisionId).toBe(first);
});

test("install an application from a checkout by submitting its build, following its revision's views, and refuse a stack's evaluation for it", async () => {
    const database = await openSpace();
    const owner = (await serveSpace(database))("owner");
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        packageId: ids.package,
        role: "application",
        selection,
        alias: "notes",
    });

    // submit the checkout's build, then open the view it mounts
    const build = { ...selection, manifest: "c".repeat(64) };
    await owner.installation.submit({
        spaceId: ids.space,
        id: notes.id,
        requestId: RequestId.create(),
        selection,
        build,
    });
    const followed = await owner.installation.get({ spaceId: ids.space, id: notes.id });
    expect(
        await owner.installation.open({
            spaceId: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: followed.revisionId,
        build,
        view: "home",
        definition: { entrypoint: "view.js", permissions: [], output: "browser" },
        path: "/",
    });

    // refuse a stack's evaluation submitted to the application
    await expect(
        owner.installation.submit({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(),
            selection,
            build,
            evaluation: { export: "personal", parameters: {}, definition: {} },
        }),
    ).rejects.toEqual(
        new ServiceError("BAD_REQUEST", {
            message: "notes is an application, submitted without a stack's evaluation",
        }),
    );
});

test("wait for an approval of an application's upgrade across every release since the one it applied, refusing another digest", async () => {
    const database = await openSpace();
    const owner = (await serveSpace(database))("owner");
    const release = (version: string) => ({ kind: "release" as const, version });
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        packageId: ids.package,
        role: "application",
        selection: release("2026.9.0"),
        alias: "notes",
    });
    const submit = (version: string) =>
        owner.installation.submit({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(),
            selection: release(version),
            build: release(version),
        });
    const read = () => owner.installation.get({ spaceId: ids.space, id: notes.id });

    // apply the first release, then submit the last and wait on the upgrade across both later releases
    await submit("2026.9.0");
    await expect.poll(async () => (await read()).conditions.ready?.reason).toBe("Deployed");
    await submit("2026.11.0");
    await expect.poll(async () => (await read()).conditions.ready?.reason).toBe("AwaitingApproval");
    const waiting = await owner.installation.plan({ spaceId: ids.space, id: notes.id });
    expect(waiting).toEqual({
        steps: [
            {
                action: "create",
                target: "installation/notes/release/2026.10.0/service/notes/procedure/count",
                risk: "safe",
                detail: "add procedure count",
            },
            {
                action: "delete",
                target: "installation/notes/release/2026.11.0/object/note/relation/editor",
                risk: "backward-incompatible",
                detail: "remove: data stored under it no longer applies",
            },
        ],
    });

    // refuse another digest, then approve the waiting plan and deploy the release
    const digest = await Plan.digest(waiting);
    const approve = (plan: string) =>
        owner.installation.approve({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(),
            plan,
        });
    const other = "0".repeat(64);
    const refusal = await approve(other).then(
        () => "approved",
        (error: { code: string; message: string }) => [error.code, error.message],
    );
    const approved = await approve(digest);
    await expect
        .poll(async () => {
            const applied = await read();

            return [applied.appliedRevisionId === applied.revisionId, applied.plan];
        })
        .toEqual([true, null]);
    expect([refusal, approved]).toEqual([
        ["CONFLICT", `plan ${digest} waits for approval, not ${other}`],
        waiting,
    ]);
});

test("refuse a build of another package, and block an upgrade from a release that is not earlier", async () => {
    // open checkout builds as a fork's, a package the installation does not install
    const database = await openSpace();
    const fork = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f0");
    const owner = (
        await serveSpace(database, undefined, {
            openBuild: (packageId, build) =>
                openBuild(build.kind === "checkout" ? fork : packageId, build),
        })
    )("owner");
    const release = (version: string) => ({ kind: "release" as const, version });
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        packageId: ids.package,
        role: "application",
        selection: release("2026.9.0"),
        alias: "notes",
    });
    const submit = (
        selection: Readonly<Record<string, unknown>>,
        build: Readonly<Record<string, unknown>>,
    ) =>
        owner.installation
            .submit({
                spaceId: ids.space,
                id: notes.id,
                requestId: RequestId.create(),
                selection,
                build,
            } as never)
            .then(
                () => "submitted",
                (error: { code: string; message: string }) => [error.code, error.message],
            );
    const read = () => owner.installation.get({ spaceId: ids.space, id: notes.id });

    // refuse the fork's checkout build at once
    await submit(release("2026.9.0"), release("2026.9.0"));
    await expect.poll(async () => (await read()).conditions.ready?.reason).toBe("Deployed");
    const forked = await submit(selection, { ...selection, manifest: "f".repeat(64) });

    // block the release that upgrades from itself until another submission
    const blocked = await submit(release("2026.12.0"), release("2026.12.0"));
    await expect.poll(async () => (await read()).conditions.ready?.reason).toBe("NoUpgrade");
    expect([forked, blocked, (await read()).conditions.ready?.message]).toEqual([
        ["BAD_REQUEST", `notes installs ${ids.package}, submitted a build of ${fork}`],
        "submitted",
        "notes 2026.12.0 upgrades from 2026.12.0, which is not earlier",
    ]);
});

test("open the release each space's installation of a package follows", async () => {
    // install the package in two spaces at different releases
    const database = await openSpace();
    const now = Date.now();
    const other = identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f1");
    await database
        .insert(space.table)
        .values({ id: other, scope: ids.account, name: "shared", createdAt: now, updatedAt: now });
    const follow = async (scope: string, version: string, suffix: string) => {
        const id = identifier("installation").parse(
            `installation-01996ab0-0000-7000-8000-0000000000${suffix}`,
        );
        const revisionId = identifier("installation-revision").parse(
            `installation-revision-01996ab0-0000-7000-8000-0000000000${suffix}`,
        );
        await database.insert(installation.table).values({
            id,
            scope: identifier("space").parse(scope),
            packageId: ids.package,
            role: "application",
            selection: { kind: "release", version },
            alias: "notes",
            createdAt: now,
            updatedAt: now,
        });
        await database.insert(installationRevision.table).values({
            id: revisionId,
            scope: identifier("space").parse(scope),
            installationId: id,
            build: { kind: "release", version },
            views: {},
            settings: [],
            digest: suffix.repeat(32),
            createdAt: now,
            updatedAt: now,
        });
        await database
            .update(installation.table)
            .set({ revisionId })
            .where(eq(installation.table.id, id));
    };
    await follow(ids.space, "2026.9.0", "e1");
    await follow(other, "2026.10.0", "e2");

    // open each space's release, and refuse a space without the package
    const version = (scope: string) =>
        openRelease(database, openBuild, scope, ids.package).then(
            (build) => build.manifest.package.version,
            (error: Error) => error.message,
        );
    const empty = "space-01996ab0-0000-7000-8000-0000000000f2";
    expect([await version(ids.space), await version(other), await version(empty)]).toEqual([
        "2026.9.0",
        "2026.10.0",
        `no single installation in ${empty} follows a release of ${ids.package}`,
    ]);
});
