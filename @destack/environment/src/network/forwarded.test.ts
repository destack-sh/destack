import { expect, test } from "@destack/test";
import { Forwarded, HostAddress } from "./index.ts";

test("read and write RFC 7239's forms: tokens, quoted IPv6 nodes, several hops, unknown parameters", () => {
    // read the RFC's examples and a quoted escape, skipping by and unknown parameters
    const parsed = [
        Forwarded.parse('for="_gazonk"'),
        Forwarded.parse('For="[2001:db8:cafe::17]:4711"'),
        Forwarded.parse("for=192.0.2.60;proto=http;by=203.0.113.43"),
        Forwarded.parse("for=192.0.2.43, for=198.51.100.17"),
        Forwarded.parse('host="notes--ada.destack.space";proto=https;secret="a\\"b"'),
        Forwarded.parse(null),
    ];

    // write a quoted IPv6 node and a token host, and read them back
    const written = Forwarded.format([
        { for: "[2001:db8::1]", host: "example.com", proto: "https" },
    ]);

    expect({ parsed, written, read: Forwarded.parse(written) }).toEqual({
        parsed: [
            [{ for: "_gazonk" }],
            [{ for: "[2001:db8:cafe::17]:4711" }],
            [{ for: "192.0.2.60", proto: "http" }],
            [{ for: "192.0.2.43" }, { for: "198.51.100.17" }],
            [{ host: "notes--ada.destack.space", proto: "https" }],
            [],
        ],
        written: 'for="[2001:db8::1]";host=example.com;proto=https',
        read: [{ for: "[2001:db8::1]", host: "example.com", proto: "https" }],
    });
});

test("replace a client's own header with the one hop its front door writes", () => {
    // send a forged header, then replace it as a front door does
    const request = new Request("https://example.com/", {
        headers: { forwarded: "for=10.0.0.1;host=other.example" },
    });
    const replaced = Forwarded.replace(request, { for: "192.0.2.43" });

    expect([Forwarded.first(replaced), replaced.headers.get("forwarded")]).toEqual([
        { for: "192.0.2.43" },
        "for=192.0.2.43",
    ]);
});

test("read and restore the host and protocol a forwarded request's client asked for, and keep a direct request as it is", () => {
    // forward a client's request to a machine's own host, and send another directly
    const forwarded = new Request(
        "http://eu-central--platform.destack.computer/bucket-1?list-type=2",
        {
            headers: { forwarded: "for=192.0.2.43;host=s3--x.destack.space;proto=https" },
        },
    );
    const direct = new Request("http://s3--x.localhost:4102/bucket-1");
    const restored = Forwarded.restore(forwarded);

    expect([
        Forwarded.host(forwarded),
        Forwarded.host(direct),
        restored.url,
        restored.headers.get("host"),
        Forwarded.restore(direct) === direct,
    ]).toEqual([
        "s3--x.destack.space",
        "s3--x.localhost:4102",
        "https://s3--x.destack.space/bucket-1?list-type=2",
        "s3--x.destack.space",
        true,
    ]);
});

test("keep the header a peer on this computer sent, and replace any other peer's with the peer itself", () => {
    // receive one forged header from each kind of peer
    const received = ["127.0.0.1", "::ffff:127.0.0.2", "::1", "192.0.2.43", "2001:db8::1"].map(
        (peer) => {
            const sent = new Request("https://example.com/", {
                headers: { forwarded: "for=10.0.0.1" },
            });

            return Forwarded.first(
                Forwarded.receive(sent, peer, (each) => HostAddress.isLoopback(each)),
            );
        },
    );

    expect(received).toEqual([
        { for: "10.0.0.1" },
        { for: "10.0.0.1" },
        { for: "10.0.0.1" },
        { for: "192.0.2.43" },
        { for: "2001:db8::1" },
    ]);
});

test("keep only the last hop a proxy on this computer appended, the client's own earlier hops dropped", () => {
    const appended = new Request("https://example.com/", {
        headers: { forwarded: "for=10.0.0.1;host=forged.example, for=192.0.2.43" },
    });
    const received = Forwarded.receive(appended, "127.0.0.1", (peer) =>
        HostAddress.isLoopback(peer),
    );

    expect(received.headers.get("forwarded")).toBe("for=192.0.2.43");
});

test("tell this computer's host names, localhost and below it and loopback addresses, from others", () => {
    expect(
        [
            "localhost",
            "notes.space.localhost",
            "127.0.0.1",
            "127.8.0.3",
            "[::1]",
            "localhost.example",
            "192.0.2.43",
            "example.com",
        ].map((name) => HostAddress.isLoopbackName(name)),
    ).toEqual([true, true, true, true, true, false, false, false]);
});

test("count an IPv4 client by its address and an IPv6 client by its /64, without ports or brackets", () => {
    expect(
        [
            "192.0.2.43",
            "192.0.2.43:4711",
            "[2001:db8:cafe:17:1::9]:4711",
            "2001:db8:cafe:17::1",
            "::ffff:192.0.2.7",
        ].map((address) => HostAddress.network(address)),
    ).toEqual([
        "192.0.2.43",
        "192.0.2.43",
        "2001:db8:cafe:17::/64",
        "2001:db8:cafe:17::/64",
        "192.0.2.7",
    ]);
});
