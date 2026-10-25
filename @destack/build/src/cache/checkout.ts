import { join } from "node:path";
import { type Environment, FileSystemError } from "@destack/environment";
import { Digest, schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import { relativePath } from "../source/dependency.ts";

/** The mode git stores a symbolic link with, whose blob stores the link's target path. */
const LINK_MODE = "120000";

/** The files no build reads: test modules, the tests folder, docs and recipes. */
const UNREAD_FILES = [/(?:^|\/)[^/]*\.test\.[^/]+$/u, /^tests\//u, /\.md$/u, /(?:^|\/)justfile$/u];

/** The hash functions of git's object formats. */
const ObjectFormat = schema.enum(["sha1", "sha256"]);

/** The WebCrypto algorithm of each git object format. */
const OBJECT_ALGORITHMS = { sha1: "SHA-1", sha256: "SHA-256" } as const;

/** The most bytes a checkout's listing may print, 64 MiB: a million paths of 64 bytes. */
const LISTING_BYTES = 64 * 1024 * 1024;

/** The files of a git checkout: its tracked files and the untracked files git does not ignore. */
export class Checkout {
    /** The checkout's top-level directory. */
    readonly root: string;
    /** The git blob id of each listed file by path, absent when git stores no current one. */
    readonly files: ReadonlyMap<string, string | undefined>;
    /** The hash function of the checkout's object format. */
    readonly #format: schema.Infer<typeof ObjectFormat>;
    /** The files and process the checkout reads through. */
    readonly #environment: Environment<"files" | "process">;

    /** Store a checkout's root, its listed files and its object format. */
    private constructor(
        root: string,
        files: ReadonlyMap<string, string | undefined>,
        format: schema.Infer<typeof ObjectFormat>,
        environment: Environment<"files" | "process">,
    ) {
        // store the root, the listing, the format and the environment
        this.root = root;
        this.files = files;
        this.#format = format;
        this.#environment = environment;
    }

    /** List the files of the git checkout enclosing a directory, refusing one outside. */
    static async list(
        directory: string,
        environment: Environment<"files" | "process">,
    ): Promise<Checkout> {
        // find the checkout's top-level directory and object format
        const revision = ["rev-parse", "--show-toplevel", "--show-object-format"];
        const listed = await Checkout.#git(revision, directory, environment);
        const [root, format] = listed.split("\n");
        if (root === undefined || root === "") {
            throw BuildError.of("CHECKOUT_FAILED", `${directory} is not a git checkout`);
        }

        // list the stored blobs, then the untracked and modified files git stores no blob for
        const [stored, changed] = await Promise.all([
            Checkout.#git(["ls-files", "--stage", "-z"], root, environment),
            Checkout.#git(
                ["ls-files", "--others", "--modified", "--exclude-standard", "-z"],
                root,
                environment,
            ),
        ]);
        const files = new Map<string, string | undefined>();
        for (const entry of Checkout.#entries(stored)) {
            // take a regular file's blob, reading a link through to its target
            const [mode, blob] = entry.slice(0, entry.indexOf("\t")).split(" ");
            files.set(entry.slice(entry.indexOf("\t") + 1), mode === LINK_MODE ? undefined : blob);
        }
        for (const path of Checkout.#entries(changed)) {
            files.set(path, undefined);
        }

        return new Checkout(root, files, ObjectFormat.parse(format), environment);
    }

    /** Digest the files a build reads by path and git blob id, hashing files without a blob. */
    async identify(directory: string): Promise<Digest> {
        // take the files below the directory
        const below = relativePath(this.root, directory);
        const prefix = below === "" ? "" : `${below}/`;
        const files = [...this.files]
            .filter(([path]) => path.startsWith(prefix))
            .map(([path, blob]) => [path.slice(prefix.length), blob] as const);

        // refuse a package outside the checkout or ignored by it, whose listing lacks its manifest
        if (!files.some(([path]) => path === "package.json")) {
            throw BuildError.of("CHECKOUT_FAILED", `git lists no package.json in ${directory}`);
        }

        // key each file a build can read by its stored blob, else by the blob of its bytes, in path order
        const blobs = await Promise.all(
            files
                .filter(([path]) => !UNREAD_FILES.some((pattern) => pattern.test(path)))
                .toSorted(([left], [right]) => (left < right ? -1 : 1))
                .map(async ([path, blob]) => [
                    path,
                    blob ?? (await this.#hash(join(this.root, prefix, path))),
                ]),
        );

        return await Digest.json(blobs, this.#environment.crypto);
    }

    /** Hash a file's bytes through its links as git hashes a blob, null for a deleted file. */
    async #hash(path: string): Promise<string | null> {
        // read the bytes, keying a deleted file as null and refusing a linked directory
        const { files } = this.#environment;
        const bytes = await files.readFile(path).catch(async (error: unknown) => {
            if (FileSystemError.is(error, "NOT_FOUND")) {
                return null;
            } else if ((await files.stat(path).catch(() => undefined))?.kind === "directory") {
                throw BuildError.of("CHECKOUT_FAILED", `git lists the linked directory ${path}`);
            }
            throw error;
        });
        if (bytes === null) {
            return null;
        }

        // put git's blob header before the bytes
        const header = new TextEncoder().encode(`blob ${bytes.byteLength}\0`);
        const object = new Uint8Array(header.byteLength + bytes.byteLength);
        object.set(header);
        object.set(bytes, header.byteLength);

        // hash the object as git hashes a blob
        const algorithm = OBJECT_ALGORITHMS[this.#format];
        const hash = await this.#environment.crypto.digest(algorithm, object);

        return new Uint8Array(hash).toHex();
    }

    /** Split git's NUL-separated output into its entries. */
    static #entries(output: string): string[] {
        return output.split("\0").filter((entry) => entry !== "");
    }

    /** Run git in a directory and read its standard output, refusing a failed run. */
    static async #git(
        command: readonly string[],
        directory: string,
        environment: Environment<"process">,
    ): Promise<string> {
        // run git and read its bounded output and its diagnostics
        const child = environment.process.spawn(["git", ...command], {
            workingDirectory: directory,
            stdout: "pipe",
            stderr: "pipe",
        });
        const [output, diagnostics, code] = await Promise.all([
            Checkout.#bounded(child.stdout, () => child.kill()),
            new Response(child.stderr).text(),
            child.exited,
        ]);

        // refuse a failed run with git's diagnostics, and an output past the bound
        if (code !== 0) {
            const reason = diagnostics.trim();
            throw BuildError.of(
                "CHECKOUT_FAILED",
                `git ${command[0]} failed in ${directory}: ${reason}`,
            );
        } else if (output === undefined) {
            throw BuildError.of(
                "CHECKOUT_FAILED",
                `git ${command[0]} printed over 64 MiB in ${directory}`,
            );
        }

        return output;
    }

    /** Read a stream's text up to the listing bound, stopping its writer and absent past it. */
    static async #bounded(
        stream: ReadableStream<Uint8Array>,
        stop: () => void,
    ): Promise<string | undefined> {
        // decode the chunks until the bound
        const decoder = new TextDecoder();
        let text = "";
        let size = 0;
        for await (const chunk of stream) {
            size += chunk.byteLength;
            if (size > LISTING_BYTES) {
                stop();

                return undefined;
            }
            text += decoder.decode(chunk, { stream: true });
        }

        return text + decoder.decode();
    }
}
