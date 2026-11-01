import { type PackageManifest, ViewDescription } from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { MachineError } from "../error/index.ts";

/** The file listing a static site's redirects, as Cloudflare's and Netlify's. */
const REDIRECTS_FILE = "_redirects";

/** The file listing a static site's headers by path, as Cloudflare's and Netlify's. */
const HEADERS_FILE = "_headers";

/** The redirect status a rule without one answers, as Cloudflare's default. */
const DEFAULT_REDIRECT_STATUS = 302;

/** The statuses a redirect answers with. */
const REDIRECT_STATUSES = new Set([301, 302, 303, 307, 308]);

/** A placeholder of a path pattern: a colon and a name of letters, digits and underscores. */
const PLACEHOLDER = /:([A-Za-z]\w*)/gu;

/** The name a pattern's `*` binds, as destinations write it. */
const SPLAT = "splat";

/** The page a static site answers a path with when it has no file for it, as Cloudflare's. */
const NOT_FOUND_FILE = "404.html";

/** A path pattern of Cloudflare's rules: `:name` takes one segment, one `*` any rest. */
class PathPattern {
    /** The pattern as written. */
    readonly text: string;
    /** The expression matching a whole path, binding each placeholder and the splat. */
    readonly #expression: RegExp;

    /** Compile a pattern, refusing one that does not start with a slash or has two splats. */
    constructor(text: string, file: string) {
        // refuse a relative pattern and a second splat
        if (!text.startsWith("/") || text.split("*").length > 2) {
            throw MachineError.of("SITE_RULE_INVALID", `invalid ${file} path: ${text}`);
        }

        // match each placeholder within one segment and the splat across segments
        const source = text
            .split(/(\*|:[A-Za-z]\w*)/u)
            .map((part) =>
                part === "*"
                    ? `(?<${SPLAT}>.*)`
                    : part.startsWith(":")
                      ? `(?<${part.slice(1)}>[^/]+)`
                      : part.replaceAll(/[.+?^${}()|[\]\\]/gu, "\\$&"),
            )
            .join("");
        this.text = text;
        try {
            this.#expression = new RegExp(`^${source}$`, "u");
        } catch (error) {
            throw MachineError.of("SITE_RULE_INVALID", `invalid ${file} path: ${text}`, {
                cause: error,
            });
        }
    }

    /** Match a path, answering what each placeholder and the splat took, none for no match. */
    match(pathname: string): Readonly<Record<string, string>> | undefined {
        const matched = this.#expression.exec(pathname);

        return matched === null ? undefined : { ...matched.groups };
    }
}

/** One redirect: the paths it matches, where it sends a match and with which status. */
interface Redirect {
    /** The paths it matches. */
    readonly source: PathPattern;
    /** Where it sends a match, `:splat` and each `:name` taking what the source matched. */
    readonly destination: string;
    /** The status it answers with. */
    readonly status: number;
}

/** One headers rule: the paths it matches, the headers it sets and those it detaches. */
interface HeadersRule {
    /** The paths it matches. */
    readonly pattern: PathPattern;
    /** The headers it sets, by lowercase name, values taking what the pattern matched. */
    readonly headers: Readonly<Record<string, string>>;
    /** The lowercase names of the headers earlier rules set that it removes. */
    readonly detached: readonly string[];
}

/** How a site answers a path: with a file, or by sending the visitor to the path it moved to. */
export type SiteAnswer =
    | {
          /** The path serves a file. */
          readonly kind: "file";
          /** The file. */
          readonly file: PackageFile;
      }
    | {
          /** The path moved. */
          readonly kind: "redirect";
          /** The path it moved to. */
          readonly path: string;
      };

