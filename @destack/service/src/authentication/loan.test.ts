import { Caller, principal } from "@destack/access";
import { PackageId } from "@destack/package";
import { expect, refusal, test } from "@destack/test";
import { createLocalJWKSet, exportJWK, generateKeyPair, type JWTPayload, SignJWT } from "jose";
import { Loan } from "./loan.ts";

/** The person's home, which lends their authority. */
const HOME = "space-019f7480-0000-7000-8000-00000000000a";

/** The space the call goes to. */
const TARGET = "space-019f7480-0000-7000-8000-00000000000b";

/** The package receiving the call. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");

/** The method the Loan lends. */
const METHOD = "@app/pages/page.update";

test("lend a person's authority from their home for one method in another space for a minute, refusing another space, a home that is not theirs, a lapsed loan and another method", async () => {
    // sign as the home with its key, published as its key set
    const keys = await generateKeyPair("ES256", { extractable: true });
    const published = createLocalJWKSet({
        keys: [{ ...(await exportJWK(keys.publicKey)), alg: "ES256" }],
    });
    const sign = (payload: JWTPayload) =>
        new SignJWT(payload).setProtectedHeader({ alg: "ES256" }).sign(keys.privateKey);
    const now = Date.now();
    const token = await Loan.sign(
        sign,
        {
            home: HOME,
            person: "user-019f7480-0000-7000-8000-0000000000c1",
            space: TARGET,
            method: METHOD,
            actor: "installation-019f7480-0000-7000-8000-0000000000c2",
        },
        now,
    );

    // verify it at the target, at another space, as another person's home's and past its minute
    const verify = (space: string, home: string, at: number) =>
        Loan.verify(
            token,
            { keys: () => published, home: async () => home, space, audience: AUDIENCE },
            at,
        );
    const lent = await verify(TARGET, HOME, now);

    expect({
        claims: [
            lent.subject,
            lent.delegates,
            lent.credential?.scope,
            lent.loan?.method,
            Caller.isExchanged(lent),
        ],
        other: await refusal(verify(HOME, HOME, now)),
        elsewhere: await refusal(verify(TARGET, TARGET, now)),
        lapsed: await refusal(verify(TARGET, HOME, now + 2 * 60_000)),
        method: await refusal(
            Promise.resolve().then(() => Loan.require(lent, "@app/pages/page.delete")),
        ),
        same: await refusal(Promise.resolve().then(() => Loan.require(lent, METHOD))),
    }).toEqual({
        claims: [
            principal.user.reference("universe", "user-019f7480-0000-7000-8000-0000000000c1"),
            [
                {
                    subject: principal.installation.reference(
                        HOME,
                        "installation-019f7480-0000-7000-8000-0000000000c2",
                    ),
                    authority: "lent",
                },
            ],
            TARGET,
            METHOD,
            false,
        ],
        other: ["UNAUTHORIZED", "invalid loan"],
        elsewhere: ["UNAUTHORIZED", "only a person's home lends their authority to another space"],
        lapsed: ["UNAUTHORIZED", "invalid loan"],
        method: ["FORBIDDEN", `the loan lends ${METHOD}, not @app/pages/page.delete`],
        same: "done",
    });
});

test("ask the egress for a Loan of the person a call's context names, from a served call's delegated caller, and for none without a delegation", () => {
    // read the context of a delegated and an undelegated served caller
    expect({
        delegated: Loan.ask(METHOD, Loan.of(caller("delegated"))),
        undelegated: Loan.ask(METHOD, Loan.of(caller())),
        outside: Loan.ask(METHOD, Loan.of(undefined)),
        read: Loan.asking(
            new Request("https://egress.test", {
                headers: { [Loan.asked]: `${METHOD} delegated` },
            }),
        ),
    }).toEqual({
        delegated: { [Loan.asked]: `${METHOD} delegated` },
        undelegated: {},
        outside: {},
        read: { method: METHOD, delegation: "delegated" },
    });
});

/** A served caller with a delegation, or without one. */
function caller(delegation?: string): Pick<Caller, "delegation"> {
    return delegation === undefined ? {} : { delegation };
}
