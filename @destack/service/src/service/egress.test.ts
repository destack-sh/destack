import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Egress } from "./egress.ts";

test("read back the installation an address writes, and no installation from other addresses", () => {
    // write an installation's address and read its identifiers back
    const installationId = schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-0000000000e1");
    const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e2");
    const address = Egress.address(installationId, spaceId);

    // read nothing from a binding's name, a space alone, three identifiers, or swapped identifiers
    const others = ["notes", spaceId, `${address}--${spaceId}`, `${spaceId}--${installationId}`];

    expect([
        Egress.installation(address),
        others.map((other) => Egress.installation(other)),
    ]).toEqual([{ installationId, spaceId }, [undefined, undefined, undefined, undefined]]);
});

test("split a request to the egress into its address and the request below it", () => {
    // split a package's service address and a binding's name, and nothing outside the egress
    const scoped = Egress.of(
        new Request("http://127.0.0.1/.destack/egress/@destack/audit/calls?x=1"),
    );
    const named = Egress.of(new Request("http://127.0.0.1/.destack/egress/notes"));
    const outside = Egress.of(new Request("http://127.0.0.1/other"));

    expect([
        [scoped?.address, scoped?.request.url],
        [named?.address, named?.request.url],
        outside,
    ]).toEqual([
        ["@destack/audit", "http://127.0.0.1/calls?x=1"],
        ["notes", "http://127.0.0.1/"],
        undefined,
    ]);
});
