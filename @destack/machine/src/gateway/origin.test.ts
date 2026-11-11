import type { Crypto } from "@destack/environment";
import { OTLP_ORIGIN_PATH } from "@destack/telemetry/otlp";
import { eq } from "@destack/db";
import { testHost } from "@destack/test/host";
import { DOMAINS } from "@destack/account/address";
import accountPackage from "@destack/account/package";
import { TestDns } from "@destack/account/test";
import { Jwt, KeyPair, Proof, PROOF_HEADER } from "@destack/identity";
import * as accountObject from "@destack/account/object";
import { space, user } from "@destack/account/object";
import { ANONYMOUS, anyone, LinkSecret, principal, relation, Role } from "@destack/access";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import {
    defineObject,
    field,
    definePackage,
    PACKAGE_PATH,
    PackageMount,
    VERSION_HEADER,
    Bookmark,
} from "@destack/object";
import objectPackage from "@destack/object/package";
import { DynamicObjectClient, createClient } from "@destack/object/client";
import { describeObject, describePackage } from "@destack/object/inspect";
import observabilityPackage from "@destack/observability/package";
import { PackageVersion, PackageId, Sandbox } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import objectDefinition from "@destack/object/destack.json" with { type: "json" };
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import { Identifier, nth, present, schema } from "@destack/schema";
import { setting } from "@destack/setting/object";
import * as stackObject from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { expect, refusal, test } from "@destack/test";
import { type Address, HOME_PATH, PLATFORM_PATH } from "@destack/view/declare";
import viewPackage from "@destack/view/destack.json" with { type: "json" };
import { LINK_COOKIE, SESSION_COOKIE } from "./origin.ts";
import {
    OWNER_EMAIL,
    PUBLISHER,
    TestComputer,
    SCENARIO_MILLISECONDS,
} from "../computer/test/index.ts";
import { TestViewBuild } from "./test/view.ts";
import { argument, plural, t } from "@destack/locale";
import { presentable, shareable } from "@destack/object";

/** The address of a second person signing in on the owner's computer. */
const MEMBER_EMAIL = "member@example.test";

/** The greeting package, whose view reads its own catalogs and its widget library's. */
const GREETING = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d2",
    name: "@acme/greeting",
    version: "2026.9.0-dev.1",
});

/** The widget library the greeting package pins as a release, with catalogs of its own. */
const WIDGETS = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d3",
    name: "@acme/widgets",
    version: "2026.9.0-dev.1",
});

/** The notes package whose views the launches open. */
const notes = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000e1",
    name: "@acme/notes",
    version: "2026.9.0-dev.1",
});

/** The notes package: it reads the person's settings, writes them once asked, and scans notes. */
const notesPackage = definePackage(
    {
        access: {
            permissions: {
                user: { home: [setting.permission("read")] },
                optional: { home: [setting.permission("write")] },
            },
        },
        capabilities: [
            { name: "camera", reason: t`Scan notes`, optional: true, sandbox: { camera: {} } },
        ],
    },
    { package: notes },
);

/** The tasks package, whose objects its own installation serves: tasks their owner completes. */
const tasks = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c4",
    name: "@acme/tasks",
    version: "2026.10.0-dev.1",
});

/** A task its owner completes. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
        scope: space,
        fields: {
            /** The person who created the task. */
            owner: field.subject().caller(),
            /** The title. */
            title: field.string(schema.string().min(1)),
            /** Whether the task is open or done. */
            status: field.state({
                initial: "open",
                transitions: { complete: { from: ["open"], to: "done", permission: "manage" } },
            }),
        },
        access: {
            permissions: { read: relation("owner"), manage: relation("owner") },
        },
        traits: [presentable({ icon: false, color: false })],
    },
    (method) => ({
        create: method.create("manage", { fields: ["title"] }),
    }),
    { package: tasks },
);

/** The tasks package's handle: its board reads and manages the person's tasks in the space. */
const tasksPackage = definePackage(
    {
        objects: [task],
        access: {
            permissions: { user: { space: [task.permission("read"), task.permission("manage")] } },
        },
    },
    { package: tasks },
);

/** A row's identifier. */
const Identified = schema.looseObject({ id: schema.string() });

/** A row's title. */
const Titled = schema.looseObject({ title: schema.string() });

/** The launch in a view's document, in the parts a scenario reads. */
const Launched = schema.looseObject({ view: schema.string(), target: ObjectReference });

/** Write a notes build with a home view listing notes and an editor presenting them. */
function notesBuild(crypto: Crypto): Promise<BuildReader> {
    return TestViewBuild.of(
        notes,
        {
            home: {
                entrypoint: "browser/home.js",
                stylesheets: [],
                sandbox: { "clipboard-write": {} },
                presents: [{ packageId: notes.id, type: "note", priority: "option" }],
                categories: ["main"],
                audience: "person" as const,
                packages: [],
                render: { mode: "client" },
            },
            editor: {
                entrypoint: "browser/editor.js",
                stylesheets: [],
                sandbox: {},
                presents: [{ packageId: notes.id, type: "note", priority: "default" }],
                categories: [],
                audience: "person" as const,
                packages: [],
                render: { mode: "client" },
            },
        },
        { package: notesPackage },
        crypto,
    );
}

/** Write the tasks build with its board view, which declares the stack service. */
function tasksBuild(crypto: Crypto): Promise<BuildReader> {
    const board = {
        entrypoint: "browser/board.js",
        stylesheets: [],
        sandbox: {},
        presents: [],
        categories: [],
        audience: "person" as const,
        packages: [stackPackage.id],
        render: { mode: "client" as const },
    };

    return MemoryBuild.of(
        tasks,
        [
            {
                kind: "object",
                kindPackageId: objectDefinition.id,
                name: "task",
                description: describeObject(task),
            },
            {
                kind: "package",
                kindPackageId: objectDefinition.id,
                name: "default",
                description: describePackage(tasksPackage),
            },
            {
                kind: "view",
                kindPackageId: viewPackage.id,
                name: "board",
                description: {
                    name: "board",
                    presents: [],
                    categories: [],
                    audience: "person" as const,
                    packages: [stackPackage.id],
                    render: { mode: "client" },
                },
            },
        ],
        crypto,
        {
            outputs: {
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views: { board },
                    directory: "browser",
                    exports: {},
                    dependencies: {},
                    peers: {},
                },
            },
        },
    );
}

/** The articles package, whose articles their owner publishes through links. */
const articles = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c5",
    name: "@acme/articles",
    version: "2026.10.0-dev.1",
});

/** An article its owner shares, publicly through a link. */
const article = defineObject(
    {
        name: "article",
        plural: "articles",
        title: t`${plural(argument("count"), { one: "Article", other: "Articles" })}`,
        scope: space,
        fields: {
            /** The title. */
            title: field.string(schema.string().min(1)),
        },
        traits: [presentable({ icon: false, color: false }), shareable({ isPublic: true })],
    },
    (method) => ({
        update: method.update("write", {
            limits: { rate: { count: 1, every: { hours: 1 } }, key: "caller" },
        }),
    }),
    { package: articles },
);

/** The articles package's handle: its reader reads, writes and shares the person's articles. */
const articlesPackage = definePackage(
    {
        objects: [article],
        access: {
            permissions: {
                user: {
                    space: [
                        article.permission("read"),
                        article.permission("write"),
                        article.permission("manage"),
                    ],
                },
            },
        },
    },
    { package: articles },
);

