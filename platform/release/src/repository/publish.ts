import { readdir } from "node:fs/promises";
import { join } from "node:path";
import { createReadStream } from "node:fs";
import { Metadata, MetadataKind } from "@tufjs/models";
import { compareVersions } from "@destack/update/release";
import { RepositoryConfiguration } from "./index.ts";
import { readRepository } from "./repository.ts";
import { RepositoryRenewal } from "./renewal.ts";
import { ReleaseBucket } from "./bucket.ts";

/** Upload immutable files before publishing the timestamp that makes them current. */
export async function publish(directory: string): Promise<void> {
    // select the destination before accessing its publication credential
    const configuration = new RepositoryConfiguration();
    const bucket = new ReleaseBucket(configuration);
    const files = await inventory(directory);
    for (const path of files) {
        if (
            !/^(metadata\/(?:[1-9]\d*\.(?:root|targets|snapshot)|timestamp)\.json|targets\/[0-9a-f]{64}\.[a-z0-9_-]+\.(?:tar\.gz|dmg|exe)|downloads\.json|install(?:\.ps1)?)$/.test(
                path,
            )
        ) {
            throw new Error(`unexpected file in public release repository: ${path}`);
        }
    }
    const timestamp = "metadata/timestamp.json";
    if (!files.includes(timestamp)) {
        throw new Error("missing signed timestamp");
    }

    // reject metadata rollback before uploading any release files
    const verified = await readRepository(directory, await configuration.root());
    const next = verified.timestamp;
    const targets = verified.targets;
    const snapshotPath = `metadata/${verified.snapshot.signed.version}.snapshot.json`;
    const targetsPath = `metadata/${verified.targets.signed.version}.targets.json`;
    const referenced = new Set([snapshotPath, targetsPath]);
    for (const target of Object.values(verified.targets.signed.targets)) {
        const path = `targets/${target.hashes.sha256}.${target.path}`;
        referenced.add(path);
        if (files.includes(path)) {
            await target.verify(createReadStream(join(directory, path)));
        }
    }
    const current = await bucket.get(timestamp);
    const revision = current.headers.get("etag");
    if (!current.ok && current.status !== 404) {
        await current.body?.cancel();
        throw new Error(`cannot read current release timestamp: ${current.status}`);
    }
    if (current.ok && !revision) {
        await current.body?.cancel();
        throw new Error("current release timestamp has no object revision");
    }
    if (revision) {
        const renewal = await RepositoryRenewal.read(configuration.url, await configuration.root());
        const previous = renewal.timestamp;
        const stored = Metadata.fromJSON(MetadataKind.Timestamp, JSON.parse(await current.text()));
        if (JSON.stringify(stored.toJSON()) !== JSON.stringify(previous.toJSON())) {
            throw new Error("public metadata differs from current storage; retry publication");
        }
        const published = Metadata.fromJSON(
            MetadataKind.Targets,
            JSON.parse(renewal.targets.toString()),
        );
        // resume an interrupted publication only when its signed timestamp is unchanged
        if (
            previous.signed.version > next.signed.version ||
            (previous.signed.version === next.signed.version &&
                JSON.stringify(previous.toJSON()) !== JSON.stringify(next.toJSON()))
        ) {
            throw new Error("metadata revision must increase or retain the exact signed timestamp");
        }
        for (const [path, file] of Object.entries(published.signed.targets)) {
            const old = file;
            const replacement = targets.signed.targets[path];
            if (!replacement) {
                throw new Error(`refusing to remove a published platform: ${path}`);
            }
            if (
                typeof replacement.custom.version !== "string" ||
                typeof old.custom.version !== "string"
            ) {
                throw new Error(`release version is missing: ${path}`);
            }
            const target = path;
            const comparison = compareVersions(replacement.custom.version, old.custom.version);
            if (comparison < 0) {
                throw new Error(`refusing to downgrade ${target}`);
            }
            if (
                comparison === 0 &&
                (replacement.hashes.sha256 !== old.hashes.sha256 ||
                    replacement.length !== old.length)
            ) {
                throw new Error(`published release is immutable: ${target} ${old.custom.version}`);
            }
        }
        // require every omitted archive to retain its authenticated published identity
        for (const target of Object.values(verified.targets.signed.targets)) {
            const path = `targets/${target.hashes.sha256}.${target.path}`;
            const existing = published.signed.targets[target.path];
            if (
                !files.includes(path) &&
                (!existing ||
                    existing.hashes.sha256 !== target.hashes.sha256 ||
                    existing.length !== target.length)
            ) {
                throw new Error(`release archive is missing: ${path}`);
            }
        }
    }
    // require complete artifacts when publishing the first repository
    else {
        await current.body?.cancel();
        for (const path of referenced) {
            if (!files.includes(path)) {
                throw new Error(`release file is missing: ${path}`);
            }
        }
    }

    // upload each immutable object without permitting different content at the same URL
    const mutable = [timestamp, "downloads.json", "install", "install.ps1"];
    const revisions = new Map<string, string | undefined>([[timestamp, revision ?? undefined]]);
    for (const path of mutable.slice(1)) {
        if (files.includes(path)) {
            revisions.set(path, await bucket.revision(path));
        }
    }
    for (const path of files.filter(
        (path) =>
            !mutable.includes(path) &&
            (referenced.has(path) || /^metadata\/\d+\.root\.json$/.test(path)),
    )) {
        await bucket.put(path, join(directory, path), contentType(path), true);
    }
    for (const path of mutable) {
        if (files.includes(path)) {
            await bucket.put(
                path,
                join(directory, path),
                contentType(path),
                false,
                revisions.get(path),
            );
        }
    }
}

/** Select the media type of an explicitly supported publication file. */
function contentType(path: string): string {
    // keep installer downloads distinct from compressed updater archives
    const types = {
        ".json": "application/json",
        ".tar.gz": "application/gzip",
        ".dmg": "application/x-apple-diskimage",
        ".exe": "application/vnd.microsoft.portable-executable",
    };
    for (const [suffix, type] of Object.entries(types)) {
        if (path.endsWith(suffix)) {
            return type;
        }
    }
    if (path === "install" || path === "install.ps1") {
        return "text/plain; charset=utf-8";
    }

    // reject files outside the public release formats
    throw new Error(`unsupported publication media type: ${path}`);
}

/** List files relative to the public repository directory. */
async function inventory(directory: string, prefix = ""): Promise<string[]> {
    const files: string[] = [];
    for (const entry of await readdir(join(directory, prefix), { withFileTypes: true })) {
        const path = prefix ? `${prefix}/${entry.name}` : entry.name;
        if (entry.isDirectory()) {
            files.push(...(await inventory(directory, path)));
        } else if (entry.isFile()) {
            files.push(path);
        } else {
            throw new Error(`unsupported release file: ${path}`);
        }
    }

    return files.sort();
}
