import { expect, test } from "@destack/test";
import { mkdir, mkdtemp, readdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { create } from "tar";
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

test("reject an archive link outside its distribution without activating files", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-archive-"));
    try {
        const source = join(directory, "source");
        await mkdir(source);
        await symlink("../../outside", join(source, "escape"));
        const archive = join(directory, "release.tar.gz");
        await create({ file: archive, gzip: true, cwd: source }, ["escape"]);

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
        ).rejects.toThrow("Archive link escapes the distribution: escape");
        expect(await installer.current()).toBeUndefined();
        expect(await readdir(join(installer.directory, "versions"))).toEqual([]);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
