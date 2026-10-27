import { present } from "@destack/schema";

/** The ports URLs of each scheme connect to when they name none. */
const DEFAULT_PORTS: Readonly<Record<string, number>> = {
    "http:": 80,
    "https:": 443,
    "ws:": 80,
    "wss:": 443,
};

/** The private and reserved IPv4 networks by address and prefix (RFC 6890). */
const PRIVATE_IPV4: readonly (readonly [number, number])[] = [
    // this network, private, shared and loopback
    [0x00_00_00_00, 8],
    [0x0a_00_00_00, 8],
    [0x64_40_00_00, 10],
    [0x7f_00_00_00, 8],

    // link local, private, protocol assignments, private and benchmarking
    [0xa9_fe_00_00, 16],
    [0xac_10_00_00, 12],
    [0xc0_00_00_00, 24],
    [0xc0_a8_00_00, 16],
    [0xc6_12_00_00, 15],

    // multicast and reserved
    [0xe0_00_00_00, 3],
];

/** The 16-bit groups of an IPv6 address. */
const IPV6_GROUPS = 8;

/** The IPv6 prefix of IPv4 addresses mapped into IPv6 (RFC 4291 2.5.5.2). */
const IPV4_MAPPED = 0xff_ff;

/** The first group of the NAT64 prefix embedding IPv4 addresses (RFC 6052). */
const NAT64 = 0x00_64;

/** A TCP endpoint: a host name or IP address and a port. */
export interface HostAddress {
    /** The host name or IP address. */
    readonly hostname: string;
    /** The port. */
    readonly port: number;
}

/** The TCP endpoints URLs connect to. */
export const HostAddress = {
    /** Read the endpoint a URL connects to, at its scheme's default port unless it names one. */
    of(url: URL): HostAddress {
        const port = url.port === "" ? DEFAULT_PORTS[url.protocol] : Number(url.port);

        return { hostname: url.hostname, port: present(port, `the port of ${url.protocol} URLs`) };
    },

    /** Report whether a host name or IP address lies outside this computer and private networks. */
    isPublic(host: string): boolean {
        // read the address without brackets or a trailing dot
        const name = host
            .toLowerCase()
            .replace(/^\[|\]$/gu, "")
            .replace(/\.$/u, "");
        const ipv4 = ipv4Number(name);
        const ipv6 = name.includes(":") ? ipv6Groups(name) : undefined;

        // check an IPv4 address against the private networks
        if (ipv4 !== undefined) {
            return !PRIVATE_IPV4.some(([network, length]) => sameNetwork(ipv4, network, length));
        }
        // check an IPv6 address, an embedded IPv4 one as itself
        else if (ipv6 !== undefined) {
            return isPublicIpv6(ipv6);
        }
        // refuse an unreadable address
        else if (name.includes(":")) {
            return false;
        }

        // take a name outside localhost as public until resolved
        return name !== "localhost" && !name.endsWith(".localhost");
    },
};

/** Read a dotted IPv4 address as a number, absent for any other text. */
function ipv4Number(text: string): number | undefined {
    // require four decimal octets
    const octets = text.split(".");
    const isDotted =
        octets.length === 4 &&
        octets.every((octet) => /^\d{1,3}$/u.test(octet) && Number(octet) < 256);

    return isDotted ? octets.reduce((total, octet) => total * 256 + Number(octet), 0) : undefined;
}

/** Read an IPv6 address as its eight groups, absent for any other text. */
function ipv6Groups(text: string): number[] | undefined {
    // read an embedded IPv4 address in the last two groups
    const parts = text.split(":");
    const last = parts.at(-1) ?? "";
    const embedded = ipv4Number(last);
    const tail = embedded === undefined ? [] : [Math.floor(embedded / 65_536), embedded % 65_536];
    const hex = embedded === undefined ? parts : parts.slice(0, -1);

    // expand the one `::` to the missing zero groups
    const gap = hex.indexOf("");
    const written = hex.filter((part) => part !== "");
    const missing = IPV6_GROUPS - written.length - tail.length;
    const isValid =
        text.split("::").length <= 2 &&
        written.every((part) => /^[0-9a-f]{1,4}$/u.test(part)) &&
        (gap === -1 ? missing === 0 : missing > 0);
    if (!isValid) {
        return undefined;
    }
    const groups = written.map((part) => Number.parseInt(part, 16));

    return gap === -1
        ? [...groups, ...tail]
        : [
              ...hex
                  .slice(0, gap)
                  .filter((part) => part !== "")
                  .map((part) => Number.parseInt(part, 16)),
              ...Array.from({ length: missing }, () => 0),
              ...hex
                  .slice(gap)
                  .filter((part) => part !== "")
                  .map((part) => Number.parseInt(part, 16)),
              ...tail,
          ];
}

