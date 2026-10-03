import { expect, test } from "@destack/test";
import { principal } from "@destack/access";
import { PackageId } from "@destack/package";
import { aligned, schema } from "@destack/schema";
import { Authentication } from "./authentication.ts";
import { Lending } from "./lending.ts";

/** Settle a verification to "verified" or the message of its failure. */
function refused(verifying: Promise<unknown>): Promise<string> {
    return verifying.then(
        () => "verified",
        (error: { message: string }) => error.message,
    );
}

/** The space the lent-to installation serves. */
const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000b1");

/** The installation the caller calls. */
const installation = principal.installation.reference(
    scope,
    "installation-01996ab0-0000-7000-8000-0000000000b2",
);

/** The package of the called installation. */
const NOTES = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000b3");

/** An agent acting for the owner on lent authority, narrowed to reading notes. */
function agentCaller(now: number): Authentication {
    const owner = principal.user.reference("universe", "user-owner");

    return new Authentication({
        credential: { kind: "session", id: "session-1" },
        audience: NOTES,
        subject: owner,
        subjects: [owner],
        delegates: [
            { subject: principal.user.reference("universe", "user-agent"), authority: "lent" },
        ],
        permissions: [
            {
                packageId: NOTES,
                type: "note",
                name: "read",
                scope,
            },
        ],
        verifiedAt: now,
        expiresAt: now + 60_000,
    });
}

test("lend a caller's whole authority until it lapses, readable by the holder's key alone", async () => {
    // sign an agent's authority for the owner, lent to an installation
    const now = Date.now();
    const caller = agentCaller(now);
    const [lending, other] = [await Lending.generate(), await Lending.generate()];
    const kept = await Lending.import(await lending.export());
    const token = await lending.sign(caller, installation, scope, now);

    // read it within the hour through the kept key, and refuse it once lapsed, altered, or at another holder
    const parts = token.split(".");
    const altered = `${aligned(parts, 0).slice(0, -2)}AA.${aligned(parts, 1)}`;
    const { subject, subjects, delegates, permissions } = caller.claims;
    expect([
        await kept.verify(token, now + 60_000),
        await refused(lending.verify(token, now + 60 * 60_000)),
        await refused(lending.verify(altered, now)),
        await refused(other.verify(token, now)),
        await refused(lending.verify("forged", now)),
    ]).toEqual([
        {
            subject,
            subjects,
            delegates,
            permissions,
            installation,
            scope,
            expiresAt: now + 60 * 60_000,
        },
        "the lending lapsed",
        "invalid lending",
        "invalid lending",
        "invalid lending",
    ]);
});