/** Write the articles build with its reader view. */
function articlesBuild(crypto: Crypto): Promise<BuildReader> {
    const reader = {
        entrypoint: "browser/reader.js",
        stylesheets: [],
        sandbox: {},
        presents: [],
        categories: [],
        audience: "person" as const,
        packages: [],
        render: { mode: "client" as const },
    };

    return MemoryBuild.of(
        articles,
        [
            {
                kind: "object",
                kindPackageId: objectDefinition.id,
                name: "article",
                description: describeObject(article),
            },
            {
                kind: "package",
                kindPackageId: objectDefinition.id,
                name: "default",
                description: describePackage(articlesPackage),
            },
            {
                kind: "view",
                kindPackageId: viewPackage.id,
                name: "reader",
                description: {
                    name: "reader",
                    presents: [],
                    categories: [],
                    audience: "person" as const,
                    packages: [],
                    render: { mode: "client" },
                },
            },
        ],
        crypto,
        {
            outputs: {
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views: { reader },
                    directory: "browser",
                    exports: {},
                    dependencies: {},
                    peers: {},
                },
            },
        },
    );
}

/** A space a computer serves, with the owner signed in on the computer's device. */
interface Served {
    /** The computer serving the space. */
    readonly computer: TestComputer;
    /** The residency's cloud computer, which serves the forge of its home. */
    readonly cloud: TestComputer;
    /** The owner's login on the computer's device. */
    readonly login: Identifier<"login">;
    /** The space. */
    readonly spaceId: Identifier<"space">;
    /** The space's address: its name and its account's handle. */
    readonly address: string;
}

/** Serve the owner's home on its cloud computer, with the organization publishing the packages. */
async function openHome(): Promise<Served> {
    const { home, login } = await TestComputer.home({ organizations: [PUBLISHER] });
    const { owner } = home.world;
    const address = `home--${owner.handle}`;

    return { computer: home, cloud: home, login, spaceId: owner.home, address };
}

/** Serve a work space of the owner's account on their own computer in the home's world. */
async function openWork(home: Served): Promise<Served> {
    // enroll the owner's computer through the relay in their account, and create the space on it
    const relay = await TestComputer.relay(home.computer.world);
    const computer = await TestComputer.open({ world: home.computer.world, relay: relay.url });
    const { owner } = computer.world;
    const signedIn = await computer.signIn(OWNER_EMAIL);
    await computer.enroll(signedIn, owner.account);
    const spaceId = await computer.space(signedIn, owner.account, "work");

    const address = `work--${owner.handle}`;

    return { computer, cloud: home.cloud, login: signedIn.id, spaceId, address };
}

/** Serve the owner's work space shared with a second person by invitation. */
async function openShared(
    home: Served,
): Promise<Served & { readonly member: Identifier<"login"> }> {
    // serve the work space, and sign the second person in on its computer
    const opened = await openWork(home);
    const { computer, spaceId } = opened;
    const member = await computer.signIn(MEMBER_EMAIL);

    // invite them to own the space with the owner, and accept as them from their own listing
    const object = space.reference(computer.world.owner.account, spaceId);
    const inviting = (await computer.device.launcher(opened.login)).session;
    const invitation = await inviting.objects().invitation.create({
        scope: computer.world.owner.account,
        requestId: RequestId.create(computer.environment),
        object,
        relationship: {
            role: Role.OWNER,
            subject: principal.user.reference(Scope.universe.id, member.subjectId),
        },
    });
    const accepting = (await computer.device.launcher(member.id)).session;
    const { items } = await accepting.accounts().user.invited({ id: member.subjectId });
    expect(items.map((each) => each.id)).toEqual([invitation.id]);
    await accepting.objects().invitation.accept({
        scope: computer.world.owner.account,
        requestId: RequestId.create(computer.environment),
        id: invitation.id,
    });

    return { ...opened, member: member.id };
}

/** Publish a build as the owner of its scope and install it in a served space, until applied. */
async function install(served: Served, build: BuildReader, name: string) {
    // publish the build into its organization's forge as the owner, then install it
    const { world } = served.computer;
    await served.cloud.publish(build, world.owner);

    return served.computer.install(served.login, served.spaceId, build, name);
}

/** Install tasks in a machine's space, applied with its objects served. */
async function installTasks(served: Served) {
    return install(served, await tasksBuild(served.computer.environment.crypto), "tasks");
}

/** Open a view as the signed-in owner, as a browser opens its launch. */
async function openPage(served: Served, address: Address) {
    // launch it under the owner's grant, and trade its token for the session cookie
    const { device: native, computer } = served.computer;
    const listener = computer;
    const session = await native.launcher(served.login);
    const signedIn = await native.signedIn(served.login);
    const launched = await session.launch(address);
    const grant = present(
        await session.granted(principal.installation.reference(launched.space, launched.target)),
        "the owner's grant to the view's installation",
    );
    const traded = await listener.fetch(new Request(launched.url));
    const cookie = nth(header(traded, "set-cookie").split(";"), 0);
    const origin = new URL(launched.url).origin;

    // send from the page's own origin with its cookie
    const fetch = (request: Request) => {
        const headers = new Headers(request.headers);
        headers.set("cookie", cookie);
        headers.set("sec-fetch-site", "same-origin");

        return listener.fetch(new Request(request, { headers }));
    };

    return { session, signedIn, grant, launched, traded, cookie, origin, fetch };
}

/** Describe the refusal an origin answers with, as its JSON body states it. */
function answered(
    code: string,
    reason: string,
    status: number,
    message: string,
    domain = "@destack/machine",
): [number, string] {
    return [
        status,
        JSON.stringify({
            defined: true,
            code,
            status,
            message,
            reason,
            domain,
        }),
    ];
}

/** Describe a response by its status and text. */
async function summary(response: Response): Promise<[number, string]> {
    return [response.status, await response.text()];
}

/** Read a header a response must have, failing when it is missing. */
function header(response: Response, name: string): string {
    return present(response.headers.get(name), `the ${name} header`);
}

/** Register a browser client from a page's origin. */
function registerClient(
    page: Awaited<ReturnType<typeof openPage>>,
    id: Identifier<"client">,
): Promise<Response> {
    return page.fetch(
        new Request(`${page.origin}/.destack/client`, {
            method: "POST",
            body: JSON.stringify({ id, name: "Firefox", platform: "macOS" }),
        }),
    );
}

/** Read the first cookie a response sets, as a browser sends it back. */
function firstCookie(response: Response): string {
    return nth(header(response, "set-cookie").split(";"), 0);
}

/** Read the context in a view's document. */
async function contextOf(document: Response): Promise<unknown> {
    const embedded = /id="destack-view">(.*)<\/script>/u.exec(await document.text());

    return embedded === null ? undefined : JSON.parse(nth(embedded, 1));
}

