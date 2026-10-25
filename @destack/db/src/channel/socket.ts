import { createHash } from "node:crypto";
import { join } from "node:path";
import type { Environment, FileLock, Socket, SocketListener } from "@destack/environment";
import { type Channel, HEARD, type Hearing } from "./channel.ts";

/** The hex digits of a path digest in its socket name, fitting macOS's 104-byte socket paths. */
const NAME_DIGITS = 16;

/** The wait before joining again while another party starts serving, about one process start. */
const REJOIN_MILLISECONDS = 20;

/** The encoder of the lines the parties send. */
const ENCODER = new TextEncoder();

/** A connected party: its socket and the writer sending it lines. */
interface Peer {
    /** The connection. */
    readonly socket: Socket;
    /** The writer sending lines over it. */
    readonly writer: WritableStreamDefaultWriter<Uint8Array>;
}

/** One listener of a channel. */
interface Listener {
    /** Receive another party's message. */
    readonly receive: (message: unknown) => void;
    /** Recheck once the party joins or joins again. */
    readonly resume: (() => void) | undefined;
    /** Fail once delivery ends. */
    readonly fail: ((error: unknown) => void) | undefined;
}

/** Connect every process on one machine sharing a path through a Unix socket. */
export function socketChannel(
    path: string,
    environment: Environment<"files" | "server">,
): Channel<unknown> {
    return new SocketChannel(path, environment);
}

/** Name the socket and lock files a path's channel meets at in a runtime directory. */
export function channelFiles(
    path: string,
    directory: string,
): { readonly socket: string; readonly lock: string } {
    const name = createHash("sha256").update(path).digest("hex").slice(0, NAME_DIGITS);

    return {
        socket: join(directory, `destack-${name}.sock`),
        lock: join(directory, `destack-${name}.lock`),
    };
}

/** A party of a socket channel: the lock holder serves the socket, the others connect to it. */
class SocketChannel implements Channel<unknown> {
    /** The path the parties share, such as a SQLite file. */
    readonly #path: string;
    /** The environment whose runtime directory and sockets the parties meet through. */
    readonly #environment: Environment<"files" | "server">;
    /** The listeners in this process. */
    readonly #listeners = new Set<Listener>();
    /** The lines sent before joining. */
    #queued: string[] = [];
    /** The join in progress or done. */
    #joining: Promise<void> | undefined;
    /** The served socket and its connected parties, while this party serves. */
    #hub:
        | { readonly listener: SocketListener; readonly lock: FileLock; readonly peers: Set<Peer> }
        | undefined;
    /** The connection to the serving party, while another party serves. */
    #peer: Peer | undefined;
    /** Whether the party closed the channel, which then refuses messages and listeners. */
    #isClosed = false;

    /** Meet the other parties of a path in an environment's runtime directory. */
    constructor(path: string, environment: Environment<"files" | "server">) {
        this.#path = path;
        this.#environment = environment;
    }

    /** Hear a token at once, as this channel sends no commit tokens. */
    hear(): Hearing {
        return HEARD;
    }

    /** Leave the channel, releasing the served socket and its lock or the connection to it. */
    async close(): Promise<void> {
        this.#isClosed = true;
        this.#listeners.clear();
        await this.#leave();
    }

