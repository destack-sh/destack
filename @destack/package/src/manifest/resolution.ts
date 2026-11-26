import { Digest, type DigestCrypto, present } from "@destack/schema";
import { DependencyName } from "../definition/index.ts";
import { ImportMap, RELATIVE_SPECIFIER } from "./output.ts";

/** The hex digits of a resolution digest in a module URL: 64 bits. */
const RESOLUTION_LENGTH = 16;

/** The media type of a module. */
export const MODULE_TYPE = "text/javascript";

/** A module script element: its attributes and its inline code. */
const MODULE_SCRIPT = /<script\b([^>]*\btype=["']module["'][^>]*)>([\s\S]*?)<\/script>/gu;

/** The source attribute of a script element. */
const SOURCE_ATTRIBUTE = /\bsrc=["']([^"']+)["']/u;

/** The URL prefixes of an image's builds and modules. */
export interface ModulePrefixes {
    /** The prefix of the builds' files, such as `/.destack/build/`. */
    readonly builds: string;
    /** The prefix of the builds' modules, such as `/.destack/module/`. */
    readonly modules: string;
}

/** A build among an image's modules. */
interface ResolvedBuild {
    /** The build's digest. */
    readonly digest: Digest;
    /** The prefix of its modules. */
    readonly prefix: string;
    /** The module URL of each specifier it imports. */
    readonly imports: Readonly<Record<string, string>>;
}

/**
 * An image's modules at URLs naming their build and its resolved dependencies.
 *
 * Each module's imports are rewritten to those URLs, which browsers load without an import map.
 * A page and its workers share each URL and its cache.
 * A module's URL changes once its build or the builds its imports resolve to change.
 */
export class ModuleResolution {
    /** The prefixes of the builds and their modules. */
    readonly prefixes: ModulePrefixes;
    /** The root build. */
    readonly root: Digest;
    /** Each build by its digest. */
    readonly #builds: ReadonlyMap<Digest, ResolvedBuild>;
    /** Each build by its segment, `<digest>.<resolution>`. */
    readonly #segments: ReadonlyMap<string, ResolvedBuild>;

    /** Hold an image's resolved builds. */
    private constructor(
        prefixes: ModulePrefixes,
        root: Digest,
        builds: ReadonlyMap<Digest, ResolvedBuild>,
    ) {
        // index the builds
        this.prefixes = prefixes;
        this.root = root;
        this.#builds = builds;
        this.#segments = new Map(
            [...builds.values()].map((build) => [
                build.prefix.slice(prefixes.modules.length, -1),
                build,
            ]),
        );
    }

    /** Resolve an image's map into module URLs. */
    static async of(
        map: ImportMap,
        root: Digest,
        prefixes: ModulePrefixes,
        dependencies: (build: Digest) => Promise<readonly string[]>,
        crypto: DigestCrypto,
    ): Promise<ModuleResolution> {
        // read each build's scope and hoisted imports
        const scopes = new Map<Digest, Readonly<Record<string, string>>>([[root, map.imports]]);
        for (const [prefix, scope] of Object.entries(map.scopes)) {
            const digest = Digest.parse(prefix.slice(prefixes.builds.length, -1));
            const named = new Set(await dependencies(digest));
            const provided = Object.entries(map.imports).filter(([specifier]) =>
                named.has(DependencyName.of(specifier)),
            );
            scopes.set(digest, {
                ...scopes.get(digest),
                ...Object.fromEntries(provided),
                ...scope,
            });
        }

        // place each build below its digest and its imports' digest
        const placed = new Map<Digest, string>();
        for (const [digest, scope] of scopes) {
            const resolution = await Digest.json(scope, crypto);
            placed.set(
                digest,
                `${prefixes.modules}${digest}.${resolution.slice(0, RESOLUTION_LENGTH)}/`,
            );
        }

        // move the imports to module URLs
        const moved = (target: string) => {
            const [digest = "", ...path] = target.slice(prefixes.builds.length).split("/");

            return `${present(placed.get(Digest.parse(digest)), `the build of '${target}'`)}${path.join("/")}`;
        };
        const builds = new Map<Digest, ResolvedBuild>();
        for (const [digest, scope] of scopes) {
            const imports = Object.fromEntries(
                Object.entries(scope).map(([specifier, target]) => [specifier, moved(target)]),
            );
            builds.set(digest, {
                digest,
                prefix: present(placed.get(digest), "the build's prefix"),
                imports,
            });
        }

        return new ModuleResolution(prefixes, root, builds);
    }

    /** Read the module URL of a build's file. */
    url(build: Digest, path: string): string {
        return `${present(this.#builds.get(build), `build '${build}' of the image`).prefix}${path}`;
    }

    /** Read the build and file of a module URL. */
    locate(url: string): { readonly build: Digest; readonly path: string } | undefined {
        // take the build by its module prefix's segment
        if (!url.startsWith(this.prefixes.modules)) {
            return undefined;
        }
        const [segment = "", ...path] = url.slice(this.prefixes.modules.length).split("/");
        const build = this.#segments.get(segment);

        return build === undefined ? undefined : { build: build.digest, path: path.join("/") };
    }

    /** Resolve a specifier a module imports to a module URL, absent for one the image lacks. */
    resolve(specifier: string, importer: string): string | undefined {
        // resolve a bare specifier through the importer's build, a relative one against the importer
        const located = present(this.locate(importer), `module '${importer}' of the image`);
        const build = present(this.#builds.get(located.build), "the module's build");
        const mapped = ImportMap.resolve(
            { imports: build.imports, scopes: {} },
            specifier,
            importer,
        );
        const isRelative = RELATIVE_SPECIFIER.test(specifier);

        return (
            mapped ?? (isRelative ? new URL(specifier, `file://${importer}`).pathname : undefined)
        );
    }

    /** Rewrite a module's bare imports to module URLs. */
    async rewrite(code: string, url: string): Promise<string> {
        const located = present(this.locate(url), `module '${url}' of the image`);
        const build = present(this.#builds.get(located.build), "the module's build");

        return await ImportMap.apply({ imports: build.imports, scopes: {} }, code, url);
    }

    /** Rewrite a page's module scripts to module URLs. */
    async document(html: string, base: string): Promise<string> {
        // read each module script once, in order
        const parts: string[] = [];
        let last = 0;
        for (const match of html.matchAll(MODULE_SCRIPT)) {
            const [whole, attributes = "", code = ""] = match;
            const source = SOURCE_ATTRIBUTE.exec(attributes)?.[1];
            const isRootPath =
                source?.startsWith(`${base}/`) === true &&
                !source.startsWith("//") &&
                !source.startsWith(this.prefixes.modules);
            const rootPath = isRootPath ? source.slice(base.length + 1) : undefined;

            // rewrite the script's source or code
            let script = whole;
            if (source !== undefined && rootPath !== undefined) {
                script = whole.replace(source, this.url(this.root, rootPath));
            } else if (source === undefined && code.trim() !== "") {
                const rewritten = await this.rewrite(code, this.url(this.root, ""));
                script = whole.replace(code, rewritten);
            }
            parts.push(html.slice(last, match.index), script);
            last = match.index + whole.length;
        }
        parts.push(html.slice(last));

        return parts.join("");
    }

    /** List the modules an entry imports statically, for preloading. */
    async preloads(
        entry: string,
        load: (url: string) => Promise<string | undefined>,
    ): Promise<string[]> {
        // follow each static import once
        const reached = new Set<string>();
        const pending = [entry];
        for (let url = pending.shift(); url !== undefined; url = pending.shift()) {
            // read a module of the image once
            const loaded =
                reached.has(url) || this.locate(url) === undefined ? undefined : await load(url);
            if (loaded === undefined) {
                continue;
            }
            reached.add(url);

            // reach each module it imports statically
            const code = await this.rewrite(loaded, url);
            const imported = await ImportMap.imports({ imports: {}, scopes: {} }, code, url);
            pending.push(...imported.statics);
        }

        return [...reached].filter((url) => url !== entry);
    }
}
