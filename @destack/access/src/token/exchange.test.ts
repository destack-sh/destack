import { expect, test } from "@destack/test";
import { type Caller } from "../context/index.ts";
import { principal } from "../declare/index.ts";
import { schema } from "@destack/schema";
import { Exchange } from "./exchange.ts";

/** A person the installation acts for. */
const person = principal.user.reference("universe", "user-ada");
/** The installation acting for the person. */
const installation = principal.installation.reference("space-home", "installation-notes");
/** The grant the installation acts under. */
const grant = {
    id: schema.identifier("grant").parse("grant-019a0000-0000-7000-8000-000000000001"),
    scope: person.id,
};

test("ask the egress to act as a served person, or as a subject under the grants its delegates include, and as nothing else", () => {
    // read the contexts of a person's call, a delegated call, an installation's call and none
    const contexts = {
        person: Exchange.of(called({ subject: person })),
        delegated: Exchange.of(
            called({ subject: person, actors: [installation], grants: [grant] }),
        ),
        installation: Exchange.of(called({ subject: installation })),
        outside: Exchange.of(undefined),
    };

    // write the header and read it back as the egress does
    const asked = Exchange.ask(contexts.delegated);
    const read = Exchange.asking(new Request("https://egress.test", { headers: asked }));

    expect({ contexts, asked, read, none: Exchange.ask(contexts.installation) }).toEqual({
        contexts: {
            person: { subject: person, grants: [] },
            delegated: { subject: person, grants: [grant] },
            installation: undefined,
            outside: undefined,
        },
        asked: { [Exchange.header]: JSON.stringify({ subject: person, grants: [grant] }) },
        read: { subject: person, grants: [grant] },
        none: {},
    });
    expect(malformed).toThrow(`invalid ${Exchange.header}`);
});

/** Build a served caller without groups or contacts. */
function called(
    caller: Pick<Caller, "subject"> & Partial<Pick<Caller, "actors" | "grants">>,
): Caller {
    return { actors: [], groups: [], contacts: [], grants: [], ...caller };
}

/** Read a header that is not JSON. */
function malformed() {
    return Exchange.asking(
        new Request("https://egress.test", { headers: { [Exchange.header]: "ada" } }),
    );
}
