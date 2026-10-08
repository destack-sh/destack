import { buildPackage, readDependencies } from "@destack/build";
import { DevelopmentServer, type MetadataOptions, type WebOptions } from "@destack/web/build";
import { cp, mkdir, readFile, rm } from "node:fs/promises";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import { watchContent } from "./content.ts";
import { prerenderRoutes } from "../src/view/content/generated/prerender-routes.ts";
import { posts } from "../src/view/content/generated/posts.ts";
import { Document } from "../src/view/content/document.ts";
import { tagline } from "../src/view/content/site.ts";

/** The site's public origin. */
const ORIGIN = "https://destack.sh";

/** The address security reports go to. */
const SECURITY_CONTACT = "mailto:florian@symbol.industries";

/** How long the security contact stays valid after a build: half a year, inside RFC 9116's one-year bound. */
const SECURITY_LIFETIME = 183 * 24 * 60 * 60 * 1000;

/** Absolute package directory for development and production builds. */
const directory = fileURLToPath(new URL("../", import.meta.url));

/** Build static pages or serve the same application with live updates. */
export async function run(command: string | undefined): Promise<void> {
    const application: WebOptions = {
        kind: "web",
        ssr: { runtime: "bun", emit: false },
        minify: true,
        app: "src/view/app.tsx",
        document: "src/view/document.tsx",
        entryServer: "src/view/entry-server.tsx",
        entryClient: "src/view/entry-client.tsx",
        renderMode: "async" as const,
        site: ORIGIN,
        prerender: { routes: prerenderRoutes, notFound: "/404" },
        metadata: await metadataOptions(),
    };

    // retain the development server until the process receives a shutdown signal
    if (command === "dev") {
        await using development = await DevelopmentServer.start({
            directory,
            application,
            server: { port: 3737, strictPort: true, watch: { ignored: ["**/.output/**"] } },
        });
        await using content = watchContent(directory, development);
        development.vite.printUrls();
        await new Promise<void>((resolve) => {
            process.once("SIGINT", resolve);
            process.once("SIGTERM", resolve);
        });
    }
    // publish only the browser output, keeping inspection and source files private
    else if (command === "build") {
        const build = await buildPackage({
            directory,
            outputs: { website: application },
            dependencies: await readDependencies(directory),
        });
        const output = join(directory, ".output");
        await rm(output, { recursive: true, force: true });
        await mkdir(output, { recursive: true });
        await build.write(join(output, "package"));
        await mkdir(join(output, "public"), { recursive: true });
        const browser = build.manifest.outputs["website-browser"];
        if (browser === undefined) {
            throw new Error("missing website-browser output in the build manifest");
        }
        await cp(join(output, "package", browser.directory), join(output, "public"), {
            recursive: true,
        });
    } else {
        throw new Error("expected build or dev");
    }
}

/** Publish the sitemap of every page, the crawler rules, blog feed, model guide, manifest and security contact. */
async function metadataOptions(): Promise<MetadataOptions> {
    // read each documentation page's title, summary and Markdown from its generated metadata
    const documentation = await Promise.all(
        prerenderRoutes
            .filter((route) => route.startsWith("/docs/"))
            .map(async (route) =>
                Document.parse(
                    JSON.parse(
                        await readFile(
                            join(directory, "public/_content", route, "index.json"),
                            "utf8",
                        ),
                    ),
                ),
            ),
    );

    return {
        sitemap: prerenderRoutes,
        robots: { rules: [{ userAgent: "*", allow: ["/"] }] },
        feed: {
            path: "/blog/feed.xml",
            home: "/blog/",
            title: "Destack blog",
            subtitle: "Updates around Destack.",
            author: { name: "Florian" },
            entries: posts.map((post) => ({
                title: post.title,
                path: post.route,
                published: post.date,
                summary: post.subtitle,
            })),
        },
        llms: {
            title: "Destack",
            summary: tagline,
            details: "Every page is also published as Markdown: add .md to its address.",
            sections: [
                {
                    title: "Docs",
                    links: documentation.map((document) => ({
                        title: document.title,
                        path: document.markdownRoute,
                        description: document.description,
                    })),
                },
                {
                    title: "Blog",
                    links: posts.map((post) => ({
                        title: post.title,
                        path: post.markdownRoute,
                        description: post.subtitle,
                    })),
                },
            ],
        },
        manifest: {
            name: "Destack",
            shortName: "Destack",
            description: tagline,
            display: "standalone",
            backgroundColor: "#0b2029",
            themeColor: "#0b2029",
            icons: [
                { src: "/brand/icon/icon-180.png", sizes: "180x180", type: "image/png" },
                { src: "/brand/icon/icon-256.png", sizes: "256x256", type: "image/png" },
                { src: "/brand/icon/icon-512.png", sizes: "512x512", type: "image/png" },
                { src: "/brand/favicon/favicon.svg", sizes: "any", type: "image/svg+xml" },
            ],
        },
        security: {
            contact: [SECURITY_CONTACT],
            expires: new Date(Date.now() + SECURITY_LIFETIME).toISOString(),
            preferredLanguages: ["en", "de"],
        },
    };
}
