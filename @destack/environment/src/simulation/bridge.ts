import { once } from "node:events";
import { mkdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import http from "node:http";
import net from "node:net";
import { join } from "node:path";
import { TLSSocket } from "node:tls";

/** The curve of the bridge's keys: P-256, which every TLS client accepts and openssl makes fast. */
const CURVE = "ec_paramgen_curve:P-256";

/** The days the bridge's certificates are valid: one, longer than any world lives. */
const VALIDITY_DAYS = "1";

/** The line a client opens a tunnel with, such as `CONNECT forge.universe.test:443 HTTP/1.1`. */
const CONNECT_LINE = /^CONNECT ([^:\s]+):\d+ HTTP\/1\.[01]\r\n/u;

/** The answer opening a tunnel. */
const ESTABLISHED = "HTTP/1.1 200 Connection Established\r\n\r\n";

/** The answer to a tunnel whose host's certificate the bridge cannot issue. */
const UNREACHABLE = "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n";

/** The answer refusing anything but a tunnel. */
const REFUSED = "HTTP/1.1 405 Method Not Allowed\r\nconnection: close\r\n\r\n";

/** A key and a certificate in PEM. */
interface Certificate {
    /** The private key. */
    readonly key: string;
    /** The certificate. */
    readonly cert: string;
}

/** An HTTPS proxy on the host that forwards host processes' requests into a simulated world. */
export class HostBridge {
    /** The variables a host process reaches the world with: the proxy and the CA it trusts. */
    readonly variables: Readonly<Record<string, string>>;
    /** The directory of the CA's key and certificate. */
    readonly #directory: string;
    /** Send a request into the world. */
    readonly #fetch: (request: Request) => Promise<Response>;
    /** The proxy's listener. */
    readonly #server: net.Server;
    /** The HTTP server reading each tunnel's requests. */
    readonly #http: http.Server;
    /** Each host's certificate, issued on its first tunnel. */
    readonly #certificates = new Map<string, Promise<Certificate>>();

    /** Bridge host processes to a world that answers their requests through a fetch. */
    constructor(directory: string, fetch: (request: Request) => Promise<Response>) {
        // issue the world's CA
        this.#directory = directory;
        this.#fetch = fetch;
        mkdirSync(directory, { recursive: true });
        const ran = Bun.spawnSync(
            [
                "openssl",
                "req",
                "-x509",
                "-newkey",
                "ec",
                "-pkeyopt",
                CURVE,
                "-nodes",
                "-keyout",
                join(directory, "ca.key"),
                "-out",
                join(directory, "ca.pem"),
                "-days",
                VALIDITY_DAYS,
                "-subj",
                "/CN=destack world",
                "-addext",
                "basicConstraints=critical,CA:TRUE",
                "-addext",
                "keyUsage=critical,keyCertSign",
            ],
            { stdout: "ignore", stderr: "pipe" },
        );
        if (ran.exitCode !== 0) {
            const code = ran.exitCode;
            const message = ran.stderr.toString().trim();
            throw new Error(`openssl 'req' exited with status ${code}: ${message}`);
        }

        // read each tunnel's requests, forwarding them into the world
        this.#http = http.createServer((request, response) => {
            void this.#forward(request, response);
        });

        // listen on the host, giving processes the proxy's address and the CA
        this.#server = net.createServer((socket) => this.#tunnel(socket));
        this.#server.listen(0, "127.0.0.1");
        const address = this.#server.address();
        if (address === null || typeof address === "string") {
            throw new TypeError("the bridge listens on no port");
        }
        this.variables = {
            HTTPS_PROXY: `http://127.0.0.1:${address.port}`,
            NODE_EXTRA_CA_CERTS: join(directory, "ca.pem"),
        };
    }

    /** Stop listening and close every tunnel. */
    close(): void {
        this.#server.close();
        this.#http.closeAllConnections();
    }

    /** Open a tunnel to a host with its certificate, refusing anything but CONNECT. */
    #tunnel(socket: net.Socket): void {
        socket.once("data", (chunk: Buffer) => {
            // refuse anything but a tunnel
            const host = CONNECT_LINE.exec(chunk.toString("latin1"))?.[1];
            if (host === undefined) {
                socket.end(REFUSED);

                return;
            }

            // answer the tunnel once its host's certificate exists, then read its requests over TLS
            void this.#certificate(host).then(
                (certificate) => {
                    socket.write(ESTABLISHED);
                    const secure = new TLSSocket(socket, { isServer: true, ...certificate });
                    this.#http.emit("connection", secure);
                },
                // refuse the tunnel loudly, as its client reports a gateway's failure
                () => socket.end(UNREACHABLE),
            );
        });
    }

    /** Forward one request into the world and write its answer back. */
    async #forward(request: http.IncomingMessage, response: http.ServerResponse): Promise<void> {
        // read the request with its headers and body
        const method = request.method ?? "GET";
        const headers = new Headers();
        for (const [name, value] of Object.entries(request.headers)) {
            const values = typeof value === "string" ? [value] : (value ?? []);
            values.forEach((each) => headers.append(name, each));
        }
        const chunks: Uint8Array[] = [];
        request.on("data", (chunk: Uint8Array) => chunks.push(chunk));
        await once(request, "end");
        const body = chunks.length === 0 ? null : Buffer.concat(chunks);

        // send it into the world, answering a failure as a bad gateway
        const url = `https://${request.headers.host}${request.url}`;
        try {
            const answered = await this.#fetch(
                new Request(url, { method, headers, body, redirect: "manual" }),
            );
            const bytes = Buffer.from(await answered.arrayBuffer());
            response.writeHead(answered.status, Object.fromEntries(answered.headers));
            response.end(bytes);
        } catch (error) {
            response.writeHead(502);
            response.end(String(error));
        }
    }

    /** Issue a host's certificate from the world's CA once. */
    #certificate(host: string): Promise<Certificate> {
        // take one issued or issuing
        const known = this.#certificates.get(host);
        if (known !== undefined) {
            return known;
        }

        // issue it once
        const issued = this.#issue(host);
        this.#certificates.set(host, issued);

        return issued;
    }

    /** Issue a certificate for a host, signed by the world's CA. */
    async #issue(host: string): Promise<Certificate> {
        // sign a new key's certificate for the host
        const key = join(this.#directory, `${host}.key`);
        const cert = join(this.#directory, `${host}.pem`);
        const issuing = Bun.spawn(
            [
                "openssl",
                "req",
                "-x509",
                "-CA",
                join(this.#directory, "ca.pem"),
                "-CAkey",
                join(this.#directory, "ca.key"),
                "-newkey",
                "ec",
                "-pkeyopt",
                CURVE,
                "-nodes",
                "-keyout",
                key,
                "-out",
                cert,
                "-days",
                VALIDITY_DAYS,
                "-subj",
                `/CN=${host}`,
                "-addext",
                `subjectAltName=DNS:${host}`,
                "-addext",
                "basicConstraints=critical,CA:FALSE",
            ],
            { stdout: "ignore", stderr: "pipe" },
        );

        // refuse a failed issue with openssl's message
        const code = await issuing.exited;
        if (code !== 0) {
            const message = (await new Response(issuing.stderr).text()).trim();
            throw new Error(`openssl 'req' exited with status ${code}: ${message}`);
        }

        return { key: await readFile(key, "utf8"), cert: await readFile(cert, "utf8") };
    }
}
