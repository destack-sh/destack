import { expect, test } from "@destack/test";
import { HostAddress } from "./socket.ts";

test("read public hosts apart from this computer, private networks and reserved use, however an address is written", () => {
    const hosts = [
        // public names and addresses
        "example.com",
        "8.8.8.8",
        "[2606:4700:4700::1111]",
        "64:ff9b::808:808",

        // loopback and local names, a trailing dot included
        "localhost",
        "localhost.",
        "home.localhost",
        "127.0.0.1",
        "[::1]",
        "[::]",

        // private, shared, link-local, reserved and multicast IPv4
        "0.1.2.3",
        "10.0.0.1",
        "100.64.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.168.1.1",
        "198.18.0.1",
        "224.0.0.1",
        "255.255.255.255",

        // IPv4 mapped or translated into IPv6, dotted or in hex
        "[::ffff:127.0.0.1]",
        "[::ffff:7f00:1]",
        "[::ffff:a9fe:a9fe]",
        "64:ff9b::a00:1",

        // unique local, link-local and multicast IPv6, and unreadable addresses
        "[fd00::1]",
        "[fe80::1]",
        "[ff02::1]",
        "[1::2::3]",
    ];

    expect(hosts.map((host) => [host, HostAddress.isPublic(host)])).toEqual([
        ["example.com", true],
        ["8.8.8.8", true],
        ["[2606:4700:4700::1111]", true],
        ["64:ff9b::808:808", true],
        ["localhost", false],
        ["localhost.", false],
        ["home.localhost", false],
        ["127.0.0.1", false],
        ["[::1]", false],
        ["[::]", false],
        ["0.1.2.3", false],
        ["10.0.0.1", false],
        ["100.64.0.1", false],
        ["169.254.169.254", false],
        ["172.16.0.1", false],
        ["192.168.1.1", false],
        ["198.18.0.1", false],
        ["224.0.0.1", false],
        ["255.255.255.255", false],
        ["[::ffff:127.0.0.1]", false],
        ["[::ffff:7f00:1]", false],
        ["[::ffff:a9fe:a9fe]", false],
        ["64:ff9b::a00:1", false],
        ["[fd00::1]", false],
        ["[fe80::1]", false],
        ["[ff02::1]", false],
        ["[1::2::3]", false],
    ]);
});
