import { GitAuthor } from "@destack/git";
import { installation, build, edit, installationRevision } from "@destack/stack/object";
import { ENDED_STATES } from "@destack/workflow";
import { Failure } from "@destack/error";
import { BuildError } from "@destack/build/error";
import { testHost } from "@destack/test/host";
import registryPackage from "@destack/registry/package";
import { PackageManifest } from "@destack/package/manifest";
import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
import { and, eq } from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";
import type { Launcher } from "../launcher/index.ts";
import { PackageId } from "@destack/package";
import { reference, referenceShape } from "@destack/registry/object";
import forgePackage from "@destack/forge/package";
import { Commit, type Identifier, present, schema } from "@destack/schema";
import { createClient } from "@destack/object/client";
import { RequestId, ObjectReference } from "@destack/sync";
import * as stackObject from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import machinePackage from "../package.ts";
import { FIXTURE_RELEASE, OWNER_EMAIL, TestComputer } from "./test/index.ts";
import { BunLockfile } from "@destack/check/bun";

/** The package of every pushed commit: a library compiling one Bun module. */
const SITE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1");

/** The branch the space's installation follows. */
const MAIN = "refs/heads/main";

/** How long a pushed commit takes at most to deploy to its installation. */
const DEPLOY_MILLISECONDS = 15_000;

/** The author of the snapshot commits. */
const AUTHOR = GitAuthor.of({ name: "Ada", handle: "ada" });

/** The environment of the developer's git commands: no user configuration, a fixed identity. */
const GIT_ENVIRONMENT = {
    PATH: present(testHost().process.variables["PATH"], "the host's PATH"),
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_AUTHOR_NAME: "Ada",
    GIT_AUTHOR_EMAIL: "ada@example.test",
    GIT_COMMITTER_NAME: "Ada",
    GIT_COMMITTER_EMAIL: "ada@example.test",
};

