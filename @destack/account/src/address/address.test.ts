import { expect, test } from "@destack/test";
import { PlatformDomain } from "./address.ts";

test("cover the platform's domains and the hostnames and wildcards below them, but no account's own", () => {
    expect(
        [
            "destack.app",
            "*.destack.space",
            "relay-eu.destack.computer",
            "notes.acme.com",
            "destack.app.acme.com",
            "notdestack.app",
        ].map((hostname) => PlatformDomain.covers(hostname)),
    ).toEqual([true, true, true, false, false, false]);
});
