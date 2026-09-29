import { expect, test } from "@destack/test";
import { DOMAINS, HostAddress } from "../src/address/index.ts";

test("read the host address of a server name, and nothing for other shapes", () => {
    expect(
        [
            "macbook.flotothemoon.destack.computer",
            "a.macbook.flotothemoon.destack.computer",
            "macbook..destack.computer",
            "-macbook.flotothemoon.destack.computer",
            "notes.personal.flotothemoon.destack.space",
        ].map((name) => HostAddress.parse(name, DOMAINS.host)),
    ).toEqual([
        { host: "macbook", handle: "flotothemoon" },
        undefined,
        undefined,
        undefined,
        undefined,
    ]);
});
