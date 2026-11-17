import type { TestPerson } from "@destack/account/test";
import { Pipe } from "@destack/environment";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import forgePackage from "../src/package.ts";
import { TestForge } from "../src/test/index.ts";
import { fixtureBuild } from "./fixture/build.ts";

/** The tarball address of each release in an npm listing. */
const Listing = schema.looseObject({
    versions: schema.record(
        schema.string(),
        schema.looseObject({ dist: schema.looseObject({ tarball: schema.string() }) }),
    ),
});

/** Fetch from a forge as a person with their token, or anonymously. */
function as(forge: TestForge, person?: TestPerson) {
    return (input: URL) => forge.endpoint(person).fetch(new Request(input));
}

test("list a package to npm clients and serve its release's tarball, to its readers alone", async () => {
    // publish a private package
    const forge = await TestForge.open();
    await forge.publish((await fixtureBuild("answer")).reader);
    const npm = `${forge.origin}${forgePackage.mounts.npm.path}`;
    const listing = new URL(`${npm}/@example%2fanswer`);

    // list it to its owner, as Bun asks for a scoped package
    const listed = await as(forge, forge.owner)(listing);
    const packument: unknown = await listed.json();
    const file = "/@example/answer/-/answer-2026.9.0.tgz";
    const served = await as(forge, forge.owner)(new URL(`${npm}${file}`));
    const archive = new Uint8Array(await served.arrayBuffer());
    const { crypto, compression } = forge.environment;
    const integrity = `sha512-${new Uint8Array(await crypto.digest("SHA-512", archive)).toBase64()}`;

    // address the tarball below the forge's own endpoint, where the caller's token applies
    const address = Listing.parse(packument).versions["2026.9.0"]?.dist.tarball ?? "";
    const mount = address.slice(0, address.length - file.length);
    const tarball = `${mount}${file}`;
    expect([listed.status, mount.endsWith(forgePackage.mounts.npm.path), packument]).toEqual([
        200,
        true,
        {
            name: "@example/answer",
            "dist-tags": { latest: "2026.9.0" },
            versions: {
                "2026.9.0": {
                    name: "@example/answer",
                    version: "2026.9.0",
                    dist: { tarball, integrity },
                },
            },
        },
    ]);

    // serve the release's build below package/ in path order: its sources and its outputs
    const paths: string[] = [];
    const unzipped = Pipe.through(new Blob([archive]).stream(), compression.decompress("gzip"));
    for await (const entry of Tarball.read(unzipped)) {
        paths.push(entry.path);
        await TarballEntry.bytes(entry);
    }
    expect([served.status, paths]).toEqual([
        200,
        [
            "package/destack.json",
            "package/output/bun/src/index.js",
            "package/output/bun/src/index.js.map",
            "package/output/workerd/src/index.js",
            "package/output/workerd/src/index.js.map",
            "package/package.json",
            "package/src/answer.ts",
            "package/src/index.ts",
        ],
    ]);

    // refuse anonymous callers and strangers, concealing the package
    const refused = await Promise.all(
        [undefined, forge.stranger()].map(async (person) => {
            const response = await as(forge, person)(listing);
            const body: unknown = await response.json();

            return [response.status, body];
        }),
    );
    const missing = {
        defined: true,
        code: "NOT_FOUND",
        status: 404,
        message: "package not found",
        domain: "@destack/forge",
        reason: "PACKAGE_NOT_FOUND",
    };
    expect(refused).toEqual([
        [404, missing],
        [404, missing],
    ]);
});
