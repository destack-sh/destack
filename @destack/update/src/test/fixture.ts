import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { create } from "tar";
import {
    type Crypto,
    type Environment,
    FileSystemError,
    type Listener,
} from "@destack/environment";
import { testHost } from "@destack/test/host";
import { Simulation } from "@destack/environment/simulation";
import type { UpdaterOptions } from "../update/updater.ts";
import type { Metadata, Root } from "@tufjs/models";
import { type ReleaseKeys, SignedRepository, SigningKey, TrustedRoot } from "../publish/index.ts";

/** The host the fixture command is built on and runs on, as the updater starts it. */
const HOST = testHost();

/** The command a fixture distribution's archive contains, by the host's file name. */
const COMMAND = HOST.host.os === "win32" ? "destack.exe" : "destack";

/** The version report the fixture command prints, as the compiled command does. */
const REPORT = JSON.stringify({ version: "2026.9.1" });

/** The seed of the worlds scenarios run in. */
const WORLD_SEED = 1;

/** The node the repository answers as in a scenario's world. */
const REPOSITORY_NODE = "repository";

/** The node the device updating runs as in a scenario's world. */
const DEVICE_NODE = "device";

/** The loopback host the repository answers at, the one plaintext host updaters accept. */
const REPOSITORY_HOST = "127.0.0.1";

/** How long fixture root metadata stays valid, in milliseconds: a year. */
const ROOT_LIFETIME_MILLISECONDS = 365 * 86_400_000;

/** The Git commit every fixture distribution claims to be built from. */
export const COMMIT = "2c9f1e7d5b3a8f60c4e1d2b7a9f0e3c5d8b1a4f6";

/** A real signed repository and isolated installation served in a simulated world. */
export class UpdateFixture implements AsyncDisposable {
    /** The caller's world the repository and the device updating run in. */
    readonly simulation: Simulation;
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
        HOST.host.os === "win32" ? "x86_64-pc-windows-msvc" : "x86_64-unknown-linux-gnu";
    /** Online metadata signers. */
    keys: ReleaseKeys;
    /** Root metadata that may be rotated by the scenario. */
    root: ReturnType<typeof TrustedRoot.create>;
    /** Public updater inputs, retaining the initial root through rotation. */
    readonly options: UpdaterOptions;
    /** The repository's listener on the world's network. */
    readonly listener: Listener<undefined>;
    /** Root signers shared as immutable key material between scenarios. */
    readonly rootKeys: SigningKey[];

    /** Construct repository state and listen for its requests in the caller's world. */
    private constructor(
        directory: string,
        rootKeys: SigningKey[],
        keys: ReleaseKeys,
        root: Metadata<Root>,
        simulation: Simulation,
    ) {
        // join the caller's world
        this.simulation = simulation;

        // locate this scenario's files
        this.directory = directory;
        this.path = join(directory, "repository");
        this.source = join(directory, "source");
        this.archive = join(directory, "release.tar.gz");
        this.rootKeys = rootKeys;
        this.keys = keys;
        this.root = root;

        // serve the on-disk repository over the world's network
        this.listener = this.simulation.environment(REPOSITORY_NODE).server.http({
            hostname: REPOSITORY_HOST,
            fetch: (request) => this.#answer(request),
        });
        this.options = {
            directory: join(directory, "installation"),
            repository: this.listener.url,
            root: SignedRepository.encode(this.root).toString(),
            target: this.target,
            environment: this.environment,
        };
    }

    /** The world the device updating runs in. */
    get environment(): Environment<"files" | "process"> {
        return this.simulation.environment(DEVICE_NODE);
    }

    /** Answer a request with the repository file at its path, or 404 when it does not exist. */
    async #answer(request: Request): Promise<Response> {
        const path = join(this.path, new URL(request.url).pathname);
        const { files } = this.simulation.environment(REPOSITORY_NODE);

