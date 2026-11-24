import { join } from "node:path";
import { type Environment, Pipe } from "@destack/environment";
import type { Simulation } from "@destack/environment/simulation";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { schema, present } from "@destack/schema";

/** The public npm registry's host, which the mirror answers as. */
const REGISTRY_HOST = "registry.npmjs.org";

/** The port HTTPS answers at. */
const HTTPS_PORT = 443;

/** The node the mirror answers from. */
const MIRROR_NODE = "npm";

/** The suffix of a release Bun's cache keeps extracted, as in `zod@4.6.7@@@1`. */
const CACHED_SUFFIX = "@@@1";

/** The path of a release's tarball below the registry, as in `/zod/-/zod-4.6.7.tgz`. */
const TARBALL_PATH = /^\/(.+)\/-\/[^/]+-([^/]+)\.tgz$/u;

/** The fields of a package file npm clients resolve by before they download. */
const INSTALL_FIELDS = [
    "dependencies",
    "optionalDependencies",
    "peerDependencies",
    "peerDependenciesMeta",
    "bin",
    "engines",
    "os",
    "cpu",
] as const;

/** The public npm registry as a world sees it: the releases the host's Bun cache holds. */
export class NpmMirror {
    /** The host's Bun cache. */
    readonly #cache: string;
    /** Each release's packed tarball and its integrity, packed on first ask. */
    readonly #packed = new Map<
        string,
        Promise<{ readonly bytes: Uint8Array<ArrayBuffer>; readonly integrity: string }>
    >();
    /** The host whose files, compression and crypto the mirror reads, packs and hashes with. */
    readonly #environment: Pick<Environment<"files">, "files" | "compression" | "crypto">;

    /** Mirror the releases a host's Bun cache holds. */
    constructor(
        cache: string,
        environment: Pick<Environment<"files">, "files" | "compression" | "crypto">,
    ) {
        this.#cache = cache;
        this.#environment = environment;
    }

    /** Read the host's Bun cache from its variables, as Bun finds it. */
    static cache(variables: Readonly<Record<string, string | undefined>>): string {
        const given = variables["BUN_INSTALL_CACHE_DIR"];
        const home =
            variables["BUN_INSTALL"] ??
            join(present(variables["HOME"], "the host's home directory"), ".bun");

        return given ?? join(home, "install", "cache");
    }

    /** Answer as the registry on a world's network until disposed. */
    serve(simulation: Simulation): Disposable {
        const listener = simulation.network.server(MIRROR_NODE).http({
            hostname: REGISTRY_HOST,
            port: HTTPS_PORT,
            fetch: (request) => this.fetch(request),
        });

        return { [Symbol.dispose]: () => void listener.stop(true) };
    }

    /** Answer a package's listing or a release's tarball, 404 for one the cache lacks. */
    async fetch(request: Request): Promise<Response> {
        // answer a tarball, else a listing
        const path = decodeURIComponent(new URL(request.url).pathname);
        const [, tarballName, version] = TARBALL_PATH.exec(path) ?? [];
        if (tarballName !== undefined && version !== undefined) {
            return this.#tarball(tarballName, version);
        }

        return this.#packument(path.slice(1));
    }

    /** List a package's cached releases in npm's abbreviated form, tagging the newest latest. */
    async #packument(name: string): Promise<Response> {
        // answer 404 for a package the cache lacks
        const versions = await this.#versions(name);
        if (versions.length === 0) {
            return new Response(null, { status: 404 });
        }

        // list what npm clients resolve by of each release, and where its tarball lies
        const { files } = this.#environment;
        const listed = await Promise.all(
            versions.map(async (version) => {
                // read the release's package file and its tarball's address
                const file = join(this.#directory(name, version), "package.json");
                const manifest = schema
                    .looseObject({})
                    .parse(JSON.parse(await files.readTextFile(file)));
                const fields = Object.fromEntries(
                    INSTALL_FIELDS.flatMap((field) =>
                        manifest[field] === undefined ? [] : [[field, manifest[field]]],
                    ),
                );
                const base = name.slice(name.lastIndexOf("/") + 1);
                const tarball = `https://${REGISTRY_HOST}/${name}/-/${base}-${version}.tgz`;
                const { integrity } = await this.#pack(name, version);

                return [
                    version,
                    { ...fields, name, version, dist: { tarball, integrity } },
                ] as const;
            }),
        );

        // tag the newest release latest, a prerelease only when the cache holds no release
        const releases = versions.filter((version) => !version.includes("-"));
        const candidates = releases.length === 0 ? versions : releases;
        const latest = candidates.toSorted(Bun.semver.order).at(-1);

        return Response.json({
            name,
            "dist-tags": { latest },
            versions: Object.fromEntries(listed),
        });
    }

    /** Answer a cached release's tarball, 404 for one the cache lacks. */
    async #tarball(name: string, version: string): Promise<Response> {
        // answer 404 for a release the cache lacks
        if (!(await this.#environment.files.exists(this.#directory(name, version)))) {
            return new Response(null, { status: 404 });
        }

        return new Response((await this.#pack(name, version)).bytes);
    }

    /** Pack a cached release into a gzipped tarball once, with its integrity. */
    #pack(
        name: string,
        version: string,
    ): Promise<{ readonly bytes: Uint8Array<ArrayBuffer>; readonly integrity: string }> {
        // take one packed or packing
        const key = `${name}@${version}`;
        const known = this.#packed.get(key);
        if (known !== undefined) {
            return known;
        }

        // pack it once
        const packing = this.#archive(this.#directory(name, version));
        this.#packed.set(key, packing);

        return packing;
    }

    /** Archive a directory's files in order, gzipped, and hash the archive as npm's integrity. */
    async #archive(
        directory: string,
    ): Promise<{ readonly bytes: Uint8Array<ArrayBuffer>; readonly integrity: string }> {
        // list the files in order
        const { files, compression, crypto } = this.#environment;
        const listed = await files.readDirectory(directory, { recursive: true });
        const paths = listed
            .filter((entry) => entry.kind === "file")
            .map((entry) => entry.name)
            .toSorted();

        // archive and gzip them
        const entries = (async function* () {
            for (const path of paths) {
                yield TarballEntry.of(
                    `package/${path}`,
                    await files.readFile(join(directory, path)),
                );
            }
        })();
        const zipped = Pipe.through(Tarball.stream(entries), compression.compress("gzip"));
        const bytes = new Uint8Array(await new Response(zipped).arrayBuffer());

        // hash it as npm's subresource integrity
        const digest = new Uint8Array(await crypto.digest("SHA-512", bytes));

        return { bytes, integrity: `sha512-${digest.toBase64()}` };
    }

    /** List the versions of a package the cache holds. */
    async #versions(name: string): Promise<readonly string[]> {
        // list the scope's or the cache's entries
        const { files } = this.#environment;
        const slash = name.indexOf("/");
        const parent = slash === -1 ? this.#cache : join(this.#cache, name.slice(0, slash));
        const prefix = `${name.slice(slash + 1)}@`;
        const entries = (await files.exists(parent)) ? await files.readDirectory(parent) : [];

        // take the extracted releases of the package
        return entries
            .map((entry) => entry.name)
            .filter((entry) => entry.startsWith(prefix) && entry.endsWith(CACHED_SUFFIX))
            .map((entry) => entry.slice(prefix.length, -CACHED_SUFFIX.length));
    }

    /** Read the directory a cached release lies in. */
    #directory(name: string, version: string): string {
        return join(this.#cache, `${name}@${version}${CACHED_SUFFIX}`);
    }
}
