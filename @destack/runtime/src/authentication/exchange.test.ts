import { expect, test } from "@destack/test";
import { type Caller, principal } from "@destack/access";
import { Identifier } from "@destack/schema";
import { Exchange } from "./exchange.ts";

/** A person the installation acts for. */
const person = principal.user.reference("universe", "user-ada");
/** The installation acting for the person. */
const installation = principal.installation.reference("space-home", "installation-notes");
/** The grant the installation acts under. */
const grant = { id: Identifier.create("grant"), scope: person.id, permissions: [] };

test("ask the egress to act for a served person, or under the grant its delegates carry, and for nothing else", () => {
    // read the contexts of a person's call, a delegated call, an installation's own call and none
    const contexts = {
        person: Exchange.of(called({ subject: person })),
        delegated: Exchange.of(called({ subject: person, actors: [installation], grant })),
        installation: Exchange.of(called({ subject: installation })),
        outside: Exchange.of(undefined),
    };

    // write the asks and read them back as the egress does
    const asked = Exchange.ask(contexts.delegated);
    const read = Exchange.asking(new Request("https://egress.test", { headers: asked }));

    expect({ contexts, asked, read, none: Exchange.ask(contexts.installation) }).toEqual({
        contexts: {
            person: { subject: person },
            delegated: { subject: person, grant: grant.id },
            installation: undefined,
            outside: undefined,
        },
        asked: { [Exchange.header]: JSON.stringify({ subject: person, grant: grant.id }) },
        read: { subject: person, grant: grant.id },
        none: {},
    });
    expect(malformed).toThrow(`invalid ${Exchange.header}`);
});

/** Build a served caller without groups or contacts. */
function called(
    caller: Pick<Caller, "subject"> & Partial<Pick<Caller, "actors" | "grant">>,
): Caller {
    return { actors: [], groups: [], contacts: [], ...caller };
}

/** Read an ask with a header that is not JSON. */
function malformed() {
    return Exchange.asking(
        new Request("https://egress.test", { headers: { [Exchange.header]: "ada" } }),
    );
}
