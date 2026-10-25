import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { Pkce } from "./pkce.ts";

test("derive the S256 challenge of RFC 7636's example verifier, and draw 43-character verifiers", async () => {
    // derive the challenge Appendix B of RFC 7636 lists for its verifier
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const challenge = await Pkce.challenge(
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        world.crypto,
    );

    // draw two verifiers of 43 unpadded base64url characters, distinct from each other
    const [first, second] = [Pkce.verifier(world.random), Pkce.verifier(world.random)];
    const shape = /^[A-Za-z0-9_-]{43}$/u;
    expect([challenge, shape.test(first), shape.test(second), first === second]).toEqual([
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        true,
        true,
        false,
    ]);
});
