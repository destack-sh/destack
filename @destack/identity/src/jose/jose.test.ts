import { TEST_START, testSimulation } from "@destack/test/world";
import { expect, refusal, test } from "@destack/test";
import { Base64Url } from "./base64.ts";
import { Jwe } from "./jwe.ts";
import { Jwk } from "./jwk.ts";
import { Jwks } from "./jwks.ts";
import { Jws } from "./jws.ts";
import { Jwt, type JwtOptions } from "./jwt.ts";

/** The P-256 key of RFC 7515 Appendix A.3. */
const ES256_KEY = {
    kty: "EC",
    crv: "P-256",
    ["x"]: "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
    ["y"]: "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0",
};

/** The JWS of RFC 7515 Appendix A.3, signed with ES256. */
const ES256_JWS =
    "eyJhbGciOiJFUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ.DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";

/** The Ed25519 key of RFC 8037 Appendix A.1. */
const ED25519_KEY = {
    kty: "OKP",
    crv: "Ed25519",
    ["d"]: "nWGxne_9WmC6hEr0kuwsxERJxWl7MmkZcDusAxyuf2A",
    ["x"]: "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo",
};

/** The JWS of RFC 8037 Appendix A.4. */
const ED25519_JWS =
    "eyJhbGciOiJFZERTQSJ9.RXhhbXBsZSBvZiBFZDI1NTE5IHNpZ25pbmc.hgyY0il_MGCjP0JzlnLWG1PPOt7-09PGcvMg3AIbQR6dWbhijcNR4ki4iylGjg5BhVsPt9g7sVvpAr_MuM0KAg";

test("verify the ES256 JWS of RFC 7515 A.3 and sign and verify the Ed25519 JWS of RFC 8037 A.4 exactly", async () => {
    // verify the ES256 example with its public key
    const world = testSimulation().environment("test");
    const es256 = await Jws.importJwk(ES256_KEY, "ES256", world.crypto);
    const verified = await Jws.verify(ES256_JWS, es256, ["ES256"], world.crypto);

    // sign the Ed25519 example's payload again, and verify it with the public half
    const { ["d"]: _private, ...edwards } = ED25519_KEY;
    const signer = await Jws.importJwk(ED25519_KEY, "EdDSA", world.crypto);
    const payload = new TextEncoder().encode("Example of Ed25519 signing");
    const signed = await Jws.sign(payload, { alg: "EdDSA" }, signer, world.crypto);
    const verifier = await Jws.importJwk(edwards, "EdDSA", world.crypto);
    const opened = await Jws.verify(ED25519_JWS, verifier, ["EdDSA"], world.crypto);

    expect({
        header: verified.header,
        payload: new TextDecoder().decode(verified.payload),
        signed,
        opened: new TextDecoder().decode(opened.payload),
        thumbprint: await Jwk.thumbprint(edwards, world.crypto),
    }).toEqual({
        header: { alg: "ES256" },
        payload: '{"iss":"joe",\r\n "exp":1300819380,\r\n "http://example.com/is_root":true}',
        signed: ED25519_JWS,
        opened: "Example of Ed25519 signing",
        thumbprint: "kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k",
    });
});

test("refuse an unsigned token, an algorithm other than the key's, a key of another identifier and a forged signature", async () => {
    // sign a token with a P-256 key named in a key set
    const world = testSimulation().environment("test");
    const pair = await world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const published = { ...(await Jwk.export(pair.publicKey, world.crypto)), kid: "key-1" };
    const keys = Jwks.local({ keys: [Jwk.parse(published)] }, world.crypto);
    const signed = await Jwt.sign(
        { sub: "ada" },
        { alg: "ES256", kid: "key-1" },
        pair.privateKey,
        world.crypto,
    );
    const [header, payload, signature] = signed.split(".");

    // forge an unsigned token, another algorithm, another key identifier and another payload
    const unsigned = `${Base64Url.encode('{"alg":"none"}')}.${payload}.`;
    const hmac = `${Base64Url.encode('{"alg":"HS256","kid":"key-1"}')}.${payload}.${signature}`;
    const other = `${Base64Url.encode('{"alg":"ES256","kid":"key-2"}')}.${payload}.${signature}`;
    const forged = `${header}.${Base64Url.encode('{"sub":"eve"}')}.${signature}`;
    const verify = (token: string) => Jws.verify(token, keys, ["ES256", "HS256"], world.crypto);

    expect({
        accepted: Jwt.claims(signed),
        unsigned: await refusal(verify(unsigned)),
        hmac: await refusal(verify(hmac)),
        other: await refusal(verify(other)),
        forged: await refusal(verify(forged)),
    }).toEqual({
        accepted: { sub: "ada" },
        unsigned: ["UNAUTHORIZED", "ALGORITHM_REFUSED", "the algorithm none is refused"],
        hmac: ["UNAUTHORIZED", "KEY_NOT_FOUND", "no key of the set signed the token"],
        other: ["UNAUTHORIZED", "KEY_NOT_FOUND", "no key of the set signed the token"],
        forged: ["UNAUTHORIZED", "SIGNATURE_INVALID", "the token's signature is invalid"],
    });
});