/** Serve the owner's home and the residency's forge, with some fixtures published. */
async function pushHome(published: readonly string[] = []) {
    // serve the owner's home on the cloud computer with some fixtures published, and a fresh working directory
    const { home, login } = await TestComputer.home({ published });
    const { device, world } = home;
    const { files } = home.environment;
    const directory = await files.makeTemporaryDirectory("destack-push-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));

    // create a platform repository as the owner on the residency's forge
    const person = await device.launcher(login);
    const residency = await person.residency(world.owner.account);
    const owner = await servedForge(home, residency, person);
    const created = await owner.repository.create({
        spaceId: residency,
        accountId: world.owner.account,
        requestId: RequestId.create(home.environment),
        name: "site",
        hosting: "platform",
    });

    // lay out the site package in a working tree, installing its scope from the universe's registry
    const { universe } = await home.machine.enrolled("laying out the site");
    const worktree = await writeSite(directory, files, universe.origin);

    // list the repository's references again
    const list = () =>
        owner.repository.refresh({
            spaceId: residency,
            id: created.id,
            requestId: RequestId.create(home.environment),
        });

    // commit a source of the site's version module, push it to main and list the references
    const pushSource = async (source: string) => {
        await files.writeTextFile(join(worktree, "src/version.ts"), source);
        await git(worktree, "add", ".");
        await git(worktree, "commit", "--quiet", "--message", "write the version");
        await git(worktree, "push", "--quiet", present(created.remote, "the remote"), "main");
        await list();

        return await git(worktree, "rev-parse", "HEAD");
    };

    // commit a version of the site
    const push = (version: number) =>
        pushSource(`/** The site's version. */\nexport const version = ${version};\n`);

    // install the site in the owner's home at the repository's main branch
    const service = createClient(
        stackPackage,
        person.endpoint(stackPackage.id, await device.locate(world.owner.home)),
        device.environment,
    );
    // follow the main branch as the person
    const follow = async () => {
        const { items } = await owner.reference.list({
            spaceId: residency,
            where: { parentId: created.id, name: MAIN },
            limit: 1,
        });
        const [found] = items;

        return reference.reference(residency, present(found, "the main branch").id);
    };
    const install = async () =>
        service.installation.create({
            scope: world.owner.home,
            requestId: RequestId.create(home.environment),
            packageId: SITE,
            ...ObjectReference.columns("version", await follow()),
            directory: ".",
            name: "site",
        });

    return {
        home,
        residency,
        owner,
        service,
        space: world.owner.home,
        person,
        worktree,
        repository: created,
        list,
        push,
        pushSource,
        install,
        followed: follow,
    };
}

/** Wait until the residency's home serves its forge in its store, answering it as the owner. */
async function servedForge(home: TestComputer, residency: Identifier<"space">, person: Launcher) {
    // wait until the residency's home deployed the forge's revision
    const { machine, device } = home;
    const database = (await machine.space(residency)).database;
    const deployed = async () =>
        (
            await (
                await machine.space(residency)
            ).installations(async (_packageId, image) =>
                (await machine.space(residency)).builds.root(image),
            )
        ).find(
            (entry) =>
                entry.installation.packageId === forgePackage.id &&
                entry.installation.appliedRevisionId !== null,
        )?.installation;
    if (
        !(await home.world.simulation.wait(
            database.log.waitUntil(
                async () => (await deployed()) !== undefined,
                home.clock.timeout(DEPLOY_MILLISECONDS),
            ),
        ))
    ) {
        const [installed] = await database
            .snapshot()
            .read(installation.table)
            .findMany({ where: { scope: residency, packageId: forgePackage.id } });
        const row =
            installed === undefined
                ? undefined
                : await database
                      .snapshot()
                      .read(installation.table)
                      .findFirst({ where: { id: installed.id } });
        const ready = row?.conditions["Ready"];
        throw new TypeError(
            `the forge of '${residency}' did not deploy: '${ready?.reason ?? "no condition"}' ${ready?.message ?? ""}`,
        );
    }
    const found = present(await deployed(), "the forge");

    // call its package below the installation on the cloud machine serving the home, as the owner
    const owner = createClient(
        registryPackage,
        await person.installation(residency, found.name, registryPackage.id),
        device.environment,
    );

    // wait until the home's store serves it, which the gateway answers unavailable before
    const isServed = async () => {
        const answered = await refusal(owner.repository.list({ spaceId: residency }));

        return answered === "done" || answered[0] !== "SERVICE_UNAVAILABLE";
    };
    if (
        !(await home.world.simulation.wait(
            database.log.waitUntil(isServed, home.clock.timeout(DEPLOY_MILLISECONDS)),
        ))
    ) {
        throw new TypeError(`the store of '${residency}' serves no forge`);
    }

    return owner;
}

/** Lay out the site package in a new git working tree, returning its path. */
async function writeSite(directory: string, files: FileSystem, universe: string): Promise<string> {
    // write the package manifest
    const worktree = join(directory, "worktree");
    await files.mkdir(join(worktree, "src"), { recursive: true });
    await files.writeTextFile(
        join(worktree, "package.json"),
        JSON.stringify({
            name: "@example/site",
            version: "2026.10.0",
            type: "module",
            exports: { ".": "./src/index.ts" },
        }),
    );

    // declare the Destack package
    await files.writeTextFile(
        join(worktree, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: SITE,
            role: "library",
            language: "typescript",
            runtimes: ["bun"],
        }),
    );

    // install its scopes from the universe's registry, as a new checkout does
    await files.writeTextFile(
        join(worktree, "bunfig.toml"),
        [
            "[install.scopes]",
            `"@destack" = { url = "${universe}/npm/" }`,
            `"@example" = { url = "${universe}/npm/" }`,
            "",
        ].join("\n"),
    );

    // write its module and start the repository
    await files.writeTextFile(
        join(worktree, "src/index.ts"),
        'export { version } from "./version.ts";\n',
    );
    await git(worktree, "init", "--quiet", "--initial-branch=main");
    await git(worktree, "config", "user.name", "Ada");
    await git(worktree, "config", "user.email", "ada@example.test");

    return worktree;
}

/** Read the build an installation follows once the space serves it, absent before. */
async function followedBuild(database: DatabaseConnection, id: Identifier<"installation">) {
    const [row] = await database
        .select({ buildId: installationRevision.table.buildId })
        .from(stackObject.installation.table)
        .innerJoin(
            installationRevision.table,
            and(
                eq(installationRevision.table.id, stackObject.installation.table.targetRevisionId),
                eq(installationRevision.table.id, stackObject.installation.table.servedRevisionId),
            ),
        )
        .where(eq(stackObject.installation.table.id, id));

    return row?.buildId;
}

