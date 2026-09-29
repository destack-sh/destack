import { mkdir, rename, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

import { plainTextFor } from "./text.ts";

/// Write the public Markdown and text representations of every content page.
export async function writePageSources(pages: readonly { markdownRoute: string; textRoute: string; markdown: string; }[], publicDirectory: string) {
    for (const page of pages) {
        await writeSource(publicDirectory, page.markdownRoute, page.markdown);
        await writeSource(
            publicDirectory,
            page.textRoute,
            plainTextFor(page.markdown),
        );
    }
}

/// Write one portable page representation below the public directory.
async function writeSource(publicDirectory: string, route: string, source: string) {
    const file = join(publicDirectory, route.slice(1));

    await mkdir(dirname(file), { recursive: true });
    const temporaryFile = `${file}.${process.pid}.tmp`;
    await writeFile(temporaryFile, `${source.trim()}\n`);
    await rename(temporaryFile, file);
}
