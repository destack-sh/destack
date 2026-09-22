import { writeFile } from "node:fs/promises";
import { join } from "node:path";

/** Render requests and write their headers and complete bodies. */
export async function renderPages(
    handler: (request: Request, options: { renderMode: "async" }) => Response | Promise<Response>,
    urls: string[],
    directory: string,
): Promise<void> {
    for (const [index, url] of urls.entries()) {
        const response = await handler(new Request(url), { renderMode: "async" });
        const body = new Uint8Array(await response.arrayBuffer());
        await writeFile(join(directory, `${index}.html`), body);
        await writeFile(
            join(directory, `${index}.json`),
            JSON.stringify({
                status: response.status,
                headers: Object.fromEntries(response.headers),
            }),
        );
        console.log(JSON.stringify({ rendered: index }));
    }
}
