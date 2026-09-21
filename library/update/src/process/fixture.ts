import { UpdateProcess } from "./process.ts";

// retain a real child process until an authenticated updater requests shutdown
const stopped = Promise.withResolvers<void>();
const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    fetch(request) {
        return registration.handle(request) ?? new Response("Not found", { status: 404 });
    },
});
await using registration = await UpdateProcess.register(process.argv[2], server.port!, async () => {
    await server.stop();
    stopped.resolve();
});
console.log("ready");
await stopped.promise;