test(
    "launch a view of the person's own home under a grant created at once, onto a session cookie serving its document with its installation's browser features, and serve it to an anonymous visitor without one",
    async () => {
        // install notes in the owner's home
        const opened = await openHome();
        const { computer, spaceId } = opened;
        const { device: native, world } = computer;
        const listener = computer.computer;
        const { owner } = world;
        const installation = await install(
            opened,
            await notesBuild(computer.environment.crypto),
            "notes",
        );
        const login = await native.launcher(opened.login);
        const address = { space: opened.address, installation: "notes", view: "home", path: "/" };

        // launch it, granting what the view asks at once, and trade the launch's token for the session cookie
        const launched = await login.launch(address);
        const granted = await login.granted(
            principal.installation.reference(spaceId, installation.id),
        );
        const traded = await listener.fetch(new Request(launched.url));
        const cookie = nth(header(traded, "set-cookie").split(";"), 0);
        const page = new URL(header(traded, "location"), launched.url).href;
        const document = await listener.fetch(new Request(page, { headers: { cookie } }));
        const policy = document.headers.get("permissions-policy");
        const context = await contextOf(document);

        // allow the optional camera and read the document again
        const service = createClient(
            objectPackage,
            login.endpoint(objectPackage.id, await native.locate(spaceId)),
            native.environment,
        );
        await service.installation.update({
            scope: spaceId,
            id: installation.id,
            requestId: RequestId.create(computer.environment),
            capabilities: ["camera"],
        });
        const allowed = await listener.fetch(new Request(page, { headers: { cookie } }));

        // serve the page without its cookie to an anonymous visitor, and refuse a launch with a forged token at the loopback origin
        const port = new URL(await native.origin()).port;
        const unopened = await listener.fetch(new Request(page));
        const forged = await listener.fetch(
            new Request(`http://notes--${opened.address}.localhost:${port}/home?token=forged`),
        );

        // open the origin's root, which serves the main view, and load a page of no view
        const root = await listener.fetch(
            new Request(`http://notes--${opened.address}.localhost:${port}/`),
        );
        const missing = await listener.fetch(
            new Request(`http://notes--${opened.address}.localhost:${port}/missing`, {
                headers: { "sec-fetch-mode": "navigate", "sec-fetch-dest": "document" },
            }),
        );
        const person = principal.user.reference(Scope.universe.id, owner.id);
        const permissions = [owner.id, spaceId].map((scope) => ({
            ...setting.permission("read"),
            scope,
        }));
        expect({
            granted: { ...granted, id: "grant" },
            url: launched.url.replace(/token=[^&]+/u, "token=…"),
            target: launched.target,
            traded: [traded.status, header(traded, "location"), cookie.split("=")[0]],
            cookie: header(traded, "set-cookie").split("; ").slice(1),
            document: document.status,
            context,
            policies: [policy, allowed.headers.get("permissions-policy")],
            unopened: [unopened.status, await contextOf(unopened)],
            forged: forged.status,
            root: [root.status, Launched.pick({ view: true }).parse(await contextOf(root)).view],
            missing: [missing.status, missing.headers.get("content-type")],
        }).toEqual({
            granted: { id: "grant", scope: owner.id, permissions },
            url: `http://notes--${opened.address}.localhost:${port}/home?token=…`,
            target: installation.id,
            traded: [303, "/home", SESSION_COOKIE],
            cookie: ["Path=/", "HttpOnly", "SameSite=Lax", "Max-Age=1209600"],
            document: 200,
            context: {
                installation: installation.id,
                space: spaceId,
                account: owner.account,
                home: owner.home,
                view: "home",
                base: "",
                universe: computer.world.universe.origin,
                catalogs: [],
                user: person,
            },
            policies: [
                Sandbox.policy({ "clipboard-write": {} }),
                Sandbox.policy({ "clipboard-write": {}, camera: {} }),
            ],
            unopened: [
                200,
                {
                    installation: installation.id,
                    space: spaceId,
                    account: owner.account,
                    view: "home",
                    base: "",
                    universe: computer.world.universe.origin,
                    catalogs: [],
                    user: ANONYMOUS,
                },
            ],
            forged: 401,
            root: [200, "home"],
            missing: [404, "text/html; charset=utf-8"],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "ask an anonymous page load's person at the issuer for a grant a view another installed needs beyond its space, then launch it as them",
    async () => {
        // install notes in the owner's work space, whose origin the person they shared it with loads signed out
        const opened = await openShared(await openHome());
        const { computer } = opened;
        const { universe } = computer.world;
        const listener = computer.computer;
        await install(opened, await notesBuild(computer.environment.crypto), "notes");
        const port = new URL(await computer.device.origin()).port;
        const origin = `http://notes--${opened.address}.localhost:${port}`;
        const loaded = await listener.fetch(
            new Request(`${origin}/`, {
                headers: { "sec-fetch-mode": "navigate", "sec-fetch-dest": "document" },
            }),
        );

        // sign the member in at the issuer, whose code returns to the origin, which asks them for the grant
        const member = await universe.signIn(MEMBER_EMAIL);
        const signedIn = await universe.approve(header(loaded, "location"), member);
        const asking = await listener.fetch(
            new Request(signedIn, { headers: { cookie: firstCookie(loaded) } }),
        );

        // consent to the grant at the issuer, then trade the launch for the origin's session and load the page
        const consented = await universe.approve(header(asking, "location"), member);
        const completed = await listener.fetch(
            new Request(consented, { headers: { cookie: firstCookie(asking) } }),
        );
        const traded = await listener.fetch(new Request(header(completed, "location")));
        const page = await listener.fetch(
            new Request(new URL(header(traded, "location"), origin).href, {
                headers: { cookie: firstCookie(traded) },
            }),
        );
        expect({
            asking: [asking.status, new URL(header(asking, "location")).pathname],
            completed: completed.status,
            page: [page.status, Launched.pick({ view: true }).parse(await contextOf(page)).view],
        }).toEqual({
            asking: [302, "/oauth/oauth2/authorize"],
            completed: 303,
            page: [200, "home"],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "ask a page's person at the issuer for an optional permission a call needs, then return to the page with it",
    async () => {
        // load notes signed in as the member, as a signed-out page load does
        const opened = await openShared(await openHome());
        const { computer } = opened;
        const { universe } = computer.world;
        const listener = computer.computer;
        await install(opened, await notesBuild(computer.environment.crypto), "notes");
        const port = new URL(await computer.device.origin()).port;
        const origin = `http://notes--${opened.address}.localhost:${port}`;
        const document = { "sec-fetch-mode": "navigate", "sec-fetch-dest": "document" };
        const loaded = await listener.fetch(new Request(`${origin}/`, { headers: document }));
        const member = await universe.signIn(MEMBER_EMAIL);
        const follow = async (from: Response, cookie: string) => {
            const signedIn = await universe.approve(header(from, "location"), member);

            return listener.fetch(new Request(signedIn, { headers: { cookie } }));
        };
        const asking = await follow(loaded, firstCookie(loaded));
        const completed = await follow(asking, firstCookie(asking));
        const traded = await listener.fetch(new Request(header(completed, "location")));
        const session = firstCookie(traded);

        // ask for writing the member's settings, which the package declares optional, returning to the editor
        const query = new URLSearchParams({
            package: setting.package.id,
            type: setting.name,
            name: "write",
            return: "/editor",
        });
        const asked = await listener.fetch(
            new Request(`${origin}/.destack/request?${query.toString()}`, {
                headers: { cookie: session },
            }),
        );
        const growing = await follow(asked, firstCookie(asked));
        const grown = await follow(growing, firstCookie(growing));
        const returned = await listener.fetch(new Request(header(grown, "location")));

        // ask the grant again with writing in both home scopes, then return to the editor
        const request = new URL(header(growing, "location")).searchParams.get("request");
        const details = schema
            .looseObject({
                authorization_details: schema.array(
                    schema.looseObject({
                        permissions: schema.array(schema.looseObject({ name: schema.string() })),
                    }),
                ),
            })
            .parse(Jwt.claims(present(request, "the signed request")));
        expect({
            asked: [asked.status, new URL(header(asked, "location")).pathname],
            permissions: details.authorization_details.flatMap((detail) =>
                detail.permissions.map((permission) => permission.name),
            ),
            returned: new URL(header(returned, "location"), origin).pathname,
        }).toEqual({
            asked: [302, "/oauth/oauth2/authorize"],
            permissions: ["read", "read", "write", "write"],
            returned: "/editor",
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "store a link's secret in its origin's cookie for an anonymous visitor, dropping it from the address on the same origin, and refuse a malformed one",
    async () => {
        // install notes in the owner's home, and draw a link's secret
        const opened = await openHome();
        const { computer } = opened;
        await install(opened, await notesBuild(computer.environment.crypto), "notes");
        const { secret } = await LinkSecret.create(computer.environment);
        const port = new URL(await computer.device.origin()).port;
        const page = `http://notes--${opened.address}.localhost:${port}/home`;

        // open the view by the link, by a malformed secret, and by a path a browser reads as another host
        const origin = new URL(page).origin;
        const linked = await computer.computer.fetch(new Request(`${page}?link=${secret}`));
        const malformed = await computer.computer.fetch(new Request(`${page}?link=forged`));
        const outside = await computer.computer.fetch(
            new Request(`${origin}//evil.example/home?link=${secret}`),
        );
        expect({
            linked: [linked.status, header(linked, "location"), header(linked, "set-cookie")],
            outside: header(outside, "location"),
            malformed: await summary(malformed),
        }).toEqual({
            linked: [303, "/home", `${LINK_COOKIE}=${secret}; Path=/; HttpOnly; SameSite=Lax`],
            outside: "/evil.example/home",
            malformed: answered(
                "FORBIDDEN",
                "LINK_SECRET_DENIED",
                403,
                "link secret is forbidden: it does not match",
                "@destack/access",
            ),
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "sign an anonymous page load of a person's view in at the issuer by its space's signed request, and launch the page as them",
    async () => {
        // install notes in the owner's home, and load its origin's root as a browser with no session
        const opened = await openHome();
        const { computer } = opened;
        const listener = computer.computer;
        await install(opened, await notesBuild(computer.environment.crypto), "notes");
        const port = new URL(await computer.device.origin()).port;
        const origin = `http://notes--${opened.address}.localhost:${port}`;
        const loaded = await listener.fetch(
            new Request(`${origin}/`, {
                headers: { "sec-fetch-mode": "navigate", "sec-fetch-dest": "document" },
            }),
        );
        const authorize = new URL(header(loaded, "location"));
        const attempt = nth(header(loaded, "set-cookie").split(";"), 0);

        // sign the owner in at the issuer, which returns the browser to the origin's open path with a code
        const back = new URL(
            await computer.world.universe.approve(authorize.href, computer.world.owner),
        );
        const completed = await listener.fetch(
            new Request(back.href, { headers: { cookie: attempt } }),
        );

        // trade the launch for the origin's session, then load the page as the owner
        const traded = await listener.fetch(new Request(header(completed, "location")));
        const cookie = nth(header(traded, "set-cookie").split(";"), 0);
        const page = await listener.fetch(
            new Request(new URL(header(traded, "location"), origin).href, { headers: { cookie } }),
        );
        expect({
            loaded: loaded.status,
            client: [
                authorize.origin + authorize.pathname,
                [...authorize.searchParams.keys()],
                authorize.searchParams.get("client_id"),
            ],
            back: back.origin + back.pathname,
            completed: [completed.status, completed.headers.getSetCookie().at(-1)],
            traded: traded.status,
            page: [page.status, Launched.pick({ view: true }).parse(await contextOf(page)).view],
        }).toEqual({
            loaded: 302,
            client: [
                `${computer.world.universe.origin}/oauth/oauth2/authorize`,
                ["client_id", "request"],
                origin,
            ],
            back: `${origin}/.destack/open`,
            completed: [303, "destack-sign-in=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"],
            traded: 303,
            page: [200, "home"],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "take an anonymous visitor's telemetry, and refuse a session cookie the space did not sign, clearing it",
    async () => {
        // install notes in the owner's home
        const opened = await openHome();
        const { computer } = opened;
        await install(opened, await notesBuild(computer.environment.crypto), "notes");
        const port = new URL(await computer.device.origin()).port;
        const origin = `http://notes--${opened.address}.localhost:${port}`;

        // send telemetry without a session, and open the view with a forged one
        const telemetry = await computer.computer.fetch(
            new Request(`${origin}${OTLP_ORIGIN_PATH}/v1/traces`, {
                method: "POST",
                headers: { origin, "content-type": "application/json" },
                body: "{}",
            }),
        );
        const forged = await computer.computer.fetch(
            new Request(`${origin}/home`, { headers: { cookie: `${SESSION_COOKIE}=forged` } }),
        );
        expect({
            telemetry: await summary(telemetry),
            forged: [...(await summary(forged)), header(forged, "set-cookie")],
        }).toEqual({
            telemetry: [200, "{}"],
            forged: [
                ...answered(
                    "UNAUTHORIZED",
                    "SESSION_UNAUTHENTICATED",
                    401,
                    "authentication required: the page's session is invalid or expired",
                ),
                `${SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0`,
            ],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "serve an article shared by link to an anonymous visitor holding its secret in the origin's cookie, and refuse it without the link or once revoked",
    async () => {
        // open the articles reader as the owner and create a article
        const home = await openHome();
        const { environment } = home.computer.device;
        await install(home, await articlesBuild(home.computer.environment.crypto), "articles");
        const address = {
            space: home.address,
            installation: "articles",
            view: "reader",
            path: "/",
        };
        const page = await openPage(home, address);
        const client = (fetch: (request: Request) => Promise<Response>) =>
            new DynamicObjectClient({
                objects: [article],
                scope: home.spaceId,
                endpoint: { url: `${page.origin}${PACKAGE_PATH}`, fetch, bookmark: new Bookmark() },
                environment,
            });
        const owner = client(page.fetch);
        const created = Identified.parse(
            await owner.call("article", "create", { title: "Roadmap" }),
        );

        // publish it to anyone holding a link's secret
        const { secret, digest } = await LinkSecret.create(environment);
        const granted = schema.looseObject({ id: schema.string() }).parse(
            await owner.call("article", "grant", {
                id: created.id,
                relation: "viewer",
                subject: anyone.reference("*", "*"),
                linkSecret: digest,
            }),
        );

        // open the reader by the link as an anonymous visitor, keeping its cookie
        const listener = home.computer.computer;
        const linked = await listener.fetch(new Request(`${page.origin}/reader?link=${secret}`));
        const cookie = nth(header(linked, "set-cookie").split(";"), 0);
        const visit = (cookies: string | undefined) =>
            client(async (request) => {
                const headers = new Headers(request.headers);
                headers.set("sec-fetch-site", "same-origin");
                if (cookies !== undefined) {
                    headers.set("cookie", cookies);
                }

                return listener.fetch(new Request(request, { headers }));
            });
        const read = (visitor: ReturnType<typeof visit>) =>
            refusal(visitor.call("article", "get", { id: created.id }));
        const withLink = Titled.parse(
            await visit(cookie).call("article", "get", { id: created.id }),
        ).title;
        const without = await read(visit(undefined));

        // revoke the link, then read by it again
        await owner.call("article", "revoke", { id: created.id, relationshipId: granted.id });
        const revoked = await read(visit(cookie));
        expect({ withLink, without, revoked }).toEqual({
            withLink: "Roadmap",
            without: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${home.spaceId}' not found`],
            revoked: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${home.spaceId}' not found`],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "limit each anonymous visitor's writes apart by the address their first proxy wrote, as anyone may edit a public article",
    async () => {
        // open the articles reader as the owner, create an article and let anyone edit it
        const home = await openHome();
        const { environment } = home.computer.device;
        await install(home, await articlesBuild(home.computer.environment.crypto), "articles");
        const page = await openPage(home, {
            space: home.address,
            installation: "articles",
            view: "reader",
            path: "/",
        });
        const client = (fetch: (request: Request) => Promise<Response>) =>
            new DynamicObjectClient({
                objects: [article],
                scope: home.spaceId,
                endpoint: { url: `${page.origin}${PACKAGE_PATH}`, fetch, bookmark: new Bookmark() },
                environment,
            });
        const owner = client(page.fetch);
        const created = Identified.parse(
            await owner.call("article", "create", { title: "Roadmap" }),
        );
        await owner.call("article", "grant", {
            id: created.id,
            relation: "editor",
            subject: anyone.reference("*", "*"),
        });

        // edit it as two anonymous visitors, each from the address its first proxy wrote
        const listener = home.computer.computer;
        const visitor = (address: string) =>
            client(async (request) => {
                const headers = new Headers(request.headers);
                headers.set("sec-fetch-site", "same-origin");
                headers.set("forwarded", `for=${address}`);

                return listener.fetch(new Request(request, { headers }));
            });
        const edit = (address: string, title: string) =>
            refusal(visitor(address).call("article", "update", { id: created.id, title }));
        const first = await edit("192.0.2.1", "Plan");
        const again = await edit("192.0.2.1", "Plans");
        const other = await edit("192.0.2.2", "Plans");

        // let each visitor edit once an hour, the second visitor apart from the first
        expect({ first, again, other }).toEqual({
            first: "done",
            again: [
                "TOO_MANY_REQUESTS",
                "RATE_EXHAUSTED",
                "'article.update' exceeds its limit of 1 calls per 3600000 ms",
            ],
            other: "done",
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "open an object in the view presenting its type most strongly, or in the view asked for, with the object as the view's target",
    async () => {
        // install notes, whose editor presents notes by default and whose home view offers them
        const opened = await openHome();
        const { device: native } = opened.computer;
        const listener = opened.computer.computer;
        await install(opened, await notesBuild(opened.computer.environment.crypto), "notes");
        const login = await native.launcher(opened.login);
        const note = ObjectReference.parse({
            packageId: notes.id,
            type: "note",
            scope: opened.spaceId,
            id: "note-01996ab0-0000-7000-8000-0000000000aa",
        });

        // open the note in the strongest view and in the home view, and a type no view presents
        const openIn = async (view?: string) => {
            const address = {
                space: opened.address,
                object: note,
                ...(view === undefined ? {} : { view }),
            };
            const launched = await login.launch(address);
            const traded = await listener.fetch(new Request(launched.url));
            const cookie = nth(header(traded, "set-cookie").split(";"), 0);
            const location = new URL(header(traded, "location"), launched.url);
            const document = await listener.fetch(
                new Request(location.href, { headers: { cookie } }),
            );
            const context = Launched.parse(await contextOf(document));

            return [location.pathname, context.view, context.target];
        };
        const strongest = await openIn();
        const asked = await openIn("home");
        const unpresented = await refusal(
            login.launch({ space: opened.address, object: { ...note, type: "folder" } }),
        );

        expect({ strongest, asked, unpresented }).toEqual({
            strongest: ["/editor/", "editor", note],
            asked: ["/home/", "home", note],
            unpresented: [
                "NOT_FOUND",
                "PRESENTER_NOT_FOUND",
                `presenter 'no view' for folder objects of '${opened.spaceId}' not found`,
            ],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "call a page's own package and a platform service its views declare as its person under their grant, refusing another origin and an undeclared service, and refusing an anonymous visitor the unshared space",
    async () => {
        // open the tasks board in the owner's home
        const home = await openHome();
        const installation = await installTasks(home);
        const page = await openPage(home, {
            space: home.address,
            installation: "tasks",
            view: "board",
            path: "/",
        });

        // create a task, complete it and list it through the page's own package
        const remote = new DynamicObjectClient({
            objects: [task],
            scope: home.spaceId,
            endpoint: { url: `${page.origin}${PACKAGE_PATH}`, fetch: page.fetch },
            environment: home.computer.device.environment,
        });
        const created = Identified.parse(await remote.call("task", "create", { title: "Water" }));
        await remote.call("task", "complete", { id: created.id });
        const listed = await remote.list("task");

        // read the installation through the declared stack service, which the grant does not give
        const call = (packageId: string, path: string) =>
            page.fetch(
                new Request(
                    `${page.origin}${PLATFORM_PATH}${PackageMount.path(packageId)}${path}`,
                    { headers: { [VERSION_HEADER]: stackPackage.version } },
                ),
            );
        const declared = await call(
            stackPackage.id,
            `/spaces/${home.spaceId}/rollouts/${installation.id}`,
        );
        const undeclared = await call(observabilityPackage.id, "/issues");
        const { database } = await home.computer.machine.space(home.spaceId);
        const [rolled] = await database
            .select({ id: stackObject.rollout.table.id })
            .from(stackObject.rollout.table)
            .where(eq(stackObject.rollout.table.installationId, installation.id));
        const { id: rolloutId } = present(rolled, "the tasks rollout");

        // refuse a call from another site, and an anonymous visitor without the page's cookie the unshared space
        const listener = home.computer.computer;
        const foreign = await listener.fetch(
            new Request(`${page.origin}${PACKAGE_PATH}/tasks`, {
                headers: { cookie: page.cookie, origin: "https://foreign.test" },
            }),
        );
        const visitor = new DynamicObjectClient({
            objects: [task],
            scope: home.spaceId,
            endpoint: {
                url: `${page.origin}${PACKAGE_PATH}`,
                fetch: (request) => {
                    const headers = new Headers(request.headers);
                    headers.set("origin", page.origin);

                    return listener.fetch(new Request(request, { headers }));
                },
            },
            environment: home.computer.device.environment,
        });
        const unshared = await refusal(visitor.list("task"));

        expect({
            listed: listed.map((row) => [row["title"], row["status"]]),
            declared: [declared.status, await declared.json()],
            undeclared: await summary(undeclared),
            foreign: await summary(foreign),
            unshared,
        }).toEqual({
            listed: [["Water", "done"]],
            declared: [
                403,
                {
                    defined: true,
                    code: "FORBIDDEN",
                    status: 403,
                    reason: "GRANT_DENIED",
                    domain: "@destack/object",
                    message: `'read' on rollout '${rolloutId}' is forbidden: grant '${page.grant.id}' does not give it`,
                    data: {
                        permission: stackObject.rollout.permission("read"),
                        object: stackObject.rollout.reference(home.spaceId, rolloutId),
                        grant: page.grant.id,
                    },
                },
            ],
            undeclared: answered(
                "FORBIDDEN",
                "VIEW_PACKAGE_DENIED",
                403,
                `package '${observabilityPackage.id}' is forbidden: no view of 'tasks' declares it`,
            ),
            foreign: answered(
                "FORBIDDEN",
                "ORIGIN_DENIED",
                403,
                "page origin is forbidden: it can only be called from the page itself",
            ),
            unshared: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${home.spaceId}' not found`],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "end a page's calls once the person revokes their grant to its installation, and once the client that launched it is revoked",
    async () => {
        // open the tasks board and list its tasks
        const home = await openHome();
        const { owner } = home.computer.world;
        await installTasks(home);
        const address = { space: home.address, installation: "tasks", view: "board", path: "/" };
        const page = await openPage(home, address);
        const bookmark = new Bookmark();
        const list = (opened: typeof page) =>
            refusal(
                new DynamicObjectClient({
                    objects: [task],
                    scope: home.spaceId,
                    endpoint: {
                        url: `${opened.origin}${PACKAGE_PATH}`,
                        fetch: opened.fetch,
                        bookmark,
                    },
                    environment: home.computer.device.environment,
                }).list("task"),
            );
        const before = await list(page);

        // revoke the grant, grant it again, then revoke the launching client
        const accounts = page.session.session.accounts();
        const objects = createClient(
            objectPackage,
            {
                ...page.session.session.endpoint(PackageId.parse(objectPackage.id)),
                bookmark,
            },
            home.computer.device.environment,
        );
        await objects.grant.revoke({
            scope: owner.id,
            id: page.grant.id,
            requestId: RequestId.create(home.computer.environment),
        });

        // refuse the page's next token once its current one lapses
        const { clock } = home.computer.environment;
        await home.computer.world.simulation.wait(clock.sleep(TOKEN_LIFETIME_MILLISECONDS + 1));
        const revoked = await list(page);
        const regranted = await openPage(home, address);
        await accounts.client.revoke({
            userId: owner.id,
            id: present(regranted.signedIn.row.clientId, "the login's client"),
            requestId: RequestId.create(home.computer.environment),
        });
        const signedOut = await list(regranted);

        expect({ before, revoked, signedOut }).toEqual({
            before: "done",
            revoked: [
                "BAD_REQUEST",
                "GRANT_INVALID",
                `the token endpoint refused the request: invalid_grant: authentication required: grant '${page.grant.id}' is revoked`,
            ],
            signedOut: [
                "BAD_REQUEST",
                "GRANT_INVALID",
                "the token endpoint refused the request: invalid_grant: cannot use client: it is revoked",
            ],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "serve the start page with its session cookie after its computer restarts, its document and its home path's calls answering",
    async () => {
        // open the start page, then restart the computer serving the home
        const home = await openHome();
        const page = await openPage(home, {
            space: home.address,
            installation: "start",
            view: "start",
            path: "/",
        });
        const restarted = await home.computer.restart();
        await restarted.ready(home.spaceId);

        // request the document and list the space's installations with the cookie from before the restart
        const send = (request: Request) => {
            const headers = new Headers(request.headers);
            headers.set("cookie", page.cookie);
            headers.set("sec-fetch-site", "same-origin");
            headers.set("host", new URL(page.origin).host);

            return restarted.computer.fetch(new Request(request, { headers }));
        };
        const location = new URL(header(page.traded, "location"), page.launched.url).href;
        const document = await send(new Request(location));
        await document.body?.cancel();
        const objects = createClient(
            objectPackage,
            {
                url: `${page.origin}${HOME_PATH}/${home.spaceId}${PackageMount.path(objectPackage.id)}`,
                fetch: send,
            },
            restarted.environment,
        );
        const installations = await objects.installation.list({ scope: home.spaceId });

        // launch the start page afresh after the restart from the browser keeping its cookie, as a window the desktop restores does
        const login = await restarted.device.launcher(home.login);
        const launched = await login.launch({
            space: home.address,
            installation: "start",
            view: "start",
            path: "/",
        });
        const traded = await restarted.computer.fetch(
            new Request(launched.url, { headers: { cookie: page.cookie } }),
        );
        await traded.body?.cancel();

        expect({
            traded: traded.status,
            document: document.status,
            installations: installations.items.map((row) => row.name).toSorted(),
        }).toEqual({
            traded: 303,
            document: 200,
            installations: [
                "connection",
                "inbox",
                "message",
                "observability",
                "social",
                "stack",
                "start",
                "vault",
            ],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "list a space's rollouts and installations through its home path, at the packages its store serves",
    async () => {
        // open the start page, whose grant lists the space's installations and rollouts
        const home = await openHome();
        const page = await openPage(home, {
            space: home.address,
            installation: "start",
            view: "start",
            path: "/",
        });
        const below = (packageId: PackageId) => ({
            url: `${page.origin}${HOME_PATH}/${home.spaceId}${PackageMount.path(packageId)}`,
            fetch: (request: Request) => {
                // send the page's own host, as a browser does
                const headers = new Headers(request.headers);
                headers.set("host", new URL(page.origin).host);

                return page.fetch(new Request(request, { headers }));
            },
        });
        const stack = createClient(stackPackage, below(stackPackage.id), home.computer.environment);
        const objects = createClient(
            objectPackage,
            below(objectPackage.id),
            home.computer.environment,
        );

        // list the applied rollouts and the enabled installations, each rollout of an enabled installation
        const rollouts = await stack.rollout.list({
            spaceId: home.spaceId,
            where: { appliedRevisionId: { isNotNull: true } },
        });
        const installations = await objects.installation.list({
            scope: home.spaceId,
            where: { status: "enabled" },
        });
        const enabled = new Set(installations.items.map((row) => row.id));
        expect(rollouts.items.filter((row) => !enabled.has(row.installationId))).toEqual([]);
    },
    SCENARIO_MILLISECONDS,
);

test(
    "refuse a page's push to a scope's home for a call outside the view's grant",
    async () => {
        // open the start page, whose grant gives the owner's settings alone
        const home = await openHome();
        const { owner } = home.computer.world;
        const page = await openPage(home, {
            space: home.address,
            installation: "start",
            view: "start",
            path: "/",
        });

        // push revoking the owner's client, which the start page's grant does not allow
        const call = accountObject.client.call("revoke", {
            ...accountObject.client.target({
                scope: user.identifier(owner.id),
                id: schema.identifier("client").parse(page.signedIn.row.clientId),
            }),
        });
        const response = await page.fetch(
            new Request(`${page.origin}${HOME_PATH}/${owner.id}/store/push`, {
                method: "POST",
                headers: { [VERSION_HEADER]: accountPackage.version },
                body: JSON.stringify({
                    scope: owner.id,
                    mutations: [{ id: RequestId.create(home.computer.environment), calls: [call] }],
                }),
            }),
        );

        // refuse the call at the universe as outside the start page's grant, which revokes no clients
        const client = schema.identifier("client").parse(page.signedIn.row.clientId);
        expect([response.status, await response.json()]).toEqual([
            403,
            {
                defined: true,
                code: "FORBIDDEN",
                status: 403,
                reason: "GRANT_DENIED",
                domain: "@destack/object",
                message: `'revoke' on client '${client}' is forbidden: grant '${page.grant.id}' does not give it`,
                data: {
                    permission: accountObject.client.permission("revoke"),
                    object: accountObject.client.reference(owner.id, client),
                    grant: page.grant.id,
                },
            },
        ]);
    },
    SCENARIO_MILLISECONDS,
);

test(
    "register the browser as the person's client from a view granted it, refusing a view granted none",
    async () => {
        // open the start page and the tasks board
        const home = await openHome();
        await installTasks(home);
        const homePage = await openPage(home, {
            space: home.address,
            installation: "start",
            view: "start",
            path: "/",
        });
        const board = await openPage(home, {
            space: home.address,
            installation: "tasks",
            view: "board",
            path: "/",
        });

        // register a browser client from the start page, granted it, then from the board, granted none
        const person = (await home.computer.device.launcher(home.login)).user.subject.id;
        const client = Identifier.create("client", home.computer.device.environment);
        const registered = await registerClient(homePage, client);
        const refused = await registerClient(
            board,
            Identifier.create("client", home.computer.device.environment),
        );

        expect({
            registered: [registered.status, await registered.json()],
            refused: await summary(refused),
        }).toEqual({
            registered: [201, { client }],
            refused: answered(
                "NOT_FOUND",
                "SCOPE_NOT_FOUND",
                404,
                `scope '${person}' not found`,
                "@destack/object",
            ),
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "bind the start page to a registered client by a proof of its key for the bind, refusing a proof for another request, and answer a removed client as gone",
    async () => {
        // open the start page, and register a browser client and its key from it
        const home = await openHome();
        const page = await openPage(home, {
            space: home.address,
            installation: "start",
            view: "start",
            path: "/",
        });
        const { environment } = home.computer.device;
        const { pair } = await KeyPair.generate(environment.crypto);
        const client = Identifier.create("client", environment);
        const now = () => environment.clock.now();
        await registerClient(page, client);
        const keyed = await page.fetch(
            new Request(`${page.origin}/.destack/client/key`, {
                method: "POST",
                body: JSON.stringify({
                    client,
                    publicKey: pair.publicKey,
                    proof: await Proof.sign(
                        pair.privateKey,
                        pair.publicKey,
                        client,
                        now(),
                        environment,
                    ),
                }),
            }),
        );

        // bind with a proof of the bind request, and with one of another request
        const url = `${page.origin}/.destack/client/bind`;
        const bind = async (target: string) =>
            page.fetch(
                new Request(url, {
                    method: "POST",
                    headers: {
                        [PROOF_HEADER]: await Proof.sign(
                            pair.privateKey,
                            pair.publicKey,
                            client,
                            now(),
                            environment,
                            { method: "POST", url: target },
                        ),
                    },
                }),
            );
        const bound = await bind(url);
        const cookie = nth(header(bound, "set-cookie").split(";"), 0);
        const location = new URL(header(page.traded, "location"), page.launched.url).href;
        const document = await home.computer.computer.fetch(
            new Request(location, { headers: { cookie } }),
        );
        await document.body?.cancel();
        const elsewhere = await bind(`${page.origin}/.destack/client/elsewhere`);

        // remove the client as the person, and bind again
        await page.session.session.accounts().client.revoke({
            userId: home.computer.world.owner.id,
            id: client,
            requestId: RequestId.create(environment),
        });
        const removed = await bind(url);

        expect({
            keyed: keyed.status,
            bound: [bound.status, await bound.json()],
            document: document.status,
            elsewhere: await summary(elsewhere),
            removed: await summary(removed),
        }).toEqual({
            keyed: 200,
            bound: [200, { client }],
            document: 200,
            elsewhere: [
                400,
                JSON.stringify({
                    defined: false,
                    code: "BAD_REQUEST",
                    status: 400,
                    message: "key proof is for another request",
                    reason: "PROOF_INVALID",
                    domain: "@destack/identity",
                }),
            ],
            removed: [410, ""],
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "launch a view of a space another machine serves at that machine, whose origin opens it",
    async () => {
        // serve the owner's home here and a work space they joined on another machine
        const home = await openHome();
        const work = await openWork(home);
        await installTasks(work);

        // launch the work space's board from this computer and open it at the work machine
        const { device: native } = home.computer;
        const login = await native.launcher(home.login);
        const address = { space: work.address, installation: "tasks", view: "board", path: "/" };
        const launched = await login.launch(address);
        const traded = await home.computer.browse(new Request(launched.url));
        const cookie = nth(header(traded, "set-cookie").split(";"), 0);
        const document = await home.computer.browse(
            new Request(new URL(header(traded, "location"), launched.url).href, {
                headers: { cookie },
            }),
        );
        await Promise.all([traded.body?.cancel(), document.body?.cancel()]);

        expect({
            host: new URL(launched.url).host,
            traded: [traded.status, header(traded, "location")],
            document: document.status,
        }).toEqual({
            host: `tasks--${work.address}.${DOMAINS.space}`,
            traded: [303, "/board"],
            document: 200,
        });
    },
    SCENARIO_MILLISECONDS,
);

test(
    "call an installation of a space another machine serves through the native device's platform route, with the universe's token for its space",
    async () => {
        // serve the owner's home here and a work space they joined on another machine
        const home = await openHome();
        const work = await openWork(home);
        await installTasks(work);

        // create and list tasks in the work space through the route the CLI's platform calls take
        const { device: native } = home.computer;
        const remote = new DynamicObjectClient({
            objects: [task],
            scope: work.spaceId,
            endpoint: await (
                await native.launcher(home.login)
            ).installation(work.spaceId, "tasks", tasks.id),
            environment: home.computer.device.environment,
        });
        await remote.call("task", "create", { title: "Write the report" });
        const listed = await remote.list("task");

        expect(listed.map((row) => [row["title"], row["status"]])).toEqual([
            ["Write the report", "open"],
        ]);
    },
    SCENARIO_MILLISECONDS,
);

/** Encode a package's German catalog of one greeting message. */
function catalog(packageId: string, message: string): Uint8Array<ArrayBuffer> {
    const written = { package: packageId, locale: "de", messages: { greeting: message } };

    return new TextEncoder().encode(JSON.stringify(written));
}

/** A catalog as the origin serves it: its package, language and messages. */
const ServedCatalog = schema.looseObject({
    package: schema.string(),
    locale: schema.string(),
    messages: schema.record(schema.string(), schema.string()),
});

test(
    "serve a view's build with the German catalogs of its package and of a release dependency, and no French ones",
    async () => {
        // publish a widget library with its German catalog, and install a greeting view pinning it
        const { crypto } = testHost();
        const widgets = await TestViewBuild.of(
            WIDGETS,
            {},
            { files: new Map([["locale/de.json", catalog(WIDGETS.id, "Schließen")]]) },
            crypto,
        );
        const greeting = await TestViewBuild.of(
            GREETING,
            {
                greeting: {
                    entrypoint: "browser/greeting.js",
                    stylesheets: [],
                    sandbox: {},
                    presents: [],
                    categories: [],
                    audience: "person" as const,
                    packages: [],
                    render: { mode: "client" },
                },
            },
            {
                files: new Map([["locale/de.json", catalog(GREETING.id, "Willkommen zurück")]]),
                dependencies: [widgets],
            },
            crypto,
        );
        const opened = await openHome();
        const { world } = opened.computer;
        await opened.cloud.publish(widgets, world.owner);
        await install(opened, greeting, "greeting");

        // open the greeting view and read its own and the library's German catalogs at its origin
        const page = await openPage(opened, {
            space: opened.address,
            installation: "greeting",
            view: "greeting",
            path: "/",
        });
        const address = new URL(header(page.traded, "location"), page.launched.url).href;
        const served = await Promise.all(
            ["/locale/de.json", `/locale/${WIDGETS.id}/de.json`].map(async (path) => {
                const response = await page.fetch(new Request(new URL(path, address).href));
                const read = ServedCatalog.parse(await response.json());

                return [read.package, read.messages["greeting"]];
            }),
        );
        // read the French catalogs neither package has, which the view's loader takes as none
        const missing = await Promise.all(
            ["/locale/fr.json", `/locale/${WIDGETS.id}/fr.json`].map(async (path) => {
                const response = await page.fetch(new Request(new URL(path, address).href));
                await response.body?.cancel();

                return response.status;
            }),
        );
        expect({ served, missing }).toEqual({
            served: [
                [GREETING.id, "Willkommen zurück"],
                [WIDGETS.id, "Schließen"],
            ],
            missing: [404, 404],
        });
    },
    SCENARIO_MILLISECONDS,
);

/** The resolver the universe reads DNS over HTTPS from, Cloudflare's. */
const RESOLVER = "cloudflare-dns.com";

/** The DNS response code of an answered lookup (RFC 1035 4.1.1). */
const NO_ERROR = 0;

/** The resource record type of TXT records (RFC 1035 3.2.2). */
const TXT = 16;

/** The domain the owner verifies and serves a site's hostname below. */
const SITE_DOMAIN = "example.com";

/** Write a site's build in memory: a static view's pages, an asset and a not-found page. */
function siteBuild(crypto: Crypto): Promise<BuildReader> {
    const encoder = new TextEncoder();
    const files = new Map([
        ["browser/view/site/index.html", encoder.encode("<head></head><h1>Home</h1>")],
        ["browser/view/site/blog/hello/index.html", encoder.encode("<h1>Hello</h1>")],
        ["browser/view/site/assets/site.js", encoder.encode("export {};")],
        ["browser/view/site/404.html", encoder.encode("<h1>Not found</h1>")],
        [
            "browser/view/site/_redirects",
            encoder.encode("/install https://download.example/install\n/old/* /:splat 301\n"),
        ],
        [
            "browser/view/site/_headers",
            encoder.encode(
                "/assets/*\n  X-Robots-Tag: noindex\n  Permissions-Policy: camera=*\n  Cache-Control: public, max-age=60\n",
            ),
        ],
        ["src/server.ts", encoder.encode("export const secret = 1;")],
    ]);
    const site = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000d1",
        name: "@acme/site",
        version: "2026.9.0-dev.1",
    });

    return TestViewBuild.of(
        site,
        {
            site: {
                entrypoint: "browser/assets/hydrate.js",
                stylesheets: [],
                sandbox: {},
                presents: [],
                categories: [],
                audience: "person" as const,
                packages: [],
                render: { mode: "static" },
            },
        },
        { files },
        crypto,
    );
}

/** Verify the owner's domain by its TXT record from a resolver in the world, and add a hostname. */
async function verifyHostname(served: Served, name: string): Promise<void> {
    // answer the universe's DNS over HTTPS lookups from the published records
    const { simulation, universe, owner } = served.computer.world;
    const dns = new TestDns();
    simulation.network.server(RESOLVER).http({
        hostname: RESOLVER,
        port: 443,
        fetch: async (request) => {
            const asked = new URL(request.url).searchParams.get("name") ?? "";
            const records = await dns.txt(asked);

            return Response.json({
                Status: NO_ERROR,
                Answer: records.map((data) => ({
                    name: asked,
                    type: TXT,
                    data: JSON.stringify(data),
                })),
            });
        },
    });

    // claim and verify the domain as the owner, entitled to custom domains, then add the hostname
    await universe.entitle(owner.account, "domain", { kind: "static", value: true });
    const accounts = owner.client(accountPackage);
    const requestId = () => RequestId.create(universe.environment);
    const claimed = await accounts.domain.create({
        scope: owner.account,
        requestId: requestId(),
        name: SITE_DOMAIN,
    });
    dns.publish({ scope: owner.account, name: SITE_DOMAIN });
    await accounts.domain.verify({ scope: owner.account, id: claimed.id, requestId: requestId() });
    await accounts.hostname.create({ accountId: owner.account, requestId: requestId(), name });
    await simulation.idle();
}

/** Create a route in a space as its owner, serving it once the space applies it. */
async function createRoute(
    served: Served,
    route: { readonly hostname: string; readonly installationId: Identifier<"installation"> },
): Promise<void> {
    // create the route as the owner, at the machine serving the space
    const { device: native, environment } = served.computer;
    const login = await native.launcher(served.login);
    const place = await native.locate(served.spaceId);
    const stacks = createClient(stackPackage, login.endpoint(stackPackage.id, place), environment);
    await stacks.route.create({
        spaceId: served.spaceId,
        requestId: RequestId.create(environment),
        hostname: route.hostname,
        path: "/",
        match: "prefix",
        installationId: route.installationId,
        rules: [{ entry: { kind: "origin" } }],
    });
    await served.computer.world.simulation.idle();
}

test(
    "serve a site at a route of its account's verified hostname to an anonymous visitor: its pages, an asset below them and its not-found page",
    async () => {
        // serve a space of the owner's account on the residency's cloud machine, which its hostnames reach
        const home = await openHome();
        const { computer } = home;
        const { owner } = computer.world;
        const login = (await computer.device.signedIn(home.login)).row;
        const spaceId = await computer.space(login, owner.account, "site");
        const opened = { ...home, spaceId, address: `site--${owner.handle}` };
        const hostname = `www.${SITE_DOMAIN}`;
        await verifyHostname(opened, hostname);

        // wait until the space's replica of its account's hostnames has the new one
        const database = (await computer.machine.space(spaceId)).database;
        await computer.waitUntil(database, async () => {
            const [found] = await database
                .select({ id: accountObject.hostname.table.id })
                .from(accountObject.hostname.table)
                .where(eq(accountObject.hostname.table.name, hostname));

            return found !== undefined;
        });

        // install the site and route the whole hostname to it
        const site = await install(
            opened,
            await siteBuild(opened.computer.environment.crypto),
            "site",
        );
        await createRoute(opened, { hostname, installationId: site.id });

        // visit as an anonymous visitor through the machine, reading each answer
        const visit = async (path: string, init: RequestInit = {}) => {
            const response = await computer.computer.fetch(
                new Request(`https://${hostname}${path}`, init),
            );
            const location = response.headers.get("location");

            return [response.status, location ?? (await response.text())];
        };
        const etag = (
            await computer.computer.fetch(new Request(`https://${hostname}/assets/site.js`))
        ).headers.get("etag");

        // serve pages with the installation's import map and assets as Cloudflare's auto-trailing-slash does, revalidated by ETag, reads only, no source,
        //  the site's headers below the installation's browser features, redirects staying on the origin,
        //  and refuse a launch and a link, which the installation's own origin takes
        expect({
            root: await visit("/"),
            slashed: await visit("/blog/hello/"),
            unslashed: await visit("/blog/hello"),
            index: await visit("/blog/hello/index.html"),
            asset: await visit("/assets/site.js"),
            assetHeaders: Object.fromEntries(
                [
                    ...(
                        await computer.computer.fetch(
                            new Request(`https://${hostname}/assets/site.js`),
                        )
                    ).headers,
                ].filter(([name]) =>
                    ["x-robots-tag", "permissions-policy", "cache-control"].includes(name),
                ),
            ),
            installed: await visit("/install"),
            moved: await visit("/old/blog/hello/"),
            escaping: await visit("/old//evil.example/"),
            rules: await visit("/_redirects"),
            unchanged: await visit("/assets/site.js", { headers: { "if-none-match": etag ?? "" } }),
            head: await visit("/assets/site.js", { method: "HEAD" }),
            posted: await visit("/assets/site.js", { method: "POST" }),
            missing: await visit("/nowhere"),
            outside: await visit("//evil.example/blog/hello"),
            source: await visit("/src/server.ts"),
            launched: await visit("/?token=forged"),
            linked: await visit(`/?link=${"a".repeat(64)}`),
        }).toEqual({
            root: [
                200,
                '<head><script type="importmap">{"imports":{},"scopes":{}}</script></head><h1>Home</h1>',
            ],
            slashed: [200, "<h1>Hello</h1>"],
            unslashed: [307, "/blog/hello/"],
            index: [307, "/blog/hello/"],
            asset: [200, "export {};"],
            assetHeaders: {
                "x-robots-tag": "noindex",
                "permissions-policy":
                    "camera=(), microphone=(), geolocation=(), display-capture=(), clipboard-read=(), clipboard-write=(), fullscreen=(), midi=(), usb=(), hid=(), serial=(), bluetooth=(), screen-wake-lock=(), idle-detection=(), local-fonts=(), window-management=(), xr-spatial-tracking=()",
                "cache-control": "public, max-age=60",
            },
            installed: [302, "https://download.example/install"],
            moved: [301, "/blog/hello/"],
            escaping: [301, "/evil.example/"],
            rules: [404, "<h1>Not found</h1>"],
            unchanged: [304, ""],
            head: [200, ""],
            posted: [405, ""],
            missing: [404, "<h1>Not found</h1>"],
            outside: [404, "<h1>Not found</h1>"],
            source: [404, "<h1>Not found</h1>"],
            launched: answered(
                "FORBIDDEN",
                "INSTALLATION_ORIGIN_DENIED",
                403,
                "view is forbidden: a route serves anonymous visitors, so open the view at its installation's origin",
            ),
            linked: answered(
                "FORBIDDEN",
                "INSTALLATION_ORIGIN_DENIED",
                403,
                "link is forbidden: a route serves anonymous visitors, so open links at the installation's origin",
            ),
        });
    },
    SCENARIO_MILLISECONDS,
);
