import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { ACCOUNT_ID, RegistryFixture } from "./registry.ts";

test.each(TEST_DIALECTS)(
    "serve npm documents of published releases with conditional reads and per-request authorization on %s",
    async (dialect) => {
        // publish a package and read its packument, a version, and a tag as the owner
        await using registry = await RegistryFixture.open(dialect);
        await registry.publish("answer");
        const authorization = "Bearer owner";
        const read = (path: string, headers: Record<string, string> = { authorization }) =>
            fetch(`${registry.origin}/npm${path}`, { headers });
        const packument = await read("/@example%2fanswer");
        const document = (await packument.json()) as {
            name: string;
            "dist-tags": Record<string, string>;
            versions: Record<string, { name: string; dist: { tarball: string } }>;
        };
        const version = await (await read("/@example/answer/2026.9.0")).json();
        const tagged = await (await read("/@example/answer/latest")).json();
        const tarball = document.versions["2026.9.0"]!.dist.tarball;

        // answer unchanged documents from their entity tag, and hide documents and archives from anonymous and other callers alike
        const cached = await read("/@example/answer", {
            authorization,
            "if-none-match": packument.headers.get("etag")!,
        });
        const denied = await Promise.all(
            [{}, { authorization: "Bearer stranger" } as Record<string, string>].flatMap(
                (headers) =>
                    ["/@example/answer", "/@example/answer/-/answer-2026.9.0.tgz"].map(
                        async (path) => {
                            const response = await read(path, { ...headers, "if-none-match": "*" });

                            return [response.status, await response.json()];
                        },
                    ),
            ),
        );
        expect({
            name: document.name,
            tags: document["dist-tags"],
            versions: Object.keys(document.versions),
            isVersionTagged: JSON.stringify(version) === JSON.stringify(tagged),
            tarball,
            archive: (await fetch(tarball, { headers: { authorization } })).headers.get(
                "content-type",
            ),
            cached: cached.status,
            denied,
        }).toEqual({
            name: "@example/answer",
            tags: { latest: "2026.9.0" },
            versions: ["2026.9.0"],
            isVersionTagged: true,
            tarball: `${registry.origin}/npm/@example/answer/-/answer-2026.9.0.tgz`,
            archive: "application/gzip",
            cached: 304,
            denied: Array.from({ length: 4 }, () => [
                404,
                { error: "package not found", code: "NOT_FOUND" },
            ]),
        });
    },
);

test.each(TEST_DIALECTS)(
    "answer only a package's own archives, versions and tags on %s",
    async (dialect) => {
        // publish two packages, and read each's tarball under the other's name
        await using registry = await RegistryFixture.open(dialect);
        await registry.publish("answer");
        await registry.publish("greeting");
        const read = async (path: string) => {
            const response = await fetch(`${registry.origin}/npm${path}`, {
                headers: { authorization: "Bearer owner" },
            });

            return [response.status, await response.json()];
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
    },
);

test.each(TEST_DIALECTS)(
    "serve public packages to anonymous npm clients with their deprecations, and hide unpublished versions on %s",
    async (dialect) => {
        // publish two versions of a public package, deprecate the first and unpublish the second
        await using registry = await RegistryFixture.open(dialect);
        await registry.publish("answer", { visibility: "public" });
        await registry.publish("answer", { version: "2026.9.1" });
        const owner = registry.client("owner");
        const releases = (await owner.release.list({ accountId: ACCOUNT_ID })).items;
        const change = (version: string) => ({
            accountId: ACCOUNT_ID,
            id: releases.find((entry) => entry.version === version)!.id,
            requestId: RequestId.create(),
        });
        await owner.release.deprecate({ ...change("2026.9.0"), message: "use 2026.9.1" });
        await owner.release.unpublish(change("2026.9.1"));

        // read the packument without credentials
        const response = await fetch(`${registry.origin}/npm/@example/answer`);
        const document = (await response.json()) as {
            "dist-tags": Record<string, string>;
            versions: Record<string, { deprecated?: string }>;
            time: Record<string, string>;
        };
        expect({
            status: response.status,
            tags: document["dist-tags"],
            versions: Object.entries(document.versions).map(([version, entry]) => [
                version,
                entry.deprecated,
            ]),
            times: Object.keys(document.time).sort(),
        }).toEqual({
            status: 200,
            tags: { latest: "2026.9.0" },
            versions: [["2026.9.0", "use 2026.9.1"]],
            times: ["2026.9.0", "created", "modified"],
        });
    },
);

test("refuse npm's writes, which Destack's own release and tag calls take", async () => {
    // send each write npm makes, with the owner's credentials
    await using registry = await RegistryFixture.open("sqlite");
    const write = async (method: string, path: string) => {
        const response = await fetch(`${registry.origin}/npm${path}`, {
            method,
            headers: { authorization: "Bearer owner", "content-type": "application/json" },
            ...(method === "PUT" ? { body: "{}" } : {}),
        });

        return [response.status, response.headers.get("allow"), await response.json()];
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