/** Run a git command in a working tree as the fixture's developer, returning its trimmed output. */
async function git(directory: string, ...command: string[]): Promise<string> {
    // run it on the host, reading its output and its errors
    const ran = testHost().process.spawn(["git", "-C", directory, ...command], {
        variables: GIT_ENVIRONMENT,
        stderr: "pipe",
    });
    const [output, errors, code] = await Promise.all([
        new Response(ran.stdout).text(),
        new Response(ran.stderr).text(),
        ran.exited,
    ]);
    if (code !== 0) {
        throw new Error(`git '${command.join(" ")}' failed: '${errors}'`);
    }

    return output.trim();
}

test("build and deploy each commit a push moves main to", async () => {
    // push the site's first version and install it following main
    const site = await pushHome();
    const { machine } = site.home;
    const { space } = site;
    const first = await site.push(1);
    const installed = await site.install();
    const served = (await machine.space(space)).database;

    // wait until the installation follows the first commit's build
    const wait = async (commit: string) => {
        const isDeployed = await site.home.world.simulation.wait(
            served.log.waitUntil(async () => {
                const current = await followedBuild(served, installed.id);
                if (current === undefined) {
                    return false;
                }
                const [built] = await served
                    .select({ source: build.table.source })
                    .from(build.table)
                    .where(eq(build.table.id, current));

                return built?.source.kind === "commit" && built.source.commit === commit;
            }, site.home.clock.timeout(DEPLOY_MILLISECONDS)),
        );
        if (!isDeployed) {
            const builds = await served.select().from(build.table);
            throw new Error(`commit '${commit}' was not deployed: ${JSON.stringify(builds)}`);
        }
    };
    await wait(first);

    // list the repository's references again and push the second version
    await site.list();
    const second = await site.push(2);
    await wait(second);

    // store one build per commit beside the stack's release builds
    const stored = await served
        .select()
        .from(build.table)
        .where(eq(build.table.scope, space))
        .orderBy(build.table.createdAt);
    const builds = stored.filter((row) => row.source.kind === "commit");
    const packages = await Promise.all(
        builds.map(async (row) => {
            const image = present(row.image, "the image");
            const reader = await (await machine.space(space)).builds.root(image);

            return PackageManifest.package(reader.manifest).id;
        }),
    );

    // root the second build alone in the machine's builds, as the space no longer runs the first
    const root = present((await machine.builds.roots()).get(`space:${space}`), "the space's root");
    const rooted = builds.map((row) => root.has(present(row.image, "the image")));

    // compile each build once, though the reconcile creates a deployed commit's build again
    const runs = await served.select().from(stackObject.compile.run.table);
    const compiled = builds.map((row) => runs.filter((run) => run.key === row.id).length);
    expect([
        builds.map((row) => [row.source, row.outputs, row.state, row.key]),
        packages,
        await followedBuild(served, installed.id),
        rooted,
        compiled,
    ]).toEqual([
        [first, second].map((commit) => {
            const source = {
                kind: "commit" as const,
                repository: site.repository.id,
                commit: Commit.parse(commit),
                directory: ".",
            };

            return [
                source,
                ["bun"],
                "succeeded",
                stackObject.Build.key({ source, outputs: null, version: null }),
            ];
        }),
        [SITE, SITE],
        builds[1]?.id,
        [false, true],
        [1, 1],
    ]);
});

