import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { principal } from "../declare/principal.ts";
import { AccessContext, Caller, KEY_ASSURANCE } from "./context.ts";
import { Elevation } from "./elevation.ts";

/** The time the caller authenticated at. */
const AUTHENTICATED_AT = 1_800_000_000_000;

/** The package a credential is verified for. */
const PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

/** An elevation asking for several factors within five minutes. */
const elevation: Elevation = { assurance: 2, maxAge: 5 * 60_000 };

test("meet an elevation until the moment its age ends, the moment until reports", () => {
    // read the elevation just before, at and after its age ends, and with too weak an assurance
    const assurance = { level: 2, authenticatedAt: AUTHENTICATED_AT };
    const ends = AUTHENTICATED_AT + elevation.maxAge;
    const read = (now: number, level = assurance.level) => ({
        meets: Elevation.meets(elevation, { ...assurance, level }, now),
        until: Elevation.until(elevation, { ...assurance, level }, now),
    });

    // end exactly at the moment until reports, so a stream deciding again then sees it lapse
    expect({
        before: read(ends - 1),
        at: read(ends),
        weak: read(AUTHENTICATED_AT, 1),
    }).toEqual({
        before: { meets: true, until: ends },
        at: { meets: false, until: undefined },
        weak: { meets: false, until: undefined },
    });
});

test("assure a principal's own code by its key, a person by their sign-in, and code acting for a person by nothing", () => {
    // read the assurance of an installation's own code, of its code acting for ada, and of ada's session
    const now = AUTHENTICATED_AT;
    const ada = principal.user.reference(Scope.universe.id, "user-ada");
    const notes = principal.installation.reference("space-home", "installation-notes");
    const grant = {
        id: schema.identifier("grant").parse("grant-01996ab0-0000-7000-8000-000000000002"),
        scope: ada.id,
    };
    const session = {
        kind: "session",
        id: "session-ada",
        audience: PACKAGE,
        verifiedAt: now,
        expiresAt: now + 60_000,
        assurance: { level: 2, authenticatedAt: now },
    };

    // meet no elevation for code acting for ada, whose own sign-in alone assures her
    expect({
        own: AccessContext.assurance(AccessContext.of(Caller.of(notes), now)),
        acting: AccessContext.assurance(
            AccessContext.of({ ...Caller.acting(ada, notes), grants: [grant] }, now),
        ),
        signedIn: AccessContext.assurance(
            AccessContext.of(Caller.of(ada), now, { credential: session }),
        ),
    }).toEqual({
        own: { level: KEY_ASSURANCE, authenticatedAt: now },
        acting: undefined,
        signedIn: { level: 2, authenticatedAt: now },
    });
});
