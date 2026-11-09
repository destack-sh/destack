import { testHost } from "@destack/test/host";
import registryPackage from "@destack/registry/package";
import { PackageManifest } from "@destack/package/manifest";
import type { FileSystem } from "@destack/environment";
import { join } from "node:path";
import { and, type DatabaseConnection, eq, Snapshot } from "@destack/db";
import type { Session } from "./device/session.ts";
import { PackageId } from "@destack/package";
import { reference, referenceShape } from "@destack/registry/object";
import forgePackage from "@destack/forge/package";
import { type Identifier, present } from "@destack/schema";
import { createClient } from "@destack/object/client";
import { RequestId } from "@destack/sync";
import * as stackObject from "@destack/stack/object";
import { installation } from "@destack/object";
import { build, installationRevision } from "@destack/stack/object";
import objectPackage from "@destack/object/package";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestComputer, SCENARIO_MILLISECONDS } from "./test/index.ts";

/** The package of every pushed commit: a library compiling one Bun module. */
const SITE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1");

/** The branch the space's installation follows. */
const MAIN = "refs/heads/main";

/** How long a pushed commit takes at most to deploy to its installation. */
const DEPLOY_MILLISECONDS = 15_000;

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

/** Serve the owner's home beside the residency's home running the forge, on its cloud machine. */
async function pushHome() {
    // serve the owner's home on the cloud computer, with a fresh working directory
    const { home, login } = await TestComputer.home();
    const { device, world } = home;
    const { files } = home.environment;
    const directory = await files.makeTemporaryDirectory("destack-push-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));

    // create a platform repository as the owner on the residency's forge
    const person = await device.session(login);
    const residency = await person.residency(world.owner.account);
    const owner = await servedForge(home, residency, person);
    const created = await owner.repository.create({
        spaceId: residency,
        accountId: world.owner.account,
        requestId: RequestId.create(home.environment),
        name: "site",
        hosting: "platform",
    });

    // lay out the site package in a working tree
    const worktree = await writeSite(directory, files);

    // list the repository's references again
    const list = () =>
        owner.repository.refresh({
            spaceId: residency,
            id: created.id,
            requestId: RequestId.create(home.environment),
        });

    // commit a version of the site, push it to main and list the references
    const push = async (version: number) => {
        await files.writeTextFile(
            join(worktree, "src/version.ts"),
            `/** The site's version. */\nexport const version = ${version};\n`,
        );
        await git(worktree, "add", ".");
        await git(worktree, "commit", "--quiet", "--message", `write version ${version}`);
        await git(worktree, "push", "--quiet", present(created.remote, "the remote"), "main");
        await list();

        return await git(worktree, "rev-parse", "HEAD");
    };

    // install the site in the owner's home following the repository's main branch
    const service = createClient(
        objectPackage,
        person.endpoint(objectPackage.id, await device.locate(world.owner.home)),
        device.environment,
    );
    const install = () =>
        service.installation.create({
            scope: world.owner.home,
            requestId: RequestId.create(home.environment),
            packageId: SITE,
            selection: {
                kind: "repository",
                repository: created.id,
                reference: MAIN,
                directory: ".",
            },
            name: "site",
        });

    return { home, residency, space: world.owner.home, repository: created, list, push, install };
}

/** Wait until the residency's home serves its forge in its store, answering it as the owner. */
async function servedForge(home: TestComputer, residency: Identifier<"space">, person: Session) {
    // wait until the residency's home deployed the forge's revision
    const { machine, device } = home;
    const database = (await machine.space(residency)).database;
    const deployed = async () =>
        (
            await stackObject.Database.installed(
                Snapshot.live(database),
                residency,
                async (_packageId, manifest) =>
                    (await machine.space(residency)).builds.contents(manifest),
            )
        ).find(
            (entry) =>
                entry.rollout.installation.packageId === forgePackage.id &&
                entry.rollout.appliedRevisionId !== null,
        )?.rollout;
    if (
        !(await home.world.simulation.wait(
            database.log.waitUntil(
                async () => (await deployed()) !== undefined,
                home.clock.timeout(DEPLOY_MILLISECONDS),
            ),
        ))
    ) {
        const [installed] = await Snapshot.live(database)
            .read(installation.table)
            .findMany({ where: { scope: residency, packageId: forgePackage.id } });
        const row =
            installed === undefined
                ? undefined
                : await Snapshot.live(database)
                      .read(installation.table)
                      .findFirst({ where: { id: installed.id } });
        const ready = row?.conditions["Ready"];
        throw new TypeError(
            `the forge of ${residency} did not deploy: ${ready?.reason ?? "no condition"} ${ready?.message ?? ""}`,
        );
    }
    const found = present(await deployed(), "the forge");

    // call its package below the installation on the cloud machine serving the home, as the owner
    const owner = createClient(
        registryPackage,
        await person.installation(residency, found.installation.name, registryPackage.id),
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
        throw new TypeError(`the store of ${residency} serves no forge`);
    }

    return owner;
}

/** Lay out the site package in a new git working tree, returning its path. */
async function writeSite(directory: string, files: FileSystem): Promise<string> {
    // write the package manifest
    const worktree = join(directory, "worktree");
    await files.mkdir(join(worktree, "src"), { recursive: true });
    await files.writeTextFile(
        join(worktree, "package.json"),
        JSON.stringify({
            name: "@acme/site",
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

    // lock its dependencies, none, as `bun install` does
    await files.writeTextFile(
        join(worktree, "bun.lock"),
        JSON.stringify({
            lockfileVersion: 2,
            configVersion: 1,
            workspaces: { "": { name: "@acme/site" } },
            packages: {},
        }),
    );

    // write its module and start the repository
    await files.writeTextFile(
        join(worktree, "src/index.ts"),
        'export { version } from "./version.ts";\n',
    );
    await git(worktree, "init", "--quiet", "--initial-branch=main");

    return worktree;
}

/** Read the build an installation follows once the space serves it, absent before. */
async function followed(database: DatabaseConnection, id: Identifier<"installation">) {
    const [row] = await database
        .select({ buildId: installationRevision.table.buildId })
        .from(installation.table)
        .innerJoin(
            installationRevision.table,
            eq(installationRevision.table.id, installation.table.revisionId),
        )
        .innerJoin(
            stackObject.rollout.table,
            and(
                eq(stackObject.rollout.table.installationId, installation.table.id),
                eq(stackObject.rollout.table.servedRevisionId, installation.table.revisionId),
            ),
        )
        .where(eq(installation.table.id, id));

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
        throw new Error(`git ${command.join(" ")} failed: ${errors}`);
    }

    return output.trim();
}

test(
    "build each commit a push moves main to once in the space selecting it, pulling it as the space, and deploy it from the machine's build store",
    async () => {
        // install the site following main and push its first version
        const site = await pushHome();
        const { machine } = site.home;
        const { space } = site;
        const installed = await site.install();
        const first = await site.push(1);
        const served = (await machine.space(space)).database;

        // wait until the installation follows the first commit's build
        const wait = async (commit: string) => {
            const isDeployed = await site.home.world.simulation.wait(
                served.log.waitUntil(async () => {
                    const current = await followed(served, installed.id);
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
                throw new Error(`commit ${commit} was not deployed: ${JSON.stringify(builds)}`);
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
                const manifest = present(row.manifest, "the manifest");
                const reader = await (await machine.space(space)).builds.contents(manifest);

                return PackageManifest.package(reader.manifest).id;
            }),
        );

        // root the second build alone in the machine's cache, as the space no longer runs the first
        const root = present(
            (await machine.cache.store.roots()).get(`space:${space}`),
            "the space's root",
        );
        const rooted = builds.map((row) => root.has(present(row.manifest, "the manifest")));
        expect([
            builds.map((row) => [row.source, row.outputs, row.state, row.commit]),
            packages,
            await followed(served, installed.id),
            rooted,
        ]).toEqual([
            [first, second].map((commit) => [
                { kind: "commit", repository: site.repository.id, commit, directory: "." },
                ["bun"],
                "succeeded",
                null,
            ]),
            [SITE, SITE],
            builds[1]?.id,
            [false, true],
        ]);
    },
    SCENARIO_MILLISECONDS,
);

test(
    "replicate the branch a space's installation follows as the space inside the account, granting its machine nothing",
    async () => {
        // push the site's first version and install it without granting the machine any role
        const site = await pushHome();
        const { machine } = site.home;
        const { space } = site;
        const first = await site.push(1);
        await site.install();
        const served = (await machine.space(space)).database;

        // wait until the space's replica of main applied its first snapshot
        const key = { scope: site.residency, repository: site.repository.id, name: MAIN };
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
    },
    SCENARIO_MILLISECONDS,
);
