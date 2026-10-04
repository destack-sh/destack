import { expect, refusal, test } from "@destack/test";
import { join } from "node:path";
import { LocalBucket } from "@destack/bucket/local";
import { PackageStore } from "@destack/build/store";
import { TEST_DIALECTS } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { UNRELEASED_MILLISECONDS } from "../src/server/index.ts";
import { SWEEP_INTERVAL_MILLISECONDS } from "../src/server/sweep.ts";
import { principal } from "@destack/access";
import { ids } from "@destack/host/test";
import { Scope } from "@destack/sync";
import { packageObject } from "../src/object/index.ts";
import { BUILDS_PATH } from "../src/service/index.ts";
import { ACCOUNT_ID, fixtureBuild, PackageFixture } from "./fixture/package.ts";

test.each(TEST_DIALECTS)(
    "serve a published release's build by its digest to its package's readers only on %s",
    async (dialect) => {
        // publish a private package, and copy its build by digest into another store as the owner
        await using forge = await PackageFixture.open(dialect);
        const published = await forge.publish("answer");
        await using bucket = await LocalBucket.open(join(forge.directory, "copy"), "host-test");
        const copied = new PackageStore(bucket);
        const location = (digest: string) => ({
            manifest: digest,
            url: `${forge.origin}/builds/${digest}/`,
        });
        await copied.copy(location(published.manifest), as("owner"));

        // refuse anonymous and other callers, an unknown build, and the build once unpublished
        const status = async (digest: string, user?: string) =>
            (await as(user)(new URL(`${location(digest).url}manifest.json`), {})).status;
        const refused = [
            await status(published.manifest),
            await status(published.manifest, "stranger"),
            await status("0".repeat(64), "owner"),
        ];
        await forge.client("owner").release.unpublish({
            accountId: ACCOUNT_ID,
            id: published.release.id,
            requestId: RequestId.create(),
        });
        expect({
            copied: (await copied.contents(published.manifest)).manifest,
            refused,
            unpublished: await status(published.manifest, "owner"),
        }).toEqual({
            copied: published.build.manifest,
            refused: [404, 404, 404],
            unpublished: 404,
        });
    },
);

test.each(TEST_DIALECTS)(
    "refuse a manifest push from a caller who may not publish its package on %s",
    async (dialect) => {
        // publish the package as its owner, then push its next build's files as a stranger
        await using forge = await PackageFixture.open(dialect);
        await forge.publish("answer");
        const build = await fixtureBuild("answer", "2026.9.1");
        const digest = await forge.published.put(build);
        const manifest = await (await forge.published.manifest(digest)).arrayBuffer();

        // refuse the stranger's manifest as a missing package, and keep the build unstored
        const pushed = await as("stranger")(
            new URL(`${forge.origin}/builds/${digest}/manifest.json`),
            { method: "PUT", body: manifest, headers: { "content-type": "application/json" } },
        );
        const body: unknown = await pushed.json();
        expect({
            status: pushed.status,
            body,
            isStored: await forge.forge.store.contains(digest),
        }).toEqual({
            status: 404,
            body: { error: "NOT_FOUND", message: "package not found" },
            isStored: false,
        });
    },
);

test.each(TEST_DIALECTS)(
    "authorize a manifest push from the forge's copy of the account, following a role the account service grants on %s",
    async (dialect) => {
        // publish the package as its owner, and keep its next build to push as a stranger
        await using forge = await PackageFixture.open(dialect);
        await forge.publish("answer");
        const digest = await forge.published.put(await fixtureBuild("answer", "2026.9.1"));
        const push = () =>
            refusal(
                forge.published.push(digest, `${forge.origin}${BUILDS_PATH}`, (request) => {
                    const headers = new Headers(request.headers);
                    headers.set("authorization", "Bearer stranger");

                    return fetch(new Request(request, { headers }));
                }),
            );

        // refuse the stranger, then grant them publishing at the account service and push once the copy follows
        const refused = await push();
        const owner = forge.accounts.user(ids.owner);
        const role = await owner.role.create({
            accountId: ACCOUNT_ID,
            requestId: RequestId.create(),
            name: "publisher",
            description: "Publishes the account's packages",
            permissions: [packageObject.permission("publish")],
        });
        await owner.account.grant({
            scope: ids.owner,
            id: ACCOUNT_ID,
            requestId: RequestId.create(),
            role: role.id,
            subject: principal.user.reference(Scope.universe.id, ids.stranger),
        });
        await forge.settle();
        expect({
            refused,
            accepted: await push(),
            isStored: await forge.forge.store.contains(digest),
        }).toEqual({
            refused: [
                "BAD_GATEWAY",
                `cannot upload ${BUILDS_PATH}${digest}/manifest.json: HTTP 404`,
            ],
            accepted: "done",
            isStored: true,
        });
    },
);

test.each(TEST_DIALECTS)(
    "sweep a pushed but unreleased build once the window passes, keeping released builds and archives on %s",
    async (dialect) => {
        // publish one build and push the next without releasing it
        await using forge = await PackageFixture.open(dialect);
        const published = await forge.publish("answer", { visibility: "public" });
        const unreleased = await forge.push(await fixtureBuild("answer", "2026.9.1"));
        const store = forge.forge.store;
        const stored = async () => ({
            released: await store.contains(published.manifest),
            archive: (await store.bucket.head(`files/${published.release.distribution.digest}`))
                ?.size,
            unreleased: await store.contains(unreleased),
        });

        // keep the unreleased build within the window, and mark it after, then drop it an interval later
        await forge.forge.sweep(Date.now());
        const within = await stored();
        const after = Date.now() + UNRELEASED_MILLISECONDS;
        await forge.forge.sweep(after);
        await forge.forge.sweep(after + SWEEP_INTERVAL_MILLISECONDS);
        expect({ within, after: await stored() }).toEqual({
            within: {
                released: true,
                archive: published.release.distribution.size,
                unreleased: true,
            },
            after: {
                released: true,
                archive: published.release.distribution.size,
                unreleased: false,
            },
        });

        // read and verify every file of the released build after the sweep
        const files = await (await store.contents(published.manifest)).reader.distributed();
        const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
        expect(sizes).toEqual(files.map((file) => file.size));
    },
);

test.each(TEST_DIALECTS)(
    "keep the build of a release created after a sweep marked it through the next sweep on %s",
    async (dialect) => {
        // push a build, and mark it in a sweep after the window
        await using forge = await PackageFixture.open(dialect);
        await forge.publish("answer");
        const build = await fixtureBuild("answer", "2026.9.1");
        const manifest = await forge.push(build);
        const after = Date.now() + UNRELEASED_MILLISECONDS + 1000;
        await forge.forge.sweep(after);

        // release the marked build, then sweep an interval later
        const released = await forge.release("answer", build);
        await forge.forge.sweep(after + SWEEP_INTERVAL_MILLISECONDS);

        // read and verify every file of the released build after the sweep
        const store = forge.forge.store;
        const files = await (await store.contents(manifest)).reader.distributed();
        const sizes = await Promise.all(files.map(async (file) => (await store.file(file)).length));
        expect({ released: released.manifest, sizes }).toEqual({
            released: manifest,
            sizes: files.map((file) => file.size),
        });
    },
);

/** Fetch as a user's bearer token, or anonymously. */
function as(user?: string) {
    return (input: URL, options: RequestInit) =>
        fetch(input, {
            ...options,
            headers: user === undefined ? {} : { authorization: `Bearer ${user}` },
        });
}