/** Report whether IPv6 groups lie outside loopback, private, link-local and multicast networks. */
function isPublicIpv6(groups: readonly number[]): boolean {
    // check an IPv4 address mapped or translated into IPv6 as itself
    const [first = 0, second = 0, , , , sixth = 0, seventh = 0, eighth = 0] = groups;
    const embedded = seventh * 65_536 + eighth;
    const isLeadingZero = groups.slice(0, 5).every((group) => group === 0);
    if ((isLeadingZero && sixth === IPV4_MAPPED) || (first === NAT64 && second === 0xff_9b)) {
        return !PRIVATE_IPV4.some(([network, length]) => sameNetwork(embedded, network, length));
    }

    // refuse unspecified, loopback and compatible addresses, unique local, link local and multicast
    const isCompatible = isLeadingZero && sixth === 0;
    const isUniqueLocal = (first & 0xfe_00) === 0xfc_00;
    const isLinkLocal = (first & 0xff_c0) === 0xfe_80;
    const isMulticast = (first & 0xff_00) === 0xff_00;

    return !isCompatible && !isUniqueLocal && !isLinkLocal && !isMulticast;
}

/** Report whether an IPv4 address lies in a network of a prefix length. */
function sameNetwork(address: number, network: number, length: number): boolean {
    const size = 2 ** (32 - length);

    return Math.floor(address / size) === Math.floor(network / size);
}

/** A Unix socket's endpoint: its path on this computer, as Bun's `unix` option has it. */
export interface UnixAddress {
    /** The socket's path. */
    readonly unix: string;
}

/** Where a stream socket connects or listens: a host and port over TCP, or a Unix socket. */
export type SocketAddress = HostAddress | UnixAddress;

/** How a connection protects its bytes: plain, in TLS, or plain until it starts TLS. */
export type SecureTransport = "off" | "on" | "starttls";

/** How a connection opens. */
export interface ConnectOptions {
    /** How it protects its bytes, plain by default and always for a Unix socket. */
    readonly secureTransport?: SecureTransport;
    /** The PEM certificates trusted as authorities instead of the system's. */
    readonly certificateAuthorities?: readonly string[];
    /** Whether to refuse a host resolving into this computer or a private network. */
    readonly isPublicOnly?: boolean;
}

/** A connection's byte streams, as the sockets of Cloudflare's `connect()` have them. */
export interface Socket {
    /** The bytes the peer sends. */
    readonly readable: ReadableStream<Uint8Array>;
    /** The bytes sent to the peer. */
    readonly writable: WritableStream<Uint8Array>;
    /** Settles once the connection opened, or fails as it could not. */
    readonly opened: Promise<void>;
    /** Settles once the connection closed. */
    readonly closed: Promise<void>;
    /** Close the connection. */
    readonly close: () => Promise<void>;
    /** Start TLS over a connection connected for it, answering the socket that replaces it. */
    readonly startTls: () => Socket;
    /** Let the process exit while the connection stays open, as Deno's `Conn.unref` does. */
    readonly unref: () => void;
}

/** Where a listener listens and what it accepts, as `Bun.listen` takes it. */
export type SocketListenOptions = (
    | {
          /** The host name it listens at, localhost by default. */
          readonly hostname?: string;
          /** The port it listens at, a free one for zero or none. */
          readonly port?: number;
          /** No Unix socket. */
          readonly unix?: never;
      }
    | {
          /** The Unix socket's path it listens at. */
          readonly unix: string;
          /** No host name. */
          readonly hostname?: never;
          /** No port. */
          readonly port?: never;
      }
) & {
    /** Take an accepted connection. */
    readonly accept: (socket: Socket) => void;
};

/** An open listener of stream sockets. */
export interface SocketListener {
    /** The address it listens at. */
    readonly address: SocketAddress;
    /** Stop listening, closing the connections it accepted. */
    readonly stop: () => Promise<void>;
    /** Let the process exit while it listens, as Deno's `Listener.unref` does. */
    readonly unref: () => void;
}

/** The readable of a socket end, with the controller its connection delivers through. */
export class SocketInbox {
    /** The bytes the end reads. */
    readonly readable: ReadableStream<Uint8Array>;
    /** The controller taking delivered bytes. */
    readonly controller: ReadableStreamDefaultController<Uint8Array>;

    /** Keep a readable and its controller. */
    private constructor(
        readable: ReadableStream<Uint8Array>,
        controller: ReadableStreamDefaultController<Uint8Array>,
    ) {
        this.readable = readable;
        this.controller = controller;
    }

    /** Open an empty inbox, calling back once its reader cancels. */
    static open(cancel: () => void): SocketInbox {
        let started: ReadableStreamDefaultController<Uint8Array> | undefined;
        const readable = new ReadableStream<Uint8Array>({
            start: (controller) => {
                started = controller;
            },
            cancel,
        });

        return new SocketInbox(readable, present(started, "the inbox's controller"));
    }
}

/** Write an address as people read it: its host and port, or the Unix socket's path. */
export function socketName(address: SocketAddress): string {
    return "unix" in address ? address.unix : `${address.hostname}:${address.port}`;
}

/** Refuse TLS toward a Unix socket, which has no server name to verify. */
export function refuseUnixTls(address: SocketAddress, options: ConnectOptions): void {
    if ("unix" in address && (options.secureTransport ?? "off") !== "off") {
        throw new TypeError("a Unix socket connection cannot use TLS");
    }
}
