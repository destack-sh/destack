import { TestForge } from "../src/test/index.ts";
import { expect, test } from "@destack/test";
import { RequestId } from "@destack/sync";
import { present } from "@destack/schema";
import type { Release, Tag } from "../src/object/index.ts";
import { fixtureBuild } from "./fixture/build.ts";

/** The email a person without access to the account signs in with. */
const STRANGER_EMAIL = "stranger@example.test";

test("serve npm documents of published releases with conditional reads and per-request authorization", async () => {
    // publish a package and read its packument, a version, a tag and the archive as the owner
    const forge = await TestForge.open();
    const { release } = await forge.publish((await fixtureBuild("answer")).reader);
    const tags = (await forge.client(forge.owner).tag.list({ spaceId: forge.space })).items;
    const read = (path: string, headers: Record<string, string> = {}, person = forge.owner) =>
        forge.fetch(`/npm${path}`, { headers }, person);
    const packument = await read("/@example%2fanswer");
    const etag = present(packument.headers.get("etag"), "the packument's entity tag");
    const archive = await read("/@example/answer/-/answer-2026.9.0.tgz");

    // answer unchanged documents from their entity tag, and hide documents and archives from anonymous and other callers alike
    const cached = await read("/@example/answer", { "if-none-match": etag });
    const stranger = await forge.home.universe.signIn(STRANGER_EMAIL);
    const denied = await Promise.all(
        [undefined, stranger].flatMap((person) =>
            ["/@example/answer", "/@example/answer/-/answer-2026.9.0.tgz"].map(async (path) => {
                const response = await (person === undefined
                    ? forge.fetch(`/npm${path}`, { headers: { "if-none-match": "*" } })
                    : read(path, { "if-none-match": "*" }, person));
                const body: unknown = await response.json();

                return [response.status, body];
            }),
        ),
    );
    const version = described(forge, release);
    const documents: unknown[] = await Promise.all([
        packument.json(),
        (await read("/@example/answer/2026.9.0")).json(),
        (await read("/@example/answer/latest")).json(),
    ]);
    expect({
        packument: documents[0],
        version: documents[1],
        tagged: documents[2],
        archive: [
            archive.status,
            archive.headers.get("content-type"),
            archive.headers.get("etag"),
            (await archive.bytes()).length,
        ],
        cached: cached.status,
        denied,
    }).toEqual({
        packument: {
            name: "@example/answer",
            "dist-tags": { latest: "2026.9.0" },
            versions: { "2026.9.0": version },
            time: timed([release], tags),
        },
        version,
        tagged: version,
        archive: [
            200,
            "application/gzip",
            `"${release.distribution.digest}"`,
            release.distribution.size,
        ],
        cached: 304,
        denied: Array.from({ length: 4 }, () => [
            404,
            { error: "package not found", code: "NOT_FOUND" },
        ]),
    });
});

test("answer only a package's own archives, versions and tags", async () => {
    // publish two packages, and read each's tarball under the other's name
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    await forge.publish((await fixtureBuild("greeting")).reader);
    const read = async (path: string) => {
        const response = await forge.fetch(`/npm${path}`, {}, forge.owner);
        const body: unknown = await response.json();

        return [response.status, body];
    };

    // refuse another package's archive, an unpublished version, an unknown tag and an unknown package
    expect([
        await read("/@example/answer/-/greeting-2026.9.0.tgz"),
        await read("/@example/answer/2027.1.0"),
        await read("/@example/answer/next"),
        await read("/@example/missing"),
    ]).toEqual([
        [404, { error: "package archive not found", code: "NOT_FOUND" }],
        [404, { error: "package version or tag not found", code: "NOT_FOUND" }],
        [404, { error: "package version or tag not found", code: "NOT_FOUND" }],
        [404, { error: "package not found", code: "NOT_FOUND" }],
    ]);
});

test("serve public packages to anonymous npm clients with their deprecations, hiding yanked versions from the packument but serving their archives", async () => {
    // publish two versions of a public package, deprecate the first and yank the second
    const forge = await TestForge.open();
    const first = await forge.publish((await fixtureBuild("answer")).reader, "public");
    const second = await forge.publish((await fixtureBuild("answer", "2026.9.1")).reader);
    const owner = forge.client(forge.owner);
    const deprecated = await owner.release.deprecate({
        spaceId: forge.space,
        id: first.release.id,
        requestId: RequestId.create(forge.environment),
        message: "use 2026.9.1",
    });
    await owner.release.yank({
        spaceId: forge.space,
        id: second.release.id,
        requestId: RequestId.create(forge.environment),
        isYanked: true,
    });
    const tags = (await owner.tag.list({ spaceId: forge.space })).items;

    // read the packument without credentials, with the first version alone, and the yanked archive an exact pin downloads
    const response = await forge.fetch("/npm/@example/answer");
    const archive = await forge.fetch("/npm/@example/answer/-/answer-2026.9.1.tgz");
    expect([
        response.status,
        await response.json(),
        [archive.status, (await archive.bytes()).length],
    ]).toEqual([
        200,
        {
            name: "@example/answer",
            "dist-tags": { latest: "2026.9.0" },
            versions: { "2026.9.0": described(forge, deprecated) },
            time: timed([deprecated], tags),
        },
        [200, second.release.distribution.size],
    ]);
});

test("refuse npm's writes, which the Destack release and tag calls take", async () => {
    // send each write npm makes, with the owner's credentials
    const forge = await TestForge.open();
    const write = async (method: string, path: string) => {
        const response = await forge.fetch(
            `/npm${path}`,
            {
                method,
                headers: { "content-type": "application/json" },
                ...(method === "PUT" ? { body: "{}" } : {}),
            },
            forge.owner,
        );
        const body: unknown = await response.json();

        return [response.status, response.headers.get("allow"), body];
    };
    const refused = [
        405,
        "GET, HEAD",
        {
            error: "publish, tag and deprecate releases through Destack",
            code: "METHOD_NOT_SUPPORTED",
        },
    ];
    expect([
        await write("PUT", "/@example%2fanswer"),
        await write("PUT", "/-/package/@example%2fanswer/dist-tags/next"),
        await write("DELETE", "/@example%2fanswer/-rev/1"),
    ]).toEqual([refused, refused, refused]);
});

/** Describe a release of the answer package as its npm version document. */
function described(forge: TestForge, release: Release) {
    const { shasum, integrity, fileCount, unpackedSize } = release.distribution;
    const tarball = `${forge.npm}/@example/answer/-/answer-${release.version}.tgz`;

    return {
        ...release.metadata,
        ...(release.deprecation === null ? {} : { deprecated: release.deprecation }),
        dist: { tarball, shasum, integrity, fileCount, unpackedSize },
    };
}

/** Time a packument by its releases and tags, and each version by its release. */
function timed(releases: readonly Release[], tags: readonly Tag[]): Record<string, string> {
    const stamps = [
        ...releases.map((entry) => entry.createdAt),
        ...tags.map((entry) => entry.updatedAt),
    ];

    return {
        created: new Date(Math.min(...stamps)).toISOString(),
        modified: new Date(Math.max(...stamps)).toISOString(),
        ...Object.fromEntries(
            releases.map((entry) => [entry.version, new Date(entry.createdAt).toISOString()]),
        ),
    };
}