test("check a token's expiry, start, issuer and audience at the time of the check, within the clock tolerance", async () => {
    // sign claims valid from a minute after the start for an hour
    const world = testSimulation().environment("test");
    const secret = new Uint8Array(32).fill(1);
    const seconds = TEST_START / 1000;
    const claims = { iss: "issuer", aud: ["one", "two"], nbf: seconds + 60, exp: seconds + 3660 };
    const token = await Jwt.sign(claims, { alg: "HS256" }, secret, world.crypto);
    const check = (now: number, issuer = "issuer", audience = "two") =>
        Jwt.verify(
            token,
            secret,
            { algorithms: ["HS256"], now, issuer, audience, clockTolerance: 5 },
            world.crypto,
        ).then(({ claims: verified }) => verified);

    expect({
        early: await refusal(check(TEST_START + 50_000)),
        tolerated: await check(TEST_START + 56_000),
        expired: await refusal(check(TEST_START + 3_665_000)),
        issuer: await refusal(check(TEST_START + 120_000, "other")),
        audience: await refusal(check(TEST_START + 120_000, "issuer", "three")),
    }).toEqual({
        early: ["UNAUTHORIZED", "TOKEN_NOT_YET_VALID", "the token is not valid yet"],
        tolerated: claims,
        expired: ["UNAUTHORIZED", "TOKEN_EXPIRED", "the token expired"],
        issuer: ["UNAUTHORIZED", "ISSUER_REFUSED", "the token comes from another issuer"],
        audience: ["UNAUTHORIZED", "AUDIENCE_REFUSED", "the token is for another audience"],
    });
});

test("encrypt to an AES-GCM key and to an ECDH key, opening each with its key alone", async () => {
    // encrypt one plaintext to a wrapping key and to a recipient's key pair
    const world = testSimulation().environment("test");
    const plaintext = new Uint8Array([1, 2, 3]);
    const wrapping = await world.crypto.generateKey({ name: "AES-GCM", length: 256 }, false, [
        "encrypt",
        "decrypt",
    ]);
    const recipient = await world.crypto.generateKey({ name: "ECDH", namedCurve: "P-256" }, false, [
        "deriveBits",
    ]);
    const wrapped = await Jwe.encrypt(
        plaintext,
        { alg: "A256GCMKW", enc: "A256GCM", kid: "1" },
        wrapping,
        world,
    );
    const agreed = await Jwe.encrypt(
        plaintext,
        { alg: "ECDH-ES+A256KW", enc: "A256GCM" },
        recipient.publicKey,
        world,
    );
    const stranger = await world.crypto.generateKey({ name: "ECDH", namedCurve: "P-256" }, false, [
        "deriveBits",
    ]);

    expect({
        wrapped: [...(await Jwe.decrypt(wrapped, wrapping, world.crypto)).plaintext],
        agreed: [...(await Jwe.decrypt(agreed, recipient.privateKey, world.crypto)).plaintext],
        header: Jwe.header(wrapped)["kid"],
        stranger: await refusal(Jwe.decrypt(agreed, stranger.privateKey, world.crypto)),
    }).toEqual({
        wrapped: [1, 2, 3],
        agreed: [1, 2, 3],
        header: "1",
        stranger: [
            "INTERNAL_SERVER_ERROR",
            "DECRYPTION_FAILED",
            "ciphertext authentication failed",
        ],
    });
});

