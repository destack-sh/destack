import { dirname, extname, join } from "node:path";
import type { LocalBuildStore } from "@destack/build";
import { type Environment, FileSystemError } from "@destack/environment";
import type { PackageFile } from "@destack/package/file";
import {
    type BuildReader,
    ImportMap,
    type Lock,
    type PackageManifest,
    VIEW_ASSET_PATH,
    ViewDescription,
} from "@destack/package/manifest";
import { Sandbox } from "@destack/package";
import type { Digest } from "@destack/schema";
import { MachineError } from "../error/index.ts";
import { BUILD_PATH } from "./origin.ts";
import { HEADERS_FILE, StaticHost, ViewAssets } from "./host.ts";

/** The headers of a dependency build's files, which never change under its digest. */
const BUILD_HEADERS = "  Cache-Control: public, max-age=31536000, immutable";

/** A static view written out as files, as a static host serves them and a machine's origin does. */
export class StaticExport {
    /** The directory the files are written below. */
    readonly directory: string;
    /** The store with the view's build and every build its pages import. */
    readonly #builds: Pick<LocalBuildStore, "contents" | "imports" | "lock">;
    /** The environment whose file system the export writes to. */
    readonly #environment: Environment<"files">;
    /** The media type of each written file's extension, which the host's headers declare. */
    readonly #types = new Map<string, string>();

    /** Export into a directory from a store of builds. */
    constructor(
        directory: string,
        builds: Pick<LocalBuildStore, "contents" | "imports" | "lock">,
        environment: Environment<"files">,
    ) {
        this.directory = directory;
        this.#builds = builds;
        this.#environment = environment;
    }

    /** Write a lock's static view: its pages, its assets and the builds they import. */
    async write(lock: Digest, view: string, linked: ReadonlyMap<string, Lock>): Promise<void> {
        // read the view's build, its public files and its static site, refusing a view without one
        const locked = await this.#builds.lock(lock);
        const reader = await this.#builds.contents(locked.root);
        const files = StaticHost.publicFiles(reader.manifest, await reader.files());
        const site = await StaticHost.open(reader.manifest, files, view, (file) =>
            reader.load(file.path),
        );
        if (site === undefined) {
            throw MachineError.of("VIEW_NOT_FOUND", `static view '${view}' of the build not found`);
        }

        // map the modules the pages import below the build path, as the origin does
        const closure = await this.#builds.imports(locked, "browser", BUILD_PATH, linked);

        // write the build's public files at their paths, then the site's files at the paths it answers, pages with the map
        for (const file of files.values()) {
            await this.#copy(reader, file, `/${file.path}`);
        }
        for (const { path, file } of [...site.paths(), ...site.configuration()]) {
            const map = file.path.endsWith(".html") ? closure.map : undefined;
            await this.#copy(reader, file, path, map);
        }

        // write each build the pages import below its digest, the view's own among them
        for (const digest of closure.builds) {
            await this.#copyBuild(digest);
        }

        // write each asset the builds emit below the asset path, by its name
        const assets = await ViewAssets.of(closure.builds, (digest) =>
            this.#builds.contents(digest),
        );
        for (const [name, asset] of assets.files) {
            await this.#copy(asset.reader, asset.file, `${VIEW_ASSET_PATH}${name}`);
        }

        // add the views' permissions, the builds' caching and the media types to the site's headers
        await this.#writeHeaders(StaticExport.#policy(reader.manifest));
    }

    /** Copy one file of a build to a path, a page with the import map at the start of its head. */
    async #copy(
        reader: BuildReader,
        file: PackageFile,
        path: string,
        map?: ImportMap,
    ): Promise<void> {
        // read the bytes, inserting the map into a page
        const bytes = await reader.load(file.path);
        const written =
            map === undefined
                ? bytes
                : new TextEncoder().encode(ImportMap.insert(new TextDecoder().decode(bytes), map));

        // write them, recording the media type of the extension
        const target = join(this.directory, path);
        await this.#environment.files.mkdir(dirname(target), { recursive: true });
        await this.#environment.files.writeFile(target, written);
        this.#types.set(extname(path), file.mediaType);
    }

    /** Copy the files a browser loads of a build below the build path, under its digest. */
    async #copyBuild(digest: Digest): Promise<void> {
        const reader = await this.#builds.contents(digest);
        for (const file of StaticExport.#browserFiles(reader.manifest, await reader.files())) {
            await this.#copy(reader, file, `${BUILD_PATH}${digest}/${file.path}`);
        }
    }

    /** List the files of a build a browser may load: all but its other runtimes' outputs. */
    static #browserFiles(manifest: PackageManifest, files: readonly PackageFile[]): PackageFile[] {
        const others = Object.values(manifest.outputs)
            .filter((output) => output.runtime !== "browser")
            .map((output) => `${output.directory}/`);

        return files.filter((file) => !others.some((directory) => file.path.startsWith(directory)));
    }

    /** Compose the browser features the build's views allow, as an installation without grants. */
    static #policy(manifest: PackageManifest): string {
        const views = Object.values(ViewDescription.list(manifest));

        return Sandbox.policy(Sandbox.merge(...views.map((view) => view.sandbox)));
    }

    /** Write the host's headers: the site's, the permissions, the caching and the media types. */
    async #writeHeaders(policy: string): Promise<void> {
        // read the site's own rules, written with its pages, none without them
        const path = join(this.directory, HEADERS_FILE);
        const own = await this.#environment.files.readTextFile(path).catch(FileSystemError.absent);

        // declare the builds immutable and each extension's media type, as the origin answers them
        const types = [...this.#types]
            .filter(([extension]) => extension !== "")
            .toSorted(([left], [right]) => left.localeCompare(right))
            .map(([extension, type]) => `/*${extension}\n  Content-Type: ${type}`);
        const rules = [
            `/*\n  Permissions-Policy: ${policy}`,
            `${BUILD_PATH}*\n${BUILD_HEADERS}`,
            ...types,
        ];
        const text = [...(own === undefined ? [] : [own.trimEnd()]), ...rules].join("\n");
        await this.#environment.files.writeTextFile(path, `${text}\n`);
    }
}
