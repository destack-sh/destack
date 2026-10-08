import { TestDatabase } from "@destack/db/test";
import { LocalKeyring } from "@destack/identity";
import { principal } from "@destack/access";
import { PackageId } from "@destack/package";
import { present } from "@destack/schema";
import { SPACE_KEY } from "@destack/service/authentication";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { decodeProtectedHeader, jwtVerify } from "jose";
import {
    directoryTables,
    DirectoryStore,
    identityKey,
    IdentityKeystore,
    KEY_SET_MILLISECONDS,
} from "../src/index.ts";

/** Read the key a token names. */
function kid(token: string): string | undefined {
    return decodeProtectedHeader(token).kid;
}

test("sign with the active key until every verifier read the next one, verifying tokens of every published key with a key set a verifier keeps", async () => {
    // start the universe's identity on a clock the test moves
    const storage = await TestDatabase.create("sqlite", [...directoryTables, identityKey], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    let now = Date.now();
    const clock = () => now;
    const directory = new DirectoryStore(storage.database, { clock });
    const keystore = new IdentityKeystore(
        await LocalKeyring.read(LocalKeyring.generate()),
        directory,
        clock,
    );
    const subject = Scope.universe.id;
    await keystore.generate(storage.database, subject);
    const sign = () => keystore.sign(storage.database, subject, { sub: "user-1" });
    const keys = directory.keys(subject);
    const verified = async (token: string) => (await jwtVerify(token, keys)).payload.sub;

    // sign and verify with the first key, then rotate to a next key
    const first = await sign();
    await verified(first);
    await keystore.rotate(storage.database, subject);

    // keep signing with the first key until the verifiers' key set age passed, then with the next one
    const during = await sign();
    now += KEY_SET_MILLISECONDS;
    const after = await sign();

    // verify every token with the key set the verifier read again once it aged
    expect([
        kid(during) === kid(first),
        kid(after) === kid(first),
        await verified(first),
        await verified(after),
        (await directory.identity(subject))?.signingKeys.length,
    ]).toEqual([true, false, "user-1", "user-1", 2]);
});

test("send a request as an installation with a token its space's key signs, which the directory verifies without a deployment", async () => {
    // start a space's identity on its machine
    const storage = await TestDatabase.create("sqlite", [...directoryTables, identityKey], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const directory = new DirectoryStore(storage.database);
    const keystore = new IdentityKeystore(
        await LocalKeyring.read(LocalKeyring.generate()),
        directory,
    );
    const placement = {
        id: "space-01996ab0-0000-7000-8000-000000000001",
        scope: "account-01996ab0-0000-7000-8000-000000000002",
        machine: "machine-01996ab0-0000-7000-8000-000000000003",
        epoch: 1,
    };
    await directory.place(placement);
    await keystore.generate(storage.database, placement.id, placement);

    // send a request as an installation of the space and verify what arrives
    const audience = PackageId.parse("package-01996ab0-0000-7000-8000-000000000004");
    const installation = principal.installation.reference(
        placement.id,
        "installation-01996ab0-0000-7000-8000-000000000005",
    );
    let received: Request | undefined;
    const send = keystore.fetch(storage.database, installation, audience, async (request) => {
        received = request;

        return new Response(null, { status: 204 });
    });
    await send(new Request("https://notes.test/"));
    const verified = await directory.authenticate(present(received, "the sent request"), {
        audience,
    });
    expect([
        verified.claims.subject,
        verified.claims.credential.kind,
        verified.claims.deployments,
    ]).toEqual([installation, SPACE_KEY, undefined]);
});