    /** Whether this party serves the socket or is connected to the party serving it. */
    get #isJoined(): boolean {
        return this.#hub !== undefined || this.#peer !== undefined;
    }

    /** Send a message to every other party, once joined. */
    notify(message: unknown): void {
        // refuse a closed channel, then send to the other parties, joining unless joined
        this.#requireOpen();
        this.#send(`${JSON.stringify(message)}\n`);
        if (!this.#isJoined) {
            this.#join();
        }
    }

    /** Receive the other parties' messages until stopped, resuming on each join. */
    listen(
        receive: (message: unknown) => void,
        resume?: () => void,
        fail?: (error: unknown) => void,
    ): () => void {
        // refuse a closed channel, then join, or resume at once when joined already
        this.#requireOpen();
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
            (error: unknown) => {
                // fail the listeners, and let the next listen or notify join again
                this.#joining = undefined;
                this.#fail(error);
            },
        );
    }

    /** Refuse using a channel the party closed. */
    #requireOpen(): void {
        if (this.#isClosed) {
            throw new TypeError(`the channel of ${this.#path} is closed`);
        }
    }

    /** Fail every listener. */
    #fail(error: unknown): void {
        for (const listener of this.#listeners) {
            listener.fail?.(error);
        }
    }

    /** Serve the socket when it has the lock, or connect to the party serving it. */
    async #elect(): Promise<void> {
        // meet at the path's files in the person's runtime directory
        const directory = await this.#environment.files.runtimeDirectory();
        const files = channelFiles(this.#path, directory);
        while (true) {
            // serve the socket while the lock is taken
            const lock = await this.#environment.files.tryLock(files.lock, "exclusive");
            if (lock !== undefined) {
                await this.#serve(files.socket, lock);

                return;
            }

            // connect to the serving party
            const peer = await this.#connect(files.socket);
            if (peer !== undefined) {
                this.#follow(peer);

                return;
            }

            // wait while another party starts serving
            await this.#environment.clock.sleep(REJOIN_MILLISECONDS);
        }
    }

    /** Serve the socket under the lock and relay the parties' lines. */
    async #serve(socket: string, lock: FileLock): Promise<void> {
        // replace a socket a stopped party left, releasing the lock where serving fails
        const peers = new Set<Peer>();
        try {
            await this.#environment.files.remove(socket, { force: true });

            // relay each connected party's lines
            const listener = await this.#environment.server.socket({
                unix: socket,
                accept: (accepted) => {
                    accepted.unref();
                    this.#relay(SocketChannel.#peerOf(accepted), peers);
                },
            });

            // never keep the process from exiting
            listener.unref();
            this.#hub = { listener, lock, peers };
        } catch (error) {
            await lock.close();
            throw error;
        }
    }

    /** Relay a connected party's lines to this process and the other parties. */
    #relay(peer: Peer, peers: Set<Peer>): void {
        // relay each line to this process and every other party, dropping the party once it goes
        peers.add(peer);
        this.#read(peer, (line) => {
            this.#deliver(line);
            for (const other of peers) {
                if (other !== peer) {
                    SocketChannel.#write(other, line);
                }
            }
        })
            .catch((error: unknown) => this.#fail(error))
            .finally(() => peers.delete(peer));
    }

    /** Receive the serving party's lines and join again when it goes away. */
    #follow(peer: Peer): void {
        // deliver the serving party's lines here, joining again while listening once it goes
        this.#peer = peer;
        this.#read(peer, (line) => this.#deliver(line))
            .catch((error: unknown) => this.#fail(error))
            .finally(() => {
                if (this.#peer === peer) {
                    this.#peer = undefined;
                    this.#joining = undefined;
                    if (this.#listeners.size > 0) {
                        this.#join();
                    }
                }
            });
    }

    /** Send one line to the other parties. */
    #send(line: string): void {
        // forward to the connected parties as their server, to the server, or queue until joined
        if (this.#hub !== undefined) {
            for (const peer of this.#hub.peers) {
                SocketChannel.#write(peer, line);
            }
        } else if (this.#peer !== undefined) {
            SocketChannel.#write(this.#peer, line);
        } else {
            this.#queued.push(line);
        }
    }

    /** Deliver one line to this process's listeners. */
    #deliver(line: string): void {
        const message: unknown = JSON.parse(line);
        for (const listener of this.#listeners) {
            listener.receive(message);
        }
    }

    /** Read a party's lines until its connection ends, throwing what delivering a line threw. */
    async #read(peer: Peer, receive: (line: string) => void): Promise<void> {
        // read the stream's chunks, ending at a failed connection as at a closed one
        const reader = peer.socket.readable.getReader();
        const decoder = new TextDecoder();
        let buffered = "";
        try {
            while (true) {
                const chunk = await reader.read().catch(() => ({ done: true, value: undefined }));
                if (chunk.done) {
                    return;
                }

                // split the text at newlines, buffering a partial line for the next chunk
                buffered += decoder.decode(chunk.value, { stream: true });
                let end = buffered.indexOf("\n");
                while (end !== -1) {
                    receive(buffered.slice(0, end + 1));
                    buffered = buffered.slice(end + 1);
                    end = buffered.indexOf("\n");
                }
            }
        } finally {
            // close the connection however the read ended
            reader.releaseLock();
            await peer.socket.close();
        }
    }

    /** Stop serving or close the connection, letting another party serve. */
    async #leave(): Promise<void> {
        // wait for the join, keeping its role for a listener or a newer join that came meanwhile
        const joining = this.#joining;
        await joining;
        if (this.#joining !== joining || this.#listeners.size > 0) {
            return;
        }

        // release whichever role the join took
        this.#joining = undefined;
        const hub = this.#hub;
        this.#hub = undefined;
        const peer = this.#peer;
        this.#peer = undefined;
        await peer?.socket.close();
        if (hub !== undefined) {
            await hub.listener.stop();
            await hub.lock.close();
        }
    }

    /** Connect to a served socket, absent while no party serves it. */
    async #connect(path: string): Promise<Peer | undefined> {
        // connect, never keeping the process from exiting, and read a refusal as no party serving
        const socket = this.#environment.network.connect({ unix: path });
        socket.unref();
        try {
            await socket.opened;
        } catch (error) {
            if (error instanceof Error && "code" in error && error.code === "ConnectionRefused") {
                return undefined;
            }
            throw error;
        }

        return SocketChannel.#peerOf(socket);
    }

    /** Take a party's connection with a writer for its lines. */
    static #peerOf(socket: Socket): Peer {
        return { socket, writer: socket.writable.getWriter() };
    }

    /** Write a line to a party, closing its connection once writing fails. */
    static #write(peer: Peer, line: string): void {
        peer.writer.write(ENCODER.encode(line)).catch(() => peer.socket.close());
    }
}