test("deploy each push of a checkout's working tree to the installation following it", async () => {
    // commit the site's first version, and register its working tree on the owner's desktop
    const site = await pushHome();
    const { machine } = site.home;
    const desktop = await TestComputer.open({ world: site.home.world });
    await desktop.enroll(await desktop.signIn(OWNER_EMAIL), site.home.world.owner.account);
    const { environment } = desktop;
    const { files } = environment;
    const head = await site.push(1);
    const client = desktop.connect(machinePackage);
    const machineId = desktop.machine.id;
    const opened = await client.checkout.create({
        machineId,
        requestId: RequestId.create(environment),
        directory: site.worktree,
        source: { kind: "existing" },
    });

    // push the working tree as it is with the owner's lease, then observe the repository's references
    const push = async (version: number) => {
        await files.writeTextFile(
            join(site.worktree, "src/version.ts"),
            `/** The site's version. */\nexport const version = ${version};\n`,
        );
        const lease = await site.owner.repository.push({
            spaceId: site.residency,
            id: site.repository.id,
        });
        const pushed = await client.checkout.push({
            machineId,
            id: opened.id,
            requestId: RequestId.create(environment),
            lease,
            of: "tree",
            author: AUTHOR,
        });
        await site.list();

        return pushed;
    };

    // install the site following the checkout's reference
    const first = await push(2);
    const { items } = await site.owner.reference.list({
        spaceId: site.residency,
        where: { parentId: site.repository.id, name: first.reference },
        limit: 1,
    });
    const followed = present(items[0], "the checkout's reference");
    const installed = await site.service.installation.create({
        scope: site.space,
        requestId: RequestId.create(environment),
        packageId: SITE,
        ...ObjectReference.columns("version", reference.reference(site.residency, followed.id)),
        directory: ".",
        name: "site",
    });
    const served = (await machine.space(site.space)).database;

    // wait until the installation serves a commit's build
    const wait = async (commit: string) => {
        const isDeployed = await site.home.world.simulation.wait(
            served.log.waitUntil(async () => {
                const current = await followedBuild(served, installed.id);
                const [built] =
                    current === undefined
                        ? []
                        : await served
                              .select({ source: build.table.source })
                              .from(build.table)
                              .where(eq(build.table.id, current));

                return built?.source.kind === "commit" && built.source.commit === commit;
            }, site.home.clock.timeout(DEPLOY_MILLISECONDS)),
        );
        if (!isDeployed) {
            const builds = await served.select().from(build.table);
            throw new Error(`commit '${commit}' was not deployed: ${JSON.stringify(builds)}`);
        }
    };
    await wait(first.commit);

    // push the working tree again, and wait until its follower serves it
    const second = await push(3);
    await wait(second.commit);

    // push snapshots on HEAD that leave HEAD and the working tree's status as they are
    const parents = [
        await git(site.worktree, "rev-parse", `${first.commit}^`),
        await git(site.worktree, "rev-parse", `${second.commit}^`),
    ];
    expect({
        reference: first.reference,
        same: second.reference === first.reference,
        parents,
        head: await git(site.worktree, "rev-parse", "HEAD"),
        status: await git(site.worktree, "status", "--porcelain"),
        version: await git(site.worktree, "show", `${second.commit}:src/version.ts`),
        // build each commit in the checkout's mirror on its machine, fetching none on the space's runner
        fetched: await files.exists(join(site.home.directory, "commits")),
    }).toEqual({
        reference: `refs/checkouts/${machineId}/${opened.id}`,
        same: true,
        parents: [head, head],
        head,
        status: "M src/version.ts",
        version: "/** The site's version. */\nexport const version = 3;",
        fetched: false,
    });
});

test("fail the installation of a branch whose commit does not build, with the build's failure", async () => {
    // push a version module that does not compile, and install the site following main
    const site = await pushHome();
    const commit = await site.pushSource("export const version = ;\n");
    const installed = await site.install();
    const served = (await site.home.machine.space(site.space)).database;

    // wait until its reconcile fails on the commit's failed build
    const read = async () =>
        present(
            (
                await served
                    .select()
                    .from(stackObject.installation.table)
                    .where(eq(stackObject.installation.table.id, installed.id))
            )[0],
            "the installation",
        );
    const isFailing = async () => (await read()).conditions["Synced"]?.status === "false";
    expect(
        await site.home.world.simulation.wait(
            served.log.waitUntil(isFailing, site.home.clock.timeout(DEPLOY_MILLISECONDS)),
        ),
    ).toBe(true);

    // report the build's failure on the installation and as the controller's refusal
    const built = (await served.select().from(build.table)).find(
        (row) => row.source.kind === "commit",
    );
    const failure = present(built?.failure, "the build's failure").message;
    const message = `cannot deploy commit '${commit}' of 'site': its build failed: ${failure}`;
    const refused = site.home.takeRefusals().map((each) => each.join(" | "));
    expect({
        state: built?.state,
        synced: (await read()).conditions["Synced"]?.message,
        refused: [...new Set(refused)],
    }).toEqual({
        state: "failed",
        synced: `@destack/stack BUILD_BLOCKED: ${message}`,
        refused: [`PRECONDITION_FAILED | BUILD_BLOCKED | ${message}`],
    });
});