test("fetch a remote key set once while it is fresh, again once it is stale, and again for an unknown key after the cooldown", async () => {
    // serve a key set that starts with one key and later gains another
    await using simulation = testSimulation();
    const world = simulation.environment("verifier");
    const pair = await world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
        "sign",
        "verify",
    ]);
    const exported = await Jwk.export(pair.publicKey, world.crypto);
    const published = [{ ...exported, kid: "key-1" }];
    let fetches = 0;
    simulation.network.server("issuer").http({
        hostname: "issuer.test",
        port: 443,
        fetch: () => {
            fetches += 1;

            return Response.json({ keys: published });
        },
    });
    const keys = Jwks.remote(
        new URL("https://issuer.test/keys"),
        {
            fetch: (request) => world.network.fetch(request),
            maxAge: 60_000,
            cooldown: 10_000,
            timeout: 5_000,
        },
        world,
    );

    // look up the known key twice, then after it went stale, then an unknown key at once and after the cooldown
    const lookups: (string | number)[] = [];
    const lookup = async (kid: string) => {
        const found = await keys({ alg: "ES256", kid }).then(
            (each) => each.length,
            (error: Error) => error.message,
        );
        lookups.push(found, fetches);
    };
    await lookup("key-1");
    await lookup("key-1");
    await simulation.clock.sleep(TEST_START + 61_000 - simulation.clock.now());
    await simulation.idle();
    await lookup("key-1");
    published.push({ ...exported, kid: "key-2" });
    await lookup("key-2");
    await simulation.clock.sleep(TEST_START + 72_000 - simulation.clock.now());
    await simulation.idle();
    await lookup("key-2");

    expect(lookups).toEqual([1, 1, 1, 1, 1, 2, "no key of the set signed the token", 2, 1, 3]);
});

/** The payload of RFC 7515 Appendices A.1 and A.2. */
const EXAMPLE_PAYLOAD =
    "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";

/** The HMAC key of RFC 7515 Appendix A.1. */
const HS256_KEY = {
    kty: "oct",
    ["k"]: "AyM1SysPpbyDfgZld3umj1qzKObwVMkoqQ-EstJQLr_T-1qS0gZH75aKtMN3Yj0iPS4hcgUuTwjAzZr1Z9CAow",
};

/** The JWS of RFC 7515 Appendix A.1, signed with HS256. */
const HS256_JWS = `eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9.${EXAMPLE_PAYLOAD}.dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk`;

/** The RSA key of RFC 7515 Appendix A.2. */
const RS256_KEY = {
    kty: "RSA",
    ["n"]: "ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp-Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOWvG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUsLA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9kG4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2PAcDTW9gb54h4FRWyuXpoQ",
    ["e"]: "AQAB",
    ["d"]: "Eq5xpGnNCivDflJsRQBXHx1hdR1k6Ulwe2JZD50LpXyWPEAeP88vLNO97IjlA7_GQ5sLKMgvfTeXZx9SE-7YwVol2NXOoAJe46sui395IW_GO-pWJ1O0BkTGoVEn2bKVRUCgu-GjBVaYLU6f3l9kJfFNS3E0QbVdxzubSu3Mkqzjkn439X0M_V51gfpRLI9JYanrC4D4qAdGcopV_0ZHHzQlBjudU2QvXt4ehNYTCBr6XCLQUShb1juUO1ZdiYoFaFQT5Tw8bGUl_x_jTj3ccPDVZFD9pIuhLhBOneufuBiB4cS98l2SR_RQyGWSeWjnczT0QU91p1DhOVRuOopznQ",
    ["p"]: "4BzEEOtIpmVdVEZNCqS7baC4crd0pqnRH_5IB3jw3bcxGn6QLvnEtfdUdiYrqBdss1l58BQ3KhooKeQTa9AB0Hw_Py5PJdTJNPY8cQn7ouZ2KKDcmnPGBY5t7yLc1QlQ5xHdwW1VhvKn-nXqhJTBgIPgtldC-KDV5z-y2XDwGUc",
    ["q"]: "uQPEfgmVtjL0Uyyx88GZFF1fOunH3-7cepKmtH4pxhtCoHqpWmT8YAmZxaewHgHAjLYsp1ZSe7zFYHj7C6ul7TjeLQeZD_YwD66t62wDmpe_HlB-TnBA-njbglfIsRLtXlnDzQkv5dTltRJ11BKBBypeeF6689rjcJIDEz9RWdc",
    dp: "BwKfV3Akq5_MFZDFZCnW-wzl-CCo83WoZvnLQwCTeDv8uzluRSnm71I3QCLdhrqE2e9YkxvuxdBfpT_PI7Yz-FOKnu1R6HsJeDCjn12Sk3vmAktV2zb34MCdy7cpdTh_YVr7tss2u6vneTwrA86rZtu5Mbr1C1XsmvkxHQAdYo0",
    dq: "h_96-mK1R_7glhsum81dZxjTnYynPbZpHziZjeeHcXYsXaaMwkOlODsWa7I9xXDoRwbKgB719rrmI2oKr6N3Do9U0ajaHF-NKJnwgjMd2w9cjz3_-kyNlxAr2v4IKhGNpmM5iIgOS1VZnOZ68m6_pbLBSp3nssTdlqvd0tIiTHU",
    qi: "IYd7DHOhrWvxkwPQsRM2tOgrjbcrfvtQJipd-DlcxyVuuM9sQLdgjVk2oy26F0EmpScGLq2MowX7fhd_QJQ3ydy5cY7YIBi87w93IKLEdfnbJtoOPLUW0ITrJReOgo1cq9SbsxYawBgfp_gh6A5603k2-ZQwVK0JKSHuLFkuQ3U",
};