/** A build's static site as Cloudflare's hosts it: its pages, redirects and headers. */
export class StaticHost {
    /** The directory of the site's pages in the build. */
    readonly directory: string;
    /** The build's public files, by path. */
    readonly files: ReadonlyMap<string, PackageFile>;
    /** The redirects, the first matching one applying. */
    readonly redirects: readonly Redirect[];
    /** The headers by path, every matching rule applying in order. */
    readonly headers: readonly HeadersRule[];

    /** Create a site of its pages, redirects and headers. */
    constructor(fields: Pick<StaticHost, "directory" | "files" | "redirects" | "headers">) {
        // set the pages and the configuration
        this.directory = fields.directory;
        this.files = fields.files;
        this.redirects = fields.redirects;
        this.headers = fields.headers;
    }

    /** Open the static site of a build's view: the route's, else the build's one. */
    static async open(
        manifest: PackageManifest,
        files: ReadonlyMap<string, PackageFile>,
        view: string | undefined,
        load: (file: PackageFile) => Promise<Uint8Array<ArrayBuffer>>,
    ): Promise<StaticHost | undefined> {
        // list each static view with the directory of its pages
        const sites = Object.values(manifest.outputs).flatMap((output) =>
            Object.entries(output.views)
                .filter(([, description]) => description.render.mode === "static")
                .map(([name]) => ({ name, directory: ViewDescription.pages(output, name) })),
        );

        // take the route's view, else the build's one, refusing two
        if (view === undefined && sites.length > 1) {
            throw MachineError.of("SITE_AMBIGUOUS", "a build serves at most one static view");
        }
        const directory =
            view === undefined
                ? sites[0]?.directory
                : sites.find((site) => site.name === view)?.directory;
        if (directory === undefined) {
            return undefined;
        }

        // read each host configuration file the pages directory has
        const read = async (name: string) => {
            const file = files.get(`${directory}/${name}`);

            return file === undefined ? undefined : new TextDecoder().decode(await load(file));
        };
        const [redirects, headers] = await Promise.all([read(REDIRECTS_FILE), read(HEADERS_FILE)]);

        return StaticHost.parse({ directory, files, redirects, headers });
    }

    /** Read a site from its files and its configuration's texts, refusing an invalid line. */
    static parse(fields: {
        readonly directory: string;
        readonly files: ReadonlyMap<string, PackageFile>;
        readonly redirects: string | undefined;
        readonly headers: string | undefined;
    }): StaticHost {
        return new StaticHost({
            directory: fields.directory,
            files: fields.files,
            redirects:
                fields.redirects === undefined ? [] : StaticHost.#redirects(fields.redirects),
            headers: fields.headers === undefined ? [] : StaticHost.#headers(fields.headers),
        });
    }

    /** The site's not-found page, absent without one. */
    get missing(): PackageFile | undefined {
        return this.files.get(`${this.directory}/${NOT_FOUND_FILE}`);
    }

    /** Find where a path redirects to and with which status, none when no rule matches. */
    redirect(pathname: string): { readonly location: string; readonly status: number } | undefined {
        for (const rule of this.redirects) {
            const matched = rule.source.match(pathname);
            if (matched !== undefined) {
                return {
                    location: StaticHost.#substitute(rule.destination, matched),
                    status: rule.status,
                };
            }
        }

        return undefined;
    }

    /** Collect the headers the rules matching a path set, later rules overriding and detaching. */
    headersOf(pathname: string): Record<string, string> {
        const collected: Record<string, string> = {};
        for (const rule of this.headers) {
            // apply each matching rule's headers, then remove those it detaches
            const matched = rule.pattern.match(pathname);
            if (matched === undefined) {
                continue;
            }
            for (const [name, value] of Object.entries(rule.headers)) {
                collected[name] = StaticHost.#substitute(value, matched);
            }
            for (const name of rule.detached) {
                delete collected[name];
            }
        }

        return collected;
    }

