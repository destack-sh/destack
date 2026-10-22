import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Egress } from "./egress.ts";

test("read back the installation an address writes, and no installation from other addresses", () => {
    // write an installation's address by its name and read the name and space back
    const name = "notes";
    const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e2");
    const address = Egress.address(name, spaceId);

    // read nothing from a binding's name: a space, three labels, swapped ones, or the punycode prefix
    const others = [
        "notes",
        spaceId,
        `${address}--${spaceId}`,
        `${spaceId}--${name}`,
        `xn--${spaceId}`,
    ];

    expect([
        Egress.installation(address),
        others.map((other) => Egress.installation(other)),
    ]).toEqual([{ name, spaceId }, [undefined, undefined, undefined, undefined, undefined]]);
});

test("split a request to the egress into its address and the request below it", () => {
    // split a package's address and a binding's name, and nothing outside the egress
    const scoped = Egress.of(
        new Request("http://127.0.0.1/.destack/egress/@destack/vault/calls?x=1"),
    );
    const named = Egress.of(new Request("http://127.0.0.1/.destack/egress/notes"));
    const outside = Egress.of(new Request("http://127.0.0.1/other"));

    expect([
        [scoped?.address, scoped?.request.url],
        [named?.address, named?.request.url],
        outside,
    ]).toEqual([
        ["@destack/vault", "http://127.0.0.1/calls?x=1"],
        ["notes", "http://127.0.0.1/"],
        undefined,
    ]);
});