/** The JWS of RFC 7515 Appendix A.2, signed with RS256. */
const RS256_JWS = `eyJhbGciOiJSUzI1NiJ9.${EXAMPLE_PAYLOAD}.cC4hiUPoj9Eetdgtv3hF80EGrhuB__dzERat0XF9g2VtQgr9PJbu3XOiZj5RZmh7AAuHIm4Bh-0Qc_lF5YKt_O8W2Fp5jujGbds9uJdbF9CUAr7t1dnZcAcQjbKBYNX4BAynRFdiuB--f_nZLgrnbyTyWzO75vRK5h6xBArLIARNPvkSjtQBMHlb1L07Qe7K0GarZRmB_eSN9383LcOLn6_dO--xi12jzDwusC-eOkHWEsqtFZESc6BfI7noOPqvhJ1phCnvWh6IeYI2w9QOYEUipUTI8np6LbgGY9Fs98rqVt5AXLIhWkWywlVmtVrBp0igcN_IoypGlUPQGe77Rw`;

test("verify the HS256 JWS of RFC 7515 A.1, sign the RS256 JWS of A.2 exactly, and verify both with their keys", async () => {
    // sign the RS256 example's header and payload again with its key
    const world = testSimulation().environment("test");
    const payload = Base64Url.decode(EXAMPLE_PAYLOAD);
    const secret = Base64Url.decode(HS256_KEY.k);
    const rsa = await Jws.importJwk(RS256_KEY, "RS256", world.crypto);
    const { ["d"]: _d, ["p"]: _p, ["q"]: _q, dp: _dp, dq: _dq, qi: _qi, ...rsaPublic } = RS256_KEY;
    const signed = await Jws.sign(payload, { alg: "RS256" }, rsa, world.crypto);

    // verify both examples
    const verifier = await Jws.importJwk(rsaPublic, "RS256", world.crypto);
    const verified = await Promise.all([
        Jws.verify(HS256_JWS, secret, ["HS256"], world.crypto),
        Jws.verify(RS256_JWS, verifier, ["RS256"], world.crypto),
    ]);

    expect({
        signed,
        verified: verified.map(({ header }) => header),
    }).toEqual({
        signed: RS256_JWS,
        verified: [{ typ: "JWT", alg: "HS256" }, { alg: "RS256" }],
    });
});

