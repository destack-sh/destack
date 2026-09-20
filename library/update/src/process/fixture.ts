import { UpdateProcess } from "./process.ts";

// retain a real child process until an authenticated updater requests shutdown
const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen() {} }, (request) => {
    return process.handle(request) ?? new Response("Not found", { status: 404 });
});
using process = await UpdateProcess.register(Deno.args[0], server.addr.port, async () => {
    await server.shutdown();
    Deno.exit(0);
});
console.log("ready");
await server.finished;
