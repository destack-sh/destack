import { WorkerdEnvironment } from "../environment.ts";

/** The response the scenario reads, given by its address in the request's query. */
export default {
    /** Read the target's body decoded and as stored, answering each one's length and encoding. */
    async fetch(request: Request): Promise<Response> {
        // read the target twice through the isolate's network
        const target = new URL(request.url).searchParams.get("target") ?? "";
        const { network } = WorkerdEnvironment.system;
        const decoded = await network.fetch(new Request(target));
        const stored = await network.fetch(new Request(target), { decompress: false });

        // answer each body's length beside the encoding its response names
        return Response.json({
            decoded: [
                (await decoded.arrayBuffer()).byteLength,
                decoded.headers.get("content-encoding"),
            ],
            stored: [
                (await stored.arrayBuffer()).byteLength,
                stored.headers.get("content-encoding"),
            ],
        });
    },
};