test("sign with an ES384 key and an RS256 key a PKCS #8 PEM holds, and verify each with its public half", async () => {
    // generate an ES384 pair, and export the RS256 key of RFC 7515 A.2 as a PEM
    const world = testSimulation().environment("test");
    const es384 = await world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-384" }, true, [
        "sign",
        "verify",
    ]);
    const rsa = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" };
    const rsaPrivate = await world.crypto.importKey("jwk", RS256_KEY, rsa, true, ["sign"]);
    const { ["d"]: _d, ["p"]: _p, ["q"]: _q, dp: _dp, dq: _dq, qi: _qi, ...rsaPublic } = RS256_KEY;
    const rsaVerifier = await Jws.importJwk(rsaPublic, "RS256", world.crypto);
    const der = new Uint8Array(await world.crypto.exportKey("pkcs8", rsaPrivate));
    const pem = `-----BEGIN PRIVATE KEY-----\n${der.toBase64()}\n-----END PRIVATE KEY-----\n`;

    // sign one payload with each, and verify it, refusing a PEM of another label
    const payload = new TextEncoder().encode("assertion");
    const curve = await Jws.sign(payload, { alg: "ES384" }, es384.privateKey, world.crypto);
    const imported = await Jws.importPkcs8(pem, "RS256", world.crypto);
    const signed = await Jws.sign(payload, { alg: "RS256" }, imported, world.crypto);
    const opened = await Promise.all([
        Jws.verify(curve, es384.publicKey, ["ES384"], world.crypto),
        Jws.verify(signed, rsaVerifier, ["RS256"], world.crypto),
    ]);

    expect({
        opened: opened.map(({ header, payload: bytes }) => [
            header.alg,
            new TextDecoder().decode(bytes),
        ]),
        signature: Base64Url.decode(curve.split(".")[2] ?? "").length,
        refused: await refusal(
            Jws.importPkcs8(pem.replaceAll("PRIVATE KEY", "PUBLIC KEY"), "RS256", world.crypto),
        ),
    }).toEqual({
        opened: [
            ["ES384", "assertion"],
            ["RS256", "assertion"],
        ],
        signature: 96,
        refused: ["BAD_REQUEST", "KEY_INVALID", "the key is not a PKCS #8 PEM private key"],
    });
});

test("refuse weak and unfitting keys, a JWS marking an extension critical, and a thumbprint without a required member", async () => {
    // generate a short RSA key and a P-384 key, and sign one HS256 token
    const world = testSimulation().environment("test");
    const rsa = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" };
    const short = await world.crypto.generateKey(
        { ...rsa, modulusLength: 1024, publicExponent: new Uint8Array([1, 0, 1]) },
        true,
        ["sign", "verify"],
    );
    const es384 = await world.crypto.generateKey({ name: "ECDSA", namedCurve: "P-384" }, false, [
        "sign",
    ]);
    const secret = Base64Url.decode(HS256_KEY.k);
    const payload = new TextEncoder().encode("payload");
    const hs256 = await Jws.sign(payload, { alg: "HS256" }, secret, world.crypto);
    const { ["d"]: _d, ["p"]: _p, ["q"]: _q, dp: _dp, dq: _dq, qi: _qi, ...rsaPublic } = RS256_KEY;
    const rsaVerifier = await Jws.importJwk(rsaPublic, "RS256", world.crypto);

    // mark an extension critical in the header, keeping the payload and signature
    const [, ...signed] = hs256.split(".");
    const critical = [Base64Url.encode(JSON.stringify({ alg: "HS256", crit: ["exp"] })), ...signed];

    expect({
        short: await refusal(
            Jws.importJwk(await Jwk.export(short.publicKey, world.crypto), "RS256", world.crypto),
        ),
        secret: await refusal(
            Jws.sign(payload, { alg: "HS256" }, new Uint8Array(16), world.crypto),
        ),
        unfitting: await refusal(
            Jws.sign(payload, { alg: "ES256" }, es384.privateKey, world.crypto),
        ),
        encryption: await refusal(
            Jws.importJwk({ ...ES256_KEY, use: "enc" }, "ES256", world.crypto),
        ),
        confused: await refusal(Jws.verify(hs256, rsaVerifier, ["HS256"], world.crypto)),
        critical: await refusal(Jws.verify(critical.join("."), secret, ["HS256"], world.crypto)),
        thumbprint: await refusal(
            Jwk.thumbprint({ kty: "EC", crv: "P-256", x: "AA" }, world.crypto),
        ),
        prototype: await refusal(Jwk.thumbprint({ kty: "toString" }, world.crypto)),
    }).toEqual({
        short: ["BAD_REQUEST", "KEY_INVALID", "the RS256 key has 1024 bits"],
        secret: ["BAD_REQUEST", "KEY_INVALID", "the HS256 key has 128 bits"],
        unfitting: ["BAD_REQUEST", "KEY_INVALID", "the key does not sign ES256"],
        encryption: ["BAD_REQUEST", "KEY_INVALID", "the key is not an ES256 key"],
        confused: ["UNAUTHORIZED", "ALGORITHM_REFUSED", "no key verifies the algorithm HS256"],
        critical: ["UNAUTHORIZED", "TOKEN_MALFORMED", "the token marks an extension critical"],
        thumbprint: ["BAD_REQUEST", "KEY_INVALID", "the EC key has no y"],
        prototype: ["BAD_REQUEST", "KEY_INVALID", "a toString key has no thumbprint"],
    });
});

