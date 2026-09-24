import { expect, test } from "@destack/test";
import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { create, Header, type HeaderData } from "tar";
import { createHash } from "node:crypto";
import { Installer } from "./installer.ts";
import { Release } from "../release/release.ts";

test("recover interrupted activation and retain both distributions", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-activation-"));
    try {
        // build two archived distributions
        const source = join(directory, "source");
        await mkdir(source);
        const installer = new Installer(join(directory, "installation"));
        const installed = [];
        for (const version of ["2026.9.0", "2026.9.1"]) {
            await writeFile(join(source, "version"), version);
            const archive = join(directory, `${version}.tar.gz`);
            await create({ file: archive, gzip: true, cwd: source }, ["version"]);
            const bytes = await readFile(archive);
            const sha256 = createHash("sha256").update(bytes).digest("hex");
            installed.push(
                await installer.stage(
                    {
                        release: new Release(version, "x86_64-unknown-linux-gnu"),
                        archive,
                        sha256,
                    },
                    async (path) => {
                        expect(await readFile(join(path, "version"), "utf8")).toBe(version);
                    },
                ),
            );
        }

        // interrupt the second activation after persisting its intent
        const [first, second] = installed;
        await installer.activate(first);
        await writeFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({
                ...second.release,
                sha256: second.sha256,
            }),
        );
        expect(await installer.current()).toEqual(first);
        await installer.recover();

        // require a complete selection and preserve the previous distribution
        expect(await installer.current()).toEqual(second);
        expect(await readdir(join(installer.directory, "versions"))).toEqual([
            first.release.directory,
            second.release.directory,
        ]);
        await expect(readFile(join(installer.directory, "activate.json"))).rejects.toMatchObject({
            code: "ENOENT",
        });
        await installer.recover();
        expect(await installer.current()).toEqual(second);

        // reject an obsolete journal without changing the selected release
        await writeFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({
                ...first.release,
                sha256: first.sha256,
            }),
        );
        await expect(installer.recover()).rejects.toThrow(
            "Refusing to activate an older or incompatible release.",
        );
        expect(await installer.current()).toEqual(second);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});

test.each([
    {
        name: "parent path",
        entry: { path: "../outside", type: "File" },
        message: "Unsafe archive path: ../outside",
    },
    {
        name: "absolute path",
        entry: { path: "/outside", type: "File" },
        message: "Unsafe archive path: /outside",
    },
    {
        name: "relative symlink",
        entry: { path: "escape", type: "SymbolicLink", linkpath: "../../outside" },
        message: "Archive link escapes the distribution: escape",
    },
    {
        name: "absolute symlink",
        entry: { path: "escape", type: "SymbolicLink", linkpath: "/outside" },
        message: "Archive link escapes the distribution: escape",
    },
    {
        name: "hard link",
        entry: { path: "escape", type: "Link", linkpath: "../outside" },
        message: "Unsupported archive entry: escape (Link)",
    },
    {
        name: "device",
        entry: { path: "device", type: "CharacterDevice" },
        message: "Unsupported archive entry: device (CharacterDevice)",
    },
] satisfies { name: string; entry: HeaderData; message: string }[])(
    "reject $name without extracting or activating files",
    async ({ entry, message }) => {
        const directory = await mkdtemp(join(tmpdir(), "destack-archive-"));
        try {
            // encode the unmodified unsafe entry, which archive creators otherwise normalize
            const bytes = Buffer.alloc(512 * 3);
            const header = new Header({ ...entry, size: 0, mode: 0o644 });
            header.encode(bytes);
            const archive = join(directory, "release.tar");
            await writeFile(archive, bytes);

            const installer = new Installer(join(directory, "installation"));
            await expect(
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
            ).rejects.toThrow(message);
            expect(await installer.current()).toBeUndefined();
            expect(await readdir(join(installer.directory, "versions"))).toEqual([]);
        } finally {
            await rm(directory, { recursive: true, force: true });
        }
    },
);
