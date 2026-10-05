import { cp, mkdtemp, readFile, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, refusal, test } from "@destack/test";
import { connect, DirectoryClient } from "@destack/account/client";
import { HostIdentity } from "@destack/host/identity";
import { MemoryKeychain } from "@destack/host/keychain";
import { LocalBucket } from "@destack/bucket/local";
import { PackageBuilder, readDependencies } from "@destack/build";
import { PackageStore } from "@destack/build/store";
import { sql } from "@destack/db";
import * as postgresql from "@destack/db/postgres";
import { PackageDefinition, PackageId } from "@destack/package";
import { BUILDS_PATH, forgeService } from "@destack/forge/service";
import { Commit, Identifier, present, schema } from "@destack/schema";
import { ServiceMount } from "@destack/service";
import { isServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { PLATFORM_HANDLES } from "../src/universe/handle.ts";
import { DevelopmentUniverse } from "../src/universe/index.ts";

/** The test PostgreSQL server of `just postgres start`. */
const SERVER = "postgres://postgres@127.0.0.1:55432";

/** The email of the universe's operator. */
const OPERATOR = "operator@destack.test";

/** The handle of the person's account, which scopes the fixture package's npm name. */
const HANDLE = "smoke";

/** The fixture package, as its destack.json declares it. */
const PACKAGE_ID = PackageId.parse("package-00000000-0000-7000-8000-0000000000f1");

/** The session Better Auth answers for a signed-in browser. */
const SignedInSession = schema.looseObject({
    /** The session. */
    session: schema.looseObject({
        /** The session's token, which clients present as a bearer token. */
        token: schema.string(),
    }),
});

/** How long the registry may take to copy a new account, in milliseconds. */
const COPY_MILLISECONDS = 5000;

test("publish a build to the registry discovery finds for an account's residency as a person it introspects, and resolve its release there as a host whose key it copied", async () => {
    await using stack = new AsyncDisposableStack();
    const directory = await mkdtemp(join(tmpdir(), "destack-universe-"));
    stack.defer(() => rm(directory, { recursive: true }));

    // start a universe over a database of its own, keeping the sign-in links it mails
    const database = `destack_smoke_${crypto.randomUUID().replaceAll("-", "")}`;
    stack.defer(() => drop(database));
    const links: string[] = [];
    const universe = stack.use(await start(database, directory, links));

    // sign a person in through a mailed link, keeping the session cookie
    const origin = universe.url.origin;
    const { cookie, send } = await signIn(origin, "smoke@destack.test", links);

    // call the region's services with the session's token, which they introspect at the account service
    const signedInSession = await fetch(`${origin}/auth/get-session`, { headers: { cookie } });
    const { session } = SignedInSession.parse(await signedInSession.json());
    const bearer = (request: Request) => {
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${session.token}`);

        return fetch(new Request(request, { headers }));
    };

    // choose the eu residency and the personal account's handle
    const accounts = connect({ url: universe.accounts, fetch: send });
    const { subject } = await accounts.authentication.current();
    const userId = schema.identifier("user").parse(subject.id);
    const person = await accounts.user.get({ id: userId });
    await accounts.user.update({
        id: userId,
        requestId: RequestId.create(),
        revision: person.revision,
        residencyId: "eu",
    });
    const account = await accounts.account.create({
        scope: userId,
        requestId: RequestId.create(),
        handle: HANDLE,
        name: "Smoke",
        kind: "personal",
    });

    // find the account's registry through the directory: the eu region at the universe's origin
    const directoryClient = new DirectoryClient(accounts);
    const region = present(
        await directoryClient.region(account.id, forgeService.package.id),
        "the registry's region",
    );
    expect(region.endpoint).toBe(origin);
    const registry = await directoryClient.placed(forgeService, account.id, bearer);

    // create the package once the registry copied the account
    await eventually(() =>
        registry.package.create({
            accountId: account.id,
            id: PACKAGE_ID,
            requestId: RequestId.create(),
            name: "hello",
            visibility: "public",
        }),
    );

    // build the fixture package, push the build and release it
    const source = join(directory, "hello");
    await cp(new URL("./fixture/hello/", import.meta.url), source, { recursive: true });
    await using builder = await PackageBuilder.start(source);
    const definition = PackageDefinition.parse(
        JSON.parse(await readFile(join(source, "destack.json"), "utf8")),
    );
    const outputs = Object.fromEntries(
        Object.entries(present(definition.publication, "the publication").outputs).map(
            ([name, output]) => [name, { kind: "module" as const, ...output }],
        ),
    );
    await using build = await builder.build({
        outputs,
        dependencies: await readDependencies(source),
        commit: Commit.parse("a".repeat(40)),
    });
    const bucket = await LocalBucket.open(join(directory, "builds"), "smoke");
    stack.use(bucket);
    const store = new PackageStore(bucket);
    const manifest = await store.put(build);
    const endpoint = ServiceMount.url(region.endpoint, forgeService.package.id);
    await store.push(manifest, `${endpoint}${BUILDS_PATH}`, bearer);
    const released = await registry.release.create({
        accountId: account.id,
        requestId: RequestId.create(),
        parentId: PACKAGE_ID,
        manifest,
    });

    // enroll a host of the account, whose token the registry verifies against its copy of the host's key
    const host = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
    await host.enroll(accounts, {
        accountId: account.id,
        requestId: RequestId.create(),
        name: "laptop",
        kind: "cloud",
    });
    const asHost = await directoryClient.placed(
        forgeService,
        account.id,
        host.fetch(fetch, forgeService.package.id, universe.accounts),
    );
    const find = { packageId: PACKAGE_ID, version: released.version };
    await expect
        .poll(async () => refusal(asHost.manifests.find(find)), { timeout: COPY_MILLISECONDS })
        .toBe("done");

    // resolve the release's build through the discovered registry, as the person and as the host
    expect([
        released.version,
        await registry.manifests.find(find),
        await asHost.manifests.find(find),
    ]).toEqual(["2026.10.0", { manifest }, { manifest }]);
}, 60_000);

test("keep the residencies with their regions and the platform's handles with the platform organisation's accounts at every start, creating only what is missing", async () => {
    await using stack = new AsyncDisposableStack();
    const directory = await mkdtemp(join(tmpdir(), "destack-universe-"));
    stack.defer(() => rm(directory, { recursive: true }));

    // start a universe twice over the same database
    const database = `destack_bootstrap_${crypto.randomUUID().replaceAll("-", "")}`;
    stack.defer(() => drop(database));
    const links: string[] = [];
    await (await start(database, directory, links)).close();
    const universe = stack.use(await start(database, directory, links));

    // read the residencies, the regions and the handles' accounts as an owner of the platform organisation
    const origin = universe.url.origin;
    const { send } = await signIn(origin, OPERATOR, links);
    const accounts = connect({ url: universe.accounts, fetch: send });
    const { items: regions } = await accounts.region.list({});
    const [platform] = (await accounts.organisation.list({})).items;
    const owned = await accounts.account.list({
        scope: present(platform, "the platform organisation").id,
        limit: 1000,
    });

    // keep each residency and region once, and one shared account of the platform per handle
    expect([
        await accounts.directory.residencies({}),
        regions
            .toSorted((left, right) => left.code.localeCompare(right.code))
            .map(({ code, residencyId }) => [code, residencyId]),
        owned.items
            .filter((entry) => entry.kind === "shared")
            .map((entry) => entry.handle)
            .toSorted(),
    ]).toEqual([
        [
            { id: "eu", name: "European Union" },
            { id: "us", name: "United States" },
        ],
        [
            ["eu-central", "eu"],
            ["us-east", "us"],
        ],
        [...PLATFORM_HANDLES].toSorted(),
    ]);
}, 60_000);

/** Start a universe over a database and a directory, keeping the sign-in links it mails. */
async function start(
    database: string,
    directory: string,
    links: string[],
): Promise<DevelopmentUniverse> {
    const relay = await freePort();

    return DevelopmentUniverse.start({
        server: SERVER,
        database,
        origin: "http://127.0.0.1:0",
        relay: { origin: `http://127.0.0.1:${relay}`, hostname: "127.0.0.1", port: relay },
        region: "eu-central",
        directory: join(directory, "universe"),
        secret: "destack-smoke-universe-secret-32-characters",
        operator: OPERATOR,
        mail: {
            sendMagicLink: async ({ url }) => {
                links.push(url);
            },
            sendCode: async () => {},
            description: "mail: keeping sign-in links",
        },
        report: () => {},
    });
}

/** Sign a person in through a mailed link, sending their requests with the session cookie. */
async function signIn(origin: string, email: string, links: readonly string[]) {
    // follow the mailed link, keeping the session cookie
    await fetch(`${origin}/auth/sign-in/magic-link`, {
        method: "POST",
        headers: { origin, "content-type": "application/json" },
        body: JSON.stringify({ email, name: email, callbackURL: "/" }),
    });
    const signedIn = await fetch(present(links.at(-1), "the sign-in link"), { redirect: "manual" });
    const cookie = signedIn.headers
        .getSetCookie()
        .map((entry) => entry.slice(0, entry.indexOf(";")))
        .join("; ");

    // send requests with the cookie from the universe's origin
    const send = (request: Request) => {
        const headers = new Headers(request.headers);
        headers.set("cookie", cookie);
        headers.set("origin", origin);

        return fetch(new Request(request, { headers }));
    };

    return { cookie, send };
}

/** Retry a call while its account is not yet copied, within the copy deadline. */
async function eventually<Value>(call: () => Promise<Value>): Promise<Value> {
    const deadline = Date.now() + COPY_MILLISECONDS;
    for (;;) {
        try {
            return await call();
        } catch (error) {
            // retry a refusal of the not yet copied account until the deadline
            if (!isServiceError(error) || error.status !== 404 || Date.now() > deadline) {
                throw error;
            }
            await new Promise((resolve) => {
                setTimeout(resolve, 50);
            });
        }
    }
}

/** Drop a universe's database. */
async function drop(database: string): Promise<void> {
    const server = await postgresql.connect(`${SERVER}/postgres`);
    try {
        await server.execute(sql.raw(`DROP DATABASE IF EXISTS "${database}" WITH (FORCE)`));
    } finally {
        await server.close();
    }
}

/** Find a free local port. */
async function freePort(): Promise<number> {
    // bind any port, and release it
    const probe = createServer();
    await new Promise<void>((resolve) => {
        probe.listen(0, "127.0.0.1", resolve);
    });
    const address = probe.address();
    await new Promise<void>((resolve) => {
        probe.close(() => resolve());
    });
    if (address === null || typeof address === "string") {
        throw new TypeError("the probe bound no TCP port");
    }

    return address.port;
}
