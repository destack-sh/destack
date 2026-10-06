import { generateKeyPairSync } from "node:crypto";
import { cp, mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { create } from "tar";
import type { UpdaterOptions } from "../update/updater.ts";
import { SignedRepository, SigningKey, TrustedRoot } from "../publish/index.ts";

/** The command a fixture distribution's archive carries, by the platform's file name. */
const COMMAND = process.platform === "win32" ? "destack.exe" : "destack";

/** The Git commit every fixture distribution claims to be built from. */
export const COMMIT = "2c9f1e7d5b3a8f60c4e1d2b7a9f0e3c5d8b1a4f6";

/** A real signed repository and isolated installation served over loopback. */
export class UpdateFixture implements AsyncDisposable {
    /** Temporary files owned by this scenario. */
    readonly directory: string;
    /** Published metadata and archives. */
    readonly path: string;
    /** Source files used when publishing changed contents. */
    readonly source: string;
    /** Distribution archive for this scenario. */
    readonly archive: string;
    /** Archive layout, with native macOS replacement covered by the bundle scenarios. */
    readonly target =
        process.platform === "win32" ? "x86_64-pc-windows-msvc" : "x86_64-unknown-linux-gnu";
    /** Online metadata signers. */
    keys = {
        targets: SigningKey.generate(),
        snapshot: SigningKey.generate(),
        timestamp: SigningKey.generate(),
    };
    /** Root metadata that may be rotated by the scenario. */
    root: ReturnType<typeof TrustedRoot.create>;
    /** Public updater inputs, retaining the initial root through rotation. */
    readonly options: UpdaterOptions;
    /** Loopback server owned by this repository. */
    readonly server: ReturnType<typeof createServer>;
    /** Root signers shared as immutable key material between scenarios. */
    readonly rootKeys: SigningKey[];

    /** Construct repository state before opening the loopback listener. */
    constructor(directory: string, rootKeys: SigningKey[]) {
        // locate this scenario's files
        this.directory = directory;
        this.path = join(directory, "repository");
        this.source = join(directory, "source");
        this.archive = join(directory, "release.tar.gz");
        this.rootKeys = rootKeys;

        // establish independent online trust with the shared offline signers
        this.root = TrustedRoot.create(
            1,
            rootKeys.map((key) => key.public),
            {
                targets: this.keys.targets.public,
                snapshot: this.keys.snapshot.public,
                timestamp: this.keys.timestamp.public,
            },
            new Date(Date.now() + 365 * 86_400_000).toISOString(),
        );
        for (const key of rootKeys.slice(0, 2)) {
            this.root.sign((bytes) => key.sign(bytes));
        }

        // serve the on-disk repository through real HTTP requests
        this.server = createServer((request, response) => {
            // reject requests without a path
            if (request.url === undefined) {
                response.statusCode = 400;
                response.end();
                return;
            }

            // answer with the file, or 404 when it does not exist
            readFile(join(this.path, request.url)).then(
                (bytes) => response.end(bytes),
                (error: NodeJS.ErrnoException) => {
                    response.statusCode = error.code === "ENOENT" ? 404 : 500;
                    response.end();
                },
            );
        });
        this.options = {
            directory: join(directory, "installation"),
            repository: new URL("http://127.0.0.1/"),
            channel: "stable",
            root: SignedRepository.encode(this.root).toString(),
            target: this.target,
        };
    }

    /** Compile the fixture command reporting version 2026.9.1 and archive it as a distribution, returning the directory with both. */
    static async compile(): Promise<string> {
        // compile the command into a fresh directory
        const directory = await mkdtemp(join(tmpdir(), "destack-update-fixture-"));
        const result = await Bun.build({
            entrypoints: [fileURLToPath(new URL("./command.ts", import.meta.url))],
            compile: {
                outfile: join(directory, COMMAND),
                autoloadDotenv: false,
                autoloadBunfig: false,
            },
        });
        if (!result.success) {
            throw new AggregateError(result.logs, "cannot compile update fixture");
        }

        // archive it with an empty desktop as a distribution
        const source = join(directory, "source");
        await mkdir(join(source, "bin"), { recursive: true });
        await mkdir(join(source, "Destack"));
        await cp(join(directory, COMMAND), join(source, "bin", COMMAND));
        await create({ file: join(directory, "release.tar.gz"), gzip: { level: 1 }, cwd: source }, [
            "bin",
            "Destack",
        ]);

        return directory;
    }

    /** Publish a compiled fixture as a version into a fresh repository and open its listener. */
    static async open(
        fixture: string,
        rootKeys: SigningKey[],
        version: string,
    ): Promise<UpdateFixture> {
        // copy shared immutable inputs into an independent scenario directory
        const directory = await mkdtemp(join(tmpdir(), "destack-updater-"));
        const repository = new UpdateFixture(directory, rootKeys);
        try {
            await cp(join(fixture, "source"), repository.source, { recursive: true });
            await cp(join(fixture, "release.tar.gz"), repository.archive);
            await SignedRepository.create(repository.path, 1, repository.root, repository.keys, [
                { target: repository.target, version, commit: COMMIT, archive: repository.archive },
            ]);

            // select an available loopback port for this repository
            await new Promise<void>((resolve, reject) => {
                repository.server.once("error", reject);
                repository.server.listen(0, "127.0.0.1", resolve);
            });
            const address = repository.server.address();
            if (address === null || typeof address === "string") {
                throw new Error("missing server address");
            }
            repository.options.repository.port = String(address.port);

            return repository;
        } catch (error) {
            await repository[Symbol.asyncDispose]();
            throw error;
        }
    }

    /** Close outstanding requests and remove all scenario files. */
    async [Symbol.asyncDispose](): Promise<void> {
        // release HTTP resources before deleting the served repository
        this.server.closeAllConnections();
        if (this.server.listening) {
            await new Promise<void>((resolve, reject) => {
                this.server.close((error) => {
                    // settle once closed
                    if (error === undefined) {
                        resolve();
                    }
                    // fail with the close error
                    else {
                        reject(error);
                    }
                });
            });
        }
        await rm(this.directory, { recursive: true, force: true });
    }
}

/** Generate the same RSA root-key family used by the hardware signing setup. */
export function createRootKey(): SigningKey {
    // retain private material only inside this test process
    const pair = generateKeyPairSync("rsa", { modulusLength: 2048 });

    return new SigningKey(pair.privateKey.export({ format: "pem", type: "pkcs8" }));
}
