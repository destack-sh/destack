import { testSimulation } from "@destack/object/test";
import { expect, test, refusal } from "@destack/test";
import { join } from "node:path";
import { create, Header, type HeaderData } from "tar";
import { createHash } from "node:crypto";
import { Installer } from "./installer.ts";
import { Release } from "../release/release.ts";

test("recover interrupted activation and retain both distributions", async () => {
    const world = testSimulation().environment("test");
    const directory = await world.files.makeTemporaryDirectory("destack-activation-");
    try {
        // build two archived distributions
        const source = join(directory, "source");
        await world.files.mkdir(source);
        const installer = new Installer(join(directory, "installation"), world);
        const installed = [];
        for (const version of ["2026.9.0", "2026.9.1"]) {
            await world.files.writeTextFile(join(source, "version"), version);
            const archive = join(directory, `${version}.tar.gz`);
            await create({ file: archive, gzip: true, cwd: source }, ["version"]);
            const bytes = await world.files.readFile(archive);
            const sha256 = createHash("sha256").update(bytes).digest("hex");
            installed.push(
                await installer.stage(
                    {
                        release: new Release(version, "x86_64-unknown-linux-gnu"),
                        archive,
                        sha256,
                    },
                    async (path) => {
                        expect(await world.files.readTextFile(join(path, "version"))).toBe(version);
                    },
                ),
            );
        }

        // interrupt the second activation after persisting its intent
        const [first, second] = installed;
        if (first === undefined || second === undefined) {
            throw new Error("expected two staged distributions");
        }
        await installer.activate(first);
        await world.files.writeTextFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({
                version: second.release.version,
                target: second.release.target,
                sha256: second.sha256,
            }),
        );
        expect(await installer.current()).toEqual(first);
        await installer.recover();

        // require a complete selection and preserve the previous distribution
        expect(await installer.current()).toEqual(second);
        expect(
            (await world.files.readDirectory(join(installer.directory, "versions"))).map(
                (entry) => entry.name,
            ),
        ).toEqual([first.release.directory, second.release.directory]);
        expect(await world.files.exists(join(installer.directory, "activate.json"))).toBe(false);
        await installer.recover();
        expect(await installer.current()).toEqual(second);

        // reject an obsolete journal without changing the selected release
        await world.files.writeTextFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({
                version: first.release.version,
                target: first.release.target,
                sha256: first.sha256,
            }),
        );
        expect(await refusal(installer.recover())).toEqual([
            "CONFLICT",
            "DOWNGRADE_REFUSED",
            "cannot activate an older or incompatible release",
        ]);
        expect(await installer.current()).toEqual(second);
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});

test("refuse a Mac activation without its application's absolute destination and identifier", async () => {
    // stage a Mac release in a world on a Mac
    const world = testSimulation().environment("test");
    const directory = await world.files.makeTemporaryDirectory("destack-mac-activation-");
    try {
        const source = join(directory, "source");
        await world.files.mkdir(join(source, "Destack.app"), { recursive: true });
        const archive = join(directory, "release.tar.gz");
        await create({ file: archive, gzip: true, cwd: source }, ["Destack.app"]);
        const sha256 = createHash("sha256")
            .update(await world.files.readFile(archive))
            .digest("hex");
        const installer = new Installer(join(directory, "installation"), world);
        const release = new Release("2026.9.1", "aarch64-apple-darwin");
        const staged = await installer.stage({ release, archive, sha256 }, async () => {});

        // refuse activating it without a destination or an identifier
        const refused = [
            await refusal(installer.activate(staged)),
            await refusal(installer.activate(staged, "/Applications/Destack.app")),
        ];

        // refuse recovering a recorded activation with a relative destination
        const record = { version: release.version, target: release.target, sha256 };
        await world.files.writeTextFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({ ...record, application: "Destack.app", applicationIdentifier: "app" }),
        );
        refused.push(await refusal(installer.recover()));
        expect(refused).toEqual([
            [
                "BAD_REQUEST",
                "OPTIONS_INVALID",
                "macOS releases require an application destination and identifier",
            ],
            [
                "BAD_REQUEST",
                "OPTIONS_INVALID",
                "macOS releases require an application destination and identifier",
            ],
            [
                "INTERNAL_SERVER_ERROR",
                "RECORD_INVALID",
                "missing absolute application destination or identifier",
            ],
        ]);
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});

test.each([
    {
        name: "parent path",
        entry: { path: "../outside", type: "File" },
        message: "unsafe archive path: ../outside",
    },
    {
        name: "absolute path",
        entry: { path: "/outside", type: "File" },
        message: "unsafe archive path: /outside",
    },
    {
        name: "relative symlink",
        entry: { path: "escape", type: "SymbolicLink", linkpath: "../../outside" },
        message: "archive link escapes the distribution: escape",
    },
    {
        name: "absolute symlink",
        entry: { path: "escape", type: "SymbolicLink", linkpath: "/outside" },
        message: "archive link escapes the distribution: escape",
    },
    {
        name: "hard link",
        entry: { path: "escape", type: "Link", linkpath: "../outside" },
        message: "unsupported archive entry: escape (Link)",
    },
    {
        name: "device",
        entry: { path: "device", type: "CharacterDevice" },
        message: "unsupported archive entry: device (CharacterDevice)",
    },
] satisfies { name: string; entry: HeaderData; message: string }[])(
    "reject $name without extracting or activating files",
    async ({ entry, message }) => {
        const world = testSimulation().environment("test");
        const directory = await world.files.makeTemporaryDirectory("destack-archive-");
        try {
            // encode the unmodified unsafe entry, which archive creators otherwise normalize
            const bytes = Buffer.alloc(512 * 3);
            const header = new Header({ ...entry, size: 0, mode: 0o644 });
            header.encode(bytes);
            const archive = join(directory, "release.tar");
            await world.files.writeFile(archive, bytes);

            const installer = new Installer(join(directory, "installation"), world);
            expect(
                await refusal(
                    installer.stage(
                        {
                            release: new Release("2026.9.1", "x86_64-unknown-linux-gnu"),
                            archive,
                            sha256: "a".repeat(64),
                        },
                        async () => {
                            throw new Error("unsafe archive reached executable verification");
                        },
                    ),
                ),
            ).toEqual(["BAD_GATEWAY", "UNSAFE_ARCHIVE", message]);
            expect(await installer.current()).toBeUndefined();
            expect(
                (await world.files.readDirectory(join(installer.directory, "versions"))).map(
                    (version) => version.name,
                ),
            ).toEqual([]);
        } finally {
            await world.files.remove(directory, { recursive: true });
        }
    },
);