test("refuse a checkout reference naming no machine, building nothing", async () => {
    // push the site's main to a checkout reference without its machine, and install it following that
    const site = await pushHome();
    const commit = await site.push(2);
    const name = "refs/checkouts/checkout-1";
    const remote = present(site.repository.remote, "the remote");
    await git(site.worktree, "push", "--quiet", remote, `HEAD:${name}`);
    await site.list();
    const { items } = await site.owner.reference.list({
        spaceId: site.residency,
        where: { parentId: site.repository.id, name },
        limit: 1,
    });
    const followed = present(items[0], "the checkout's reference");
    const installed = await site.service.installation.create({
        scope: site.space,
        requestId: RequestId.create(site.home.environment),
        packageId: SITE,
        ...ObjectReference.columns("version", reference.reference(site.residency, followed.id)),
        directory: ".",
        name: "site",
    });
    const served = (await site.home.machine.space(site.space)).database;

    // wait until its reconcile refuses the reference
    const read = async () =>
        present(
            (
                await served
                    .select()
                    .from(stackObject.installation.table)
                    .where(eq(stackObject.installation.table.id, installed.id))
            )[0],
            "the installation",
        );
    const isFailing = async () => (await read()).conditions["Synced"]?.status === "false";
    expect(
        await site.home.world.simulation.wait(
            served.log.waitUntil(isFailing, site.home.clock.timeout(DEPLOY_MILLISECONDS)),
        ),
    ).toBe(true);

    // refuse the reference before any build of the commit exists
    const message = `cannot build reference '${name}': it must name a personal machine, as refs/checkouts/<machine>/<checkout>`;
    const builds = await served.select().from(build.table);
    const refused = site.home.takeRefusals().map((each) => each.join(" | "));
    expect({
        synced: (await read()).conditions["Synced"]?.message,
        built: builds.filter((row) => row.commit === commit).length,
        refused: [...new Set(refused)],
    }).toEqual({
        synced: `@destack/stack RUNNER_BLOCKED: ${message}`,
        built: 0,
        refused: [`PRECONDITION_FAILED | RUNNER_BLOCKED | ${message}`],
    });
});

test("replicate the branch an installation follows into its space", async () => {
    // push the site's first version and install it without granting the machine any role
    const site = await pushHome();
    const { machine } = site.home;
    const { space } = site;
    const first = await site.push(1);
    await site.install();
    const served = (await machine.space(space)).database;

    // wait until the space's replica of main applied its first snapshot
    const main = await site.followed();
    const key = { scope: site.residency, id: schema.identifier("reference").parse(main.id) };
    await site.home.world.simulation.wait(
        served.log.waitUntil(
            () => referenceShape.isSynced(served, key),
            site.home.clock.timeout(DEPLOY_MILLISECONDS),
        ),
    );

    // replicate main at the pushed commit
    const rows = await served.select().from(reference.table);
    expect(rows.map((row) => [row.parentId, row.name, row.commit])).toEqual([
        [site.repository.id, MAIN, first],
    ]);
});

/** Install the site following main, then edit main to depend on a package as its owner. */
async function dependOn(site: Awaited<ReturnType<typeof pushHome>>, name: string) {
    // install the site following main, and wait until the space replicates main
    const installed = await site.install();
    const served = (await site.home.machine.space(site.space)).database;
    const main = await site.followed();
    const key = { scope: site.residency, id: schema.identifier("reference").parse(main.id) };
    await site.home.world.simulation.wait(
        served.log.waitUntil(
            () => referenceShape.isSynced(served, key),
            site.home.clock.timeout(DEPLOY_MILLISECONDS),
        ),
    );

    // edit it as the owner, approving the space's push to the repository
    const approving = createClient(
        stackPackage,
        site.person.endpoint(stackPackage.id, await site.home.device.locate(site.space), {
            decide: async () => "approve",
        }),
        site.home.device.environment,
    );
    const { run } = await approving.installation.edit({
        scope: site.space,
        requestId: RequestId.create(site.home.environment),
        id: installed.id,
        operations: [{ kind: "dependency.add", package: name, range: "latest" }],
        message: `depend on ${name}`,
    });

    // wait until its run ends
    const read = async () =>
        present(
            (await served.select().from(edit.run.table).where(eq(edit.run.table.id, run)))[0],
            "the edit's run",
        );
    const isEnded = await site.home.world.simulation.wait(
        served.log.waitUntil(async () => {
            const { state } = await read();

            return ENDED_STATES.some((ended) => ended === state);
        }, site.home.clock.timeout(DEPLOY_MILLISECONDS)),
    );

    // read the run and main's head as the forge has it
    const remote = present(site.repository.remote, "the remote");
    await git(site.worktree, "fetch", "--quiet", remote, "main");
    const head = await git(site.worktree, "rev-parse", "FETCH_HEAD");

    return { isEnded, ended: await read(), head };
}

