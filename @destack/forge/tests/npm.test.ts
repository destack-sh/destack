import { expect, test } from "@destack/test";
import { RequestId } from "@destack/runtime/request";
import { present } from "@destack/schema";
import type { Release, Tag } from "../src/object/index.ts";
import { fixtureBuild } from "./fixture/build.ts";
import { PackageFixture } from "./fixture/package.ts";

test("serve npm documents of published releases with conditional reads and per-request authorization", async () => {
    // publish a package and read its packument, a version, a tag and the archive as the owner
    await using forge = await PackageFixture.open();
    const { release } = await forge.publish(await fixtureBuild("answer"));
    const tags = (await forge.client("owner").tag.list({ spaceId: forge.space })).items;
    const authorization = "Bearer owner";
    const read = (path: string, headers: Record<string, string> = { authorization }) =>
        fetch(`${forge.origin}/npm${path}`, { headers });
    const packument = await read("/@example%2fanswer");
    const etag = present(packument.headers.get("etag"), "the packument's entity tag");
    const archive = await read("/@example/answer/-/answer-2026.9.0.tgz");

    // answer unchanged documents from their entity tag, and hide documents and archives from anonymous and other callers alike
    const cached = await read("/@example/answer", { authorization, "if-none-match": etag });
    const callers: Record<string, string>[] = [{}, { authorization: "Bearer stranger" }];
    const denied = await Promise.all(
        callers.flatMap((headers) =>
            ["/@example/answer", "/@example/answer/-/answer-2026.9.0.tgz"].map(async (path) => {
                const response = await read(path, { ...headers, "if-none-match": "*" });
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
    await using forge = await PackageFixture.open();
    await forge.publish(await fixtureBuild("answer"));
    await forge.publish(await fixtureBuild("greeting"));
    const read = async (path: string) => {
        const response = await fetch(`${forge.origin}/npm${path}`, {
            headers: { authorization: "Bearer owner" },
        });
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
    await using forge = await PackageFixture.open();
    const first = await forge.publish(await fixtureBuild("answer"), "public");
    const second = await forge.publish(await fixtureBuild("answer", "2026.9.1"));
    const owner = forge.client("owner");
    const deprecated = await owner.release.deprecate({
        spaceId: forge.space,
        id: first.release.id,
        requestId: RequestId.create(Date.now()),
        message: "use 2026.9.1",
    });
    await owner.release.yank({
        spaceId: forge.space,
        id: second.release.id,
        requestId: RequestId.create(Date.now()),
        isYanked: true,
    });
    const tags = (await owner.tag.list({ spaceId: forge.space })).items;

    // read the packument without credentials, with the first version alone, and the yanked archive an exact pin downloads
    const response = await fetch(`${forge.origin}/npm/@example/answer`);
    const archive = await fetch(`${forge.origin}/npm/@example/answer/-/answer-2026.9.1.tgz`);
    expect([response.status, await response.json(), archive.status]).toEqual([
        200,
        {
            name: "@example/answer",
            "dist-tags": { latest: "2026.9.0" },
            versions: { "2026.9.0": described(forge, deprecated) },
            time: timed([deprecated], tags),
        },
        200,
    ]);
});

test("refuse npm's writes, which the Destack release and tag calls take", async () => {
    // send each write npm makes, with the owner's credentials
    await using forge = await PackageFixture.open();
    const write = async (method: string, path: string) => {
        const response = await fetch(`${forge.origin}/npm${path}`, {
            method,
            headers: { authorization: "Bearer owner", "content-type": "application/json" },
            ...(method === "PUT" ? { body: "{}" } : {}),
        });
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
function described(forge: PackageFixture, release: Release) {
    const { shasum, integrity, fileCount, unpackedSize } = release.distribution;
    const tarball = `${forge.origin}/npm/@example/answer/-/answer-${release.version}.tgz`;

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
