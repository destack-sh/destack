import { MachineIdentity } from "@destack/account/client";
import { STACKS, ids, OPERATOR_EMAIL } from "@destack/account/test";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { MemoryBucket } from "@destack/bucket/test";
import { TestDatabase } from "@destack/db/test";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import { PackageMount } from "@destack/runtime";
import { CallKey } from "@destack/runtime/request";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { Universe, universeDatabase } from "../src/universe/index.ts";
import { Clock, Network } from "@destack/environment";

/** The origin the test universe answers at. */
const ORIGIN = "https://universe.test";

test("start a universe over its declaration, answering the account package below its mount and nothing elsewhere", async () => {
    // start the universe over a fresh database, with a cloud machine serving its residency
    const storage = await TestDatabase.create("sqlite", universeDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const keyring = await LocalKeyring.read(LocalKeyring.generate());
    const key = CallKey.of(keyring);
    const machine = new MachineIdentity(ids.machine, new MemoryKeychain(), Clock.system);
    const { publicKey } = await machine.generate();
    await using universe = await Universe.start({
        declaration: {
            residencies: [{ code: "eu", name: "Europe" }],
            platform: {
                name: "Destack",
                residencyId: "eu",
                operator: OPERATOR_EMAIL,
                reserved: [],
                publishers: [],
                handle: "destack",
                machines: [
                    {
                        id: ids.machine,
                        name: "eu-central",
                        residencyId: "eu",
                        providerCode: "cloudflare",
                        location: "eu",
                        key: publicKey,
                        origin: "https://eu-central.machine.test/",
                    },
                ],
            },
            stacks: STACKS,
        },
        origin: ORIGIN,
        database: storage.database,
        keyring,
        callKey: () => key,
        mail: { send: async () => ({ kind: "sent" as const }) },
        events: new MemoryBucket(Clock.system),
        ipAddress: { disableIpTracking: true },
        environment: {
            clock: Clock.system,
            network: { ...Network.system, fetch: async () => new Response(null, { status: 502 }) },
        },
        // leave the residency's home unreachable, its follows retrying
        report: () => {},
    });

    // answer the issuer's discovery below the account package's mount, and refuse other paths
    const issuer = `${PackageMount.url(ORIGIN, accountPackage.id)}${OAUTH_PATH}`;
    const discovery = await universe.fetch(
        new Request(`${issuer}/.well-known/openid-configuration`),
    );
    const { issuer: discovered } = schema
        .looseObject({ issuer: schema.string() })
        .parse(await discovery.json());
    const outside = await universe.fetch(new Request(`${ORIGIN}/auth/sign-in`));
    expect([discovery.status, discovered, outside.status]).toEqual([200, issuer, 404]);
});
