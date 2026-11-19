import { dirname, extname, join } from "node:path";
import type { LocalBuildStore } from "@destack/build";
import { type Environment, FileSystemError } from "@destack/environment";
import type { PackageFile } from "@destack/package/file";
import {
    type BuildReader,
    ImportMap,
    type BuildImage,
    type PackageManifest,
    ViewDescription,
} from "@destack/package/manifest";
import { OriginPath, Sandbox } from "@destack/package";
import type { Digest } from "@destack/schema";
import { MachineError } from "../error/index.ts";
import { HEADERS_FILE, StaticHost, AssetIndex } from "./host.ts";

/** The headers of a dependency build's files, which never change under its digest. */
const BUILD_HEADERS = "  Cache-Control: public, max-age=31536000, immutable";

/** The files whose text refers to other files of the site: pages and stylesheets. */
const REFERRING = new Set([".html", ".css"]);

/** A disk path or a path below the asset or build path, where a page or stylesheet loads a file. */
const REFERENCE =
    /(?:\b(?:href|src)=["']|url\(["']?)(file:\/\/[^"'\s)<>]+|\/\.destack\/(?:asset|build)\/[^"'\s)<>?#]+)/gu;

/** A static view written out as files, as a static host serves them and a machine's origin does. */
export class StaticExport {
    /** The directory the files are written below. */
    readonly directory: string;
    /** The store with the view's build and every build its pages import. */
    readonly #builds: Pick<LocalBuildStore, "contents" | "imports" | "image">;
    /** The environment whose file system the export writes to. */
    readonly #environment: Environment<"files">;
    /** The media type of each written file's extension, which the host's headers declare. */
    readonly #types = new Map<string, string>();
    /** The paths written, each a file the host answers. */
    readonly #paths = new Set<string>();
    /** The addresses pages and stylesheets refer to, with the first file referring to each. */
    readonly #references = new Map<string, string>();

    /** Export into a directory from a store of builds. */
    constructor(
        directory: string,
        builds: Pick<LocalBuildStore, "contents" | "imports" | "image">,
        environment: Environment<"files">,
    ) {
        this.directory = directory;
        this.#builds = builds;
        this.#environment = environment;
    }

    /** Write an image's static view: its pages, its assets and the builds they import. */
    async write(
        image: Digest,
        view: string,
        linked: ReadonlyMap<string, BuildImage>,
    ): Promise<void> {
        // read the view's build, its public files and its static site, refusing a view without one
        const opened = await this.#builds.image(image);
        const reader = await this.#builds.contents(opened.root);
        const files = StaticHost.publicFiles(reader.manifest, await reader.files());
        const site = await StaticHost.open(reader.manifest, files, view, (file) =>
            reader.load(file.path),
        );
        if (site === undefined) {
            throw MachineError.of("VIEW_NOT_FOUND", `static view '${view}' of the build not found`);
        }

        // map the modules the pages import below the build path, as the origin does
        const closure = await this.#builds.imports(
            opened,
            "browser",
            `${OriginPath.build}/`,
            linked,
        );

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
        const assets = await AssetIndex.of(closure.builds, (digest) =>
            this.#builds.contents(digest),
        );
        for (const [name, asset] of assets.files) {
            await this.#copy(asset.reader, asset.file, `${OriginPath.asset}/${name}`);
        }

        // refuse a page or stylesheet referring to a disk path, or to an asset or build file the site lacks
        for (const [reference, path] of this.#references) {
            if (!this.#paths.has(reference)) {
                throw MachineError.of(
                    "SITE_INVALID",
                    `static file '${path}' is invalid: it refers to '${reference}', which the site does not serve`,
                );
            }
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

        // note the addresses a page or stylesheet refers to, by the first file referring to each
        if (REFERRING.has(extname(path))) {
            for (const [, reference = ""] of new TextDecoder()
                .decode(written)
                .matchAll(REFERENCE)) {
                if (!this.#references.has(reference)) {
                    this.#references.set(reference, path);
                }
            }
        }
        this.#paths.add(path);

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
            await this.#copy(reader, file, `${OriginPath.build}/${digest}/${file.path}`);
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
            `${OriginPath.build}/*\n${BUILD_HEADERS}`,
            ...types,
        ];
        const text = [...(own === undefined ? [] : [own.trimEnd()]), ...rules].join("\n");
        await this.#environment.files.writeTextFile(path, `${text}\n`);
    }
}