test("refuse a JWE header with crit, zip, apu or apv, a tag of another length, and a key of another management", async () => {
    // encrypt one plaintext to a wrapping key
    const world = testSimulation().environment("test");
    const wrapping = await world.crypto.generateKey({ name: "AES-GCM", length: 256 }, false, [
        "encrypt",
        "decrypt",
    ]);
    const recipient = await world.crypto.generateKey({ name: "ECDH", namedCurve: "P-256" }, false, [
        "deriveBits",
    ]);
    const compact = await Jwe.encrypt(
        new Uint8Array([1]),
        { alg: "A256GCMKW", enc: "A256GCM" },
        wrapping,
        world,
    );
    const [encodedHeader, key, iv, body, tag] = compact.split(".");

    // add a member to the protected header, or lengthen the tag, keeping the other segments
    const withMember = (member: string) => {
        const header = Base64Url.encode(JSON.stringify({ ...Jwe.header(compact), [member]: "x" }));

        return [header, key, iv, body, tag].join(".");
    };
    const longTag = Base64Url.encode(new Uint8Array(20));
    const lengthened = [encodedHeader, key, iv, body, longTag].join(".");

    expect({
        members: await Promise.all(
            ["crit", "zip", "apu", "apv"].map((member) =>
                refusal(Jwe.decrypt(withMember(member), wrapping, world.crypto)),
            ),
        ),
        tag: await refusal(Jwe.decrypt(lengthened, wrapping, world.crypto)),
        management: await refusal(Jwe.decrypt(compact, recipient.privateKey, world.crypto)),
    }).toEqual({
        members: [
            ["BAD_REQUEST", "CIPHERTEXT_MALFORMED", "the ciphertext header has crit"],
            ["BAD_REQUEST", "CIPHERTEXT_MALFORMED", "the ciphertext header has zip"],
            ["BAD_REQUEST", "CIPHERTEXT_MALFORMED", "the ciphertext header has apu"],
            ["BAD_REQUEST", "CIPHERTEXT_MALFORMED", "the ciphertext header has apv"],
        ],
        tag: ["BAD_REQUEST", "CIPHERTEXT_MALFORMED", "the ciphertext's tag has 20 bytes, not 16"],
        management: [
            "INTERNAL_SERVER_ERROR",
            "DECRYPTION_FAILED",
            "the ciphertext is sealed by A256GCMKW, the key opens ECDH-ES+A256KW",
        ],
    });
});

test("refuse a token older than its maximum age, issued in the future, of another type, or without a required claim", async () => {
    // sign a token issued at the start, typed as an access token
    const world = testSimulation().environment("test");
    const secret = Base64Url.decode(HS256_KEY.k);
    const seconds = TEST_START / 1000;
    const token = await Jwt.sign(
        { sub: "machine", iat: seconds },
        { alg: "HS256", typ: "at+jwt" },
        secret,
        world.crypto,
    );
    const check = (options: Partial<JwtOptions>) =>
        refusal(
            Jwt.verify(
                token,
                secret,
                { algorithms: ["HS256"], now: TEST_START, ...options },
                world.crypto,
            ),
        );

    expect({
        old: await check({ now: TEST_START + 61_000, maxTokenAge: 60 }),
        future: await check({ now: TEST_START - 10_000, maxTokenAge: 60 }),
        type: await check({ typ: "JWT" }),
        required: await check({ requiredClaims: ["jti"] }),
    }).toEqual({
        old: ["UNAUTHORIZED", "TOKEN_EXPIRED", "the token is older than its longest age"],
        future: ["UNAUTHORIZED", "CLAIM_INVALID", "the token was issued in the future"],
        type: ["UNAUTHORIZED", "CLAIM_INVALID", "the token's type is not JWT"],
        required: ["UNAUTHORIZED", "CLAIM_INVALID", "the token has no jti claim"],
    });
});
