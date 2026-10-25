import { join } from "node:path";
import type { Environment } from "@destack/environment";

/** A server module's request handler, rendering a page in a mode. */
export type PageHandler = (
    request: Request,
    options: { readonly renderMode: "async" },
) => Response | Promise<Response>;

/** Render requests and write their headers and complete bodies. */
export async function renderPages(
    handler: PageHandler,
    urls: string[],
    directory: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // write each page and its response, reporting each completed one on the standard output
    const { files, process } = environment;
    for (const [index, url] of urls.entries()) {
        const response = await handler(new Request(url), { renderMode: "async" });
        const body = new Uint8Array(await response.arrayBuffer());
        await files.writeFile(join(directory, `${index}.html`), body);
        await files.writeTextFile(
            join(directory, `${index}.json`),
            JSON.stringify({
                status: response.status,
                headers: Object.fromEntries(response.headers),
            }),
        );
        await process.stdout.write(`${JSON.stringify({ rendered: index })}\n`);
    }
}