    /** Resolve a path as Cloudflare's auto-trailing-slash: a file, or where it moved. */
    resolve(pathname: string): SiteAnswer | undefined {
        // send a page's file to its canonical path, never serving the host configuration's files
        const at = (path: string) =>
            path === `/${REDIRECTS_FILE}` || path === `/${HEADERS_FILE}`
                ? undefined
                : this.files.get(`${this.directory}${path}`);
        if (pathname.endsWith("/index.html") && at(pathname) !== undefined) {
            return { kind: "redirect", path: pathname.slice(0, -"index.html".length) };
        } else if (pathname.endsWith(".html") && at(pathname) !== undefined) {
            return { kind: "redirect", path: pathname.slice(0, -".html".length) };
        }

        // serve a directory's index, else send a page's directory path to its name
        const trimmed = pathname.replace(/\/$/u, "");
        const index = at(`${trimmed}/index.html`);
        if (pathname.endsWith("/")) {
            const named = trimmed === "" ? undefined : at(`${trimmed}.html`);
            if (index !== undefined) {
                return { kind: "file", file: index };
            } else if (named !== undefined) {
                return { kind: "redirect", path: trimmed };
            }

            return undefined;
        }

        // serve an asset or a page by its name, else send a directory's name to its path
        const asset = at(pathname) ?? at(`${pathname}.html`);
        if (asset !== undefined) {
            return { kind: "file", file: asset };
        } else if (index !== undefined) {
            return { kind: "redirect", path: `${pathname}/` };
        }

        return undefined;
    }

    /** Parse a `_redirects` file: one `source destination [status]` per line. */
    static #redirects(text: string): Redirect[] {
        const lines = text
            .split("\n")
            .map((line) => line.trim())
            .filter((line) => line !== "" && !line.startsWith("#"));

        return lines.map((line) => {
            // split the source, the destination and the status
            const [source = "", destination = "", status, ...rest] = line.split(/\s+/u);
            const code = status === undefined ? DEFAULT_REDIRECT_STATUS : Number(status);
            if (destination === "" || rest.length > 0 || !REDIRECT_STATUSES.has(code)) {
                throw MachineError.of(
                    "SITE_RULE_INVALID",
                    `invalid ${REDIRECTS_FILE} line: ${line}`,
                );
            }

            return { source: new PathPattern(source, REDIRECTS_FILE), destination, status: code };
        });
    }

    /** Parse a `_headers` file: a path, then its indented header and detach lines. */
    static #headers(text: string): HeadersRule[] {
        const rules: {
            pattern: PathPattern;
            headers: Record<string, string>;
            detached: string[];
        }[] = [];
        for (const line of text
            .split("\n")
            .filter((each) => each.trim() !== "" && !each.trim().startsWith("#"))) {
            // start a rule at an unindented path
            const current = rules.at(-1);
            const trimmed = line.trim();
            const separator = trimmed.indexOf(":");
            if (!/^\s/u.test(line)) {
                rules.push({
                    pattern: new PathPattern(trimmed, HEADERS_FILE),
                    headers: {},
                    detached: [],
                });
            }
            // detach a header an earlier rule set
            else if (current !== undefined && trimmed.startsWith("!")) {
                current.detached.push(trimmed.slice(1).trim().toLowerCase());
            }
            // set a header on the rule above it
            else if (current !== undefined && separator > 0) {
                const name = trimmed.slice(0, separator).trim().toLowerCase();
                current.headers[name] = trimmed.slice(separator + 1).trim();
            }
            // refuse a header line outside a rule or without a name
            else {
                throw MachineError.of("SITE_RULE_INVALID", `invalid ${HEADERS_FILE} line: ${line}`);
            }
        }

        return rules;
    }

    /** Replace `:splat` and each `:name` in a text with what a pattern matched, encoded. */
    static #substitute(text: string, matched: Readonly<Record<string, string>>): string {
        return text.replaceAll(PLACEHOLDER, (written, name: string) => {
            const value = matched[name];

            return value === undefined
                ? written
                : value.split("/").map(encodeURIComponent).join("/");
        });
    }
}
