import { UpdateProcess } from "./process.ts";

/** The shutdown request from an authenticated updater, which ends this child process. */
const stopped = Promise.withResolvers<void>();
/** The server answering the updater's stop request. */
const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    fetch(request) {
        return registration.handle(request) ?? new Response("Not found", { status: 404 });
    },
});
/** The process registration served until shutdown. */
await using registration = await UpdateProcess.register(process.argv[2], server.port!, async () => {
    await server.stop();
    stopped.resolve();
});
console.log("ready");
await stopped.promise;