test("fail an edit whose dependency does not resolve, leaving its branch at the base", async () => {
    // push the site's first version, then depend on a library nothing publishes
    const site = await pushHome();
    const base = await site.push(1);
    const { isEnded, ended, head } = await dependOn(site, "@example/other");

    // fail it with the install's refusal and push nothing, so main stays at the base
    expect({
        isEnded,
        state: ended.state,
        failure: ended.failure,
        output: ended.output,
        head,
    }).toEqual({
        isEnded: true,
        state: "failed",
        failure: Failure.of(
            BuildError.of(
                "DEPENDENCY_INVALID",
                [
                    "dependencies of '@example/site' are invalid: bun install exited with status 1: Resolving dependencies",
                    "Resolved, downloaded and extracted [1]",
                    "error: GET https://eu.machine.test/spaces/space-***/package-***/npm/@example%2fother - 404",
                    "error: @example/other@latest failed to resolve",
                ].join("\n"),
            ),
        ),
        output: null,
        head: base,
    });
});

test("depend on a published app in one checked commit on main, its lock pinning the release", async () => {
    // push the site's first version, then depend on the published journal
    const site = await pushHome(["journal"]);
    const base = await site.push(1);
    const { isEnded, ended, head } = await dependOn(site, "@example/journal");

    // read what the pushed commit changed, its message and its parent, and check its manifest and lock out
    const changed = await git(site.worktree, "diff", "--name-only", base, "FETCH_HEAD");
    const message = await git(site.worktree, "log", "-1", "--format=%s%n%P", "FETCH_HEAD");
    await git(site.worktree, "checkout", "FETCH_HEAD", "--", "package.json", "bun.lock");
    const { files } = site.home.environment;
    const manifest = schema
        .looseObject({ dependencies: schema.record(schema.string(), schema.string()).default({}) })
        .parse(JSON.parse(await files.readTextFile(join(site.worktree, "package.json"))));
    const lockfile = await BunLockfile.read(site.worktree, files);

    // push one commit on the base changing the manifest and lock alone, the lock pinning the release
    expect({
        isEnded,
        state: ended.state,
        failure: ended.failure,
        output: ended.output,
        changed: changed.split("\n"),
        message: message.split("\n"),
        dependencies: manifest.dependencies,
        locked: lockfile?.packages["@example/journal"]?.[0],
    }).toEqual({
        isEnded: true,
        state: "completed",
        failure: null,
        output: { base, commit: head },
        changed: ["bun.lock", "package.json"],
        message: ["depend on @example/journal", base],
        dependencies: { "@example/journal": "latest" },
        locked: `@example/journal@${FIXTURE_RELEASE}`,
    });
});

test("send npm clients to the forge at the universe's origin alone, leaving a machine's own paths to it", async () => {
    // ask for a published package at the universe's origin and at the cloud machine's own host
    const { home, login, organizations } = await TestComputer.home({ published: ["journal"] });
    const { universe } = await home.machine.enrolled("asking for a package");
    const path = "/npm/@example%2fjournal";
    const atUniverse = await home.browse(
        new Request(`${universe.origin}${path}`, { redirect: "manual" }),
    );
    const atMachine = await home.browse(
        new Request(`https://eu.machine.test${path}`, { redirect: "manual" }),
    );

    // find the publisher's residency home, whose forge serves its packages
    const publisher = present(organizations.get("example"), "the publisher");
    const residency = await (await home.device.launcher(login)).residency(publisher);
    const location = atUniverse.headers.get("location");

    // redirect the one at the universe to the publisher's forge, and answer the other as the machine's
    expect([
        [atUniverse.status, location === null ? null : new URL(location).pathname],
        [atMachine.status, atMachine.headers.get("location")],
    ]).toEqual([
        [307, `/spaces/${residency}/${registryPackage.id}${path}`],
        [404, null],
    ]);
});
