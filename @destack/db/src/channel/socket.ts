import { createHash } from "node:crypto";
import { unlink } from "node:fs/promises";
import { createConnection, createServer, type Server, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { FileLock } from "@destack/fs";
import type { Channel } from "./channel.ts";

/** The hexadecimal digits of a path's digest naming its socket, short enough for the 104 bytes of a macOS socket path. */
const NAME_DIGITS = 16;

/** The wait before joining again while another party starts serving, about one process start. */
const REJOIN_MILLISECONDS = 20;

/** One listener of a channel. */
interface Listener {
    /** Receive another party's message. */
    readonly receive: (message: unknown) => void;
    /** Recheck after each delivery start. */
    readonly resume: (() => void) | undefined;
    /** Fail once delivery ends. */
    readonly fail: ((error: unknown) => void) | undefined;
}

/** Reach every process on one machine sharing a path, such as a SQLite file's writers, through a Unix socket. */
export function socketChannel(path: string): Channel<unknown> {
    return new SocketChannel(path);
}

/** A party of a socket channel: the one with its lock serves the socket, the others connect to it. */
class SocketChannel implements Channel<unknown> {
    /** The socket path. */
    readonly #socket: string;
    /** The lock path electing the serving party. */
    readonly #lock: string;
    /** The listeners in this process. */
    readonly #listeners = new Set<Listener>();
    /** The lines sent before joining. */
    #queued: string[] = [];
    /** The join in progress or done. */
    #joining: Promise<void> | undefined;
    /** The served socket and its connected parties, while this party serves. */
    #hub:
        | { readonly server: Server; readonly lock: FileLock; readonly peers: Set<Socket> }
        | undefined;
    /** The connection to the serving party, while another party serves. */
    #peer: Socket | undefined;

    /** Derive the socket and lock paths of a path. */
    constructor(path: string) {
        const name = createHash("sha256").update(path).digest("hex").slice(0, NAME_DIGITS);
        this.#socket = join(tmpdir(), `destack-${name}.sock`);
        this.#lock = join(tmpdir(), `destack-${name}.lock`);
    }

    /** Whether this party serves the socket or is connected to the party serving it. */
    get #isJoined(): boolean {
        return this.#hub !== undefined || this.#peer !== undefined;
    }

    /** Send a message to every other party, once joined. */
    notify(message: unknown): void {
        // send to the other parties, or queue until joined
        const line = `${JSON.stringify(message)}\n`;
        if (this.#isJoined) {
            this.#send(line);
        } else {
            this.#queued.push(line);
            this.#join();
        }
    }

    /** Receive the other parties' messages until stopped, resuming on each join. */
    listen(
        receive: (message: unknown) => void,
        resume?: () => void,
        fail?: (error: unknown) => void,
    ): () => void {
        // join, or resume at once when joined already
        const listener = { receive, resume, fail };
        this.#listeners.add(listener);
        if (this.#isJoined) {
            resume?.();
        } else {
            this.#join();
        }

        // leave once the last listener stops
        return () => {
            this.#listeners.delete(listener);
            if (this.#listeners.size === 0) {
                void this.#leave();
            }
        };
    }

    /** Join the channel once, sending the queued lines and resuming every listener. */
    #join(): void {
        this.#joining ??= this.#elect().then(
            () => {
                // send the queued lines, then let each listener recheck
                const queued = this.#queued;
                this.#queued = [];
                for (const line of queued) {
                    this.#send(line);
                }
                for (const listener of this.#listeners) {
                    listener.resume?.();
                }
            },
            (error: unknown) => this.#fail(error),
        );
    }

    /** Fail every listener. */
    #fail(error: unknown): void {
        for (const listener of this.#listeners) {
            listener.fail?.(error);
        }
    }

    /** Serve the socket when it has the lock, or connect to the party serving it. */
    async #elect(): Promise<void> {
        while (true) {
            // serve the socket, replacing one a stopped party left
            const lock = await FileLock.tryAcquire(this.#lock);
            if (lock !== undefined) {
                await unlink(this.#socket).catch((error: NodeJS.ErrnoException) => {
                    if (error.code !== "ENOENT") {
                        throw error;
                    }
                });
                const peers = new Set<Socket>();
                const server = createServer((peer) => {
                    // relay each connected party's lines to the others
                    peers.add(peer);
                    this.#read(peer, (line) => {
                        // deliver here, and forward to every other party
                        this.#deliver(line);
                        for (const other of peers) {
                            if (other !== peer) {
                                other.write(line);
                            }
                        }
                    });
                    // drop a party whose connection fails
                    peer.on("close", () => peers.delete(peer));
                    peer.on("error", () => peer.destroy());
                    peer.unref();
                });
                await new Promise<void>((resolve, reject) => {
                    server.once("error", reject);
                    server.listen(this.#socket, () => resolve());
                });

                // fail the listeners once serving fails, and never keep the process alive
                server.removeAllListeners("error");
                server.on("error", (error) => this.#fail(error));
                server.unref();
                this.#hub = { server, lock, peers };

                return;
            }

            // connect to the serving party, joining again when it goes away
            const peer = await connect(this.#socket);
            if (peer !== undefined) {
                this.#read(peer, (line) => this.#deliver(line));
                peer.on("error", () => peer.destroy());
                peer.unref();
                peer.on("close", () => {
                    this.#peer = undefined;
                    this.#joining = undefined;
                    if (this.#listeners.size > 0) {
                        this.#join();
                    }
                });
                this.#peer = peer;

                return;
            }

            // wait while another party starts serving
            await new Promise((resolve) => {
                setTimeout(resolve, REJOIN_MILLISECONDS);
            });
        }
    }

    /** Send one line to the other parties. */
    #send(line: string): void {
        // forward to the connected parties as their server, or to the server
        if (this.#hub !== undefined) {
            for (const peer of this.#hub.peers) {
                peer.write(line);
            }
        } else {
            this.#peer?.write(line);
        }
    }

    /** Deliver one line to this process's listeners. */
    #deliver(line: string): void {
        const message: unknown = JSON.parse(line);
        for (const listener of this.#listeners) {
            listener.receive(message);
        }
    }

    /** Read a connection's lines. */
    #read(socket: Socket, receive: (line: string) => void): void {
        // split the stream at newlines, keeping a partial line for the next chunk
        let buffered = "";
        socket.setEncoding("utf8");
        socket.on("data", (chunk: string) => {
            buffered += chunk;
            let end = buffered.indexOf("\n");
            while (end !== -1) {
                receive(buffered.slice(0, end + 1));
                buffered = buffered.slice(end + 1);
                end = buffered.indexOf("\n");
            }
        });
    }

    /** Stop serving or close the connection, letting another party serve. */
    async #leave(): Promise<void> {
        // wait for the join, then release whichever role it took
        await this.#joining;
        this.#joining = undefined;
        const hub = this.#hub;
        this.#hub = undefined;
        const peer = this.#peer;
        this.#peer = undefined;
        peer?.removeAllListeners("close");
        peer?.destroy();
        if (hub !== undefined) {
            for (const other of hub.peers) {
                other.destroy();
            }
            await new Promise<void>((resolve) => {
                hub.server.close(() => resolve());
            });
            await hub.lock.close();
        }
    }
}

/** Connect to a served socket, absent while no party serves it. */
function connect(path: string): Promise<Socket | undefined> {
    return new Promise((resolve, reject) => {
        const socket = createConnection(path);
        socket.once("connect", () => {
            socket.removeAllListeners("error");
            resolve(socket);
        });
        socket.once("error", (error: NodeJS.ErrnoException) => {
            // report a socket no party serves, and fail otherwise
            if (error.code === "ENOENT" || error.code === "ECONNREFUSED") {
                resolve(undefined);
            } else {
                reject(error);
            }
        });
    });
}