        return await files.readFile(path).then(
            (bytes) => new Response(bytes.slice()),
            (error: unknown) =>
                new Response(null, { status: FileSystemError.is(error, "NOT_FOUND") ? 404 : 500 }),
        );
    }

    /** Write the fixture command that reports version 2026.9.1 and archive it as a distribution. */
    static async compile(): Promise<string> {
        // write the command into a fresh directory: a shell script where one runs, else a compiled executable
        const { files } = HOST;
        const directory = await files.makeTemporaryDirectory("destack-update-fixture-");
        const outfile = join(directory, COMMAND);
        if (HOST.host.os !== "win32") {
            const script = `#!/bin/sh\nprintf '%s\\n' '${REPORT}'\n`;
            await files.writeTextFile(outfile, script, { mode: 0o755 });
        } else {
            await UpdateFixture.#build(outfile);
        }

        // archive it with an empty desktop as a distribution
        const source = join(directory, "source");
        await files.mkdir(join(source, "bin"), { recursive: true });
        await files.mkdir(join(source, "Destack"));
        await files.copyFile(join(directory, COMMAND), join(source, "bin", COMMAND));
        await create({ file: join(directory, "release.tar.gz"), gzip: { level: 1 }, cwd: source }, [
            "bin",
            "Destack",
        ]);

        return directory;
    }

    /** Compile the fixture command into an executable, where no shell script runs. */
    static async #build(outfile: string): Promise<void> {
        const entrypoint = fileURLToPath(new URL("./command.ts", import.meta.url));
        const { code, stderr } = await HOST.process
            .spawn([
                HOST.process.executable,
                "build",
                entrypoint,
                "--compile",
                "--outfile",
                outfile,
                "--no-compile-autoload-dotenv",
                "--no-compile-autoload-bunfig",
            ])
            .output();
        if (code !== 0) {
            throw new Error(`cannot compile update fixture: ${new TextDecoder().decode(stderr)}`);
        }
    }

    /** Open a world paused at the system's time, near the wall clock tuf-js checks expiry on. */
    static world(): Simulation {
        return new Simulation({ seed: WORLD_SEED, start: HOST.clock.now() });
    }

    /** Publish a compiled fixture as a version into a fresh repository listening in a world. */
    static async open(
        fixture: string,
        rootKeys: SigningKey[],
        version: string,
        simulation: Simulation,
    ): Promise<UpdateFixture> {
        // copy shared immutable inputs into a repository of its own, and publish the version
        const { files } = simulation.environment(DEVICE_NODE);
        const directory = await files.makeTemporaryDirectory("destack-updater-");
        const repository = await UpdateFixture.create(directory, rootKeys, simulation);
        try {
            await files.copy(join(fixture, "source"), repository.source);
            await files.copyFile(join(fixture, "release.tar.gz"), repository.archive);
            await SignedRepository.create(
                repository.path,
                1,
                repository.root,
                repository.keys,
                [
                    {
                        target: repository.target,
                        version,
                        commit: COMMIT,
                        archive: repository.archive,
                    },
                ],
                repository.environment,
            );

            return repository;
        } catch (error) {
            await repository[Symbol.asyncDispose]();
            throw error;
        }
    }

    /** Create an unpublished repository in a directory, signed by two shared root signers. */
    static async create(
        directory: string,
        rootKeys: SigningKey[],
        simulation: Simulation,
    ): Promise<UpdateFixture> {
        // establish independent online trust with the shared offline signers
        const { clock, crypto } = simulation.environment(DEVICE_NODE);
        const keys = {
            targets: await SigningKey.generate("ed25519", crypto),
            snapshot: await SigningKey.generate("ed25519", crypto),
            timestamp: await SigningKey.generate("ed25519", crypto),
        };
        const root = TrustedRoot.create(
            1,
            rootKeys.map((key) => key.public),
            {
                targets: keys.targets.public,
                snapshot: keys.snapshot.public,
                timestamp: keys.timestamp.public,
            },
            new Date(clock.now() + ROOT_LIFETIME_MILLISECONDS).toISOString(),
            simulation.environment(DEVICE_NODE),
        );
        for (const key of rootKeys.slice(0, 2)) {
            await key.sign(root);
        }

        return new UpdateFixture(directory, rootKeys, keys, root, simulation);
    }

    /** Stop the repository's listener and remove all scenario files. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.listener.stop(true);
        await this.environment.files.remove(this.directory, { recursive: true });
    }
}

/** Generate the same RSA root-key family used by the hardware signing setup. */
export async function createRootKey(crypto: Crypto): Promise<SigningKey> {
    return await SigningKey.generate("rsassa-pss-sha256", crypto);
}
