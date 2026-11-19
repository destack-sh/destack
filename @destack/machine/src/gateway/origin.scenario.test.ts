import { testIdentity } from "@destack/object/test";
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
    EDITION_HEADER,
    VERSION_HEADER,
    Bookmark,
} from "@destack/object";
import objectPackage from "@destack/object/package";
import { createClient, ProcedureRemote } from "@destack/object/client";
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
import { type Address, HOME_PATH, PLATFORM_PATH, ViewAddress } from "@destack/view/declare";
import viewPackage from "@destack/view/destack.json" with { type: "json" };
import { LINK_COOKIE, SESSION_COOKIE } from "./origin.ts";
import { OWNER_EMAIL, PUBLISHER, TestComputer } from "../computer/test/index.ts";
import { TestViewBuild } from "./test/view.ts";
import { argument, plural, t } from "@destack/locale";
import { presentable, shareable } from "@destack/object";

/** The address of a second person signing in on the owner's computer. */
const MEMBER_EMAIL = "member@example.test";

/** The greeting package, whose view reads its own catalogs and its widget library's. */
const GREETING = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d2",
    name: "@acme/greeting",
    version: "2026.9.0-dev.20260912100001.0123456789ab",
});

/** The widget library the greeting package requires as a release, with catalogs of its own. */
const WIDGETS = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d3",
    name: "@acme/widgets",
    version: "2026.9.0-dev.20260912100001.0123456789ab",
});

/** The notes package whose views the launches open. */
const notes = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000e1",
    name: "@acme/notes",
    version: "2026.9.0-dev.20260912100001.0123456789ab",
});

/** The notes package: it reads the person's settings, writes them once asked, and scans notes. */
const notesPackage = definePackage(
    {
        ...testIdentity,
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
    version: "2026.10.0-dev.20260912100001.0123456789ab",
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
        ...testIdentity,
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
    version: "2026.10.0-dev.20260912100001.0123456789ab",
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
        ...testIdentity,
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
    /** The home of the space's account's residency, which its views read. */
    readonly residencyHome: Identifier<"space">;
}

/** Serve the owner's home on its cloud computer, with the organization publishing the packages. */
async function openHome(): Promise<Served> {
    const { home, login } = await TestComputer.home({ organizations: [PUBLISHER] });
    const { owner } = home.world;
    const address = `home--${owner.handle}`;
    const accounts = await home.machine.accounts();
    const residencyHome = schema
        .identifier("space")
        .parse(await accounts.residency.locate({ account: owner.account }));

    return { computer: home, cloud: home, login, spaceId: owner.home, address, residencyHome };
}

/** Serve a work space of the owner's account on their own computer in the home's world. */
async function openWork(home: Served): Promise<Served> {
    // enroll the owner's computer through the relay in their account, and create the space on it
    const relay = await TestComputer.relay(home.computer.world);
    const computer = await TestComputer.open({ world: home.computer.world, relay: relay.url });
    const { owner } = computer.world;
    const signedIn = await computer.signIn(OWNER_EMAIL);
    await computer.enroll(signedIn, owner.account);
    const spaceId = await computer.space(signedIn.id, owner.account, "work");

    const address = `work--${owner.handle}`;

    return {
        computer,
        cloud: home.cloud,
        login: signedIn.id,
        spaceId,
        address,
        residencyHome: home.residencyHome,
    };
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
async function install(
    served: Served,
    build: BuildReader,
    name: string,
    builds: readonly BuildReader[] = [],
) {
    // lock the build with the builds it runs with
    const { world } = served.computer;
    await served.cloud.publish(await world.distribution.lock(build, builds), world.owner);

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

test("launch a view onto a session cookie serving its document", async () => {
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
    const granted = await login.granted(principal.installation.reference(spaceId, installation.id));
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

    // open the origin's root, which serves the main view, and load a page of no view
    const port = new URL(await native.origin()).port;
    const root = await listener.fetch(
        new Request(`http://notes--${opened.address}.localhost:${port}/`),
    );
    const missing = await listener.fetch(
        new Request(`http://notes--${opened.address}.localhost:${port}/missing`, {
            headers: { "sec-fetch-mode": "navigate", "sec-fetch-dest": "document" },
        }),
    );

    // open a note in the view presenting notes most strongly and in the view asked for
    const note = ObjectReference.parse({
        packageId: notes.id,
        type: "note",
        scope: spaceId,
        id: "note-01996ab0-0000-7000-8000-0000000000aa",
    });
    const openIn = async (view?: string) => {
        const object = await login.launch({
            space: opened.address,
            object: note,
            ...(view === undefined ? {} : { view }),
        });
        const objectTraded = await listener.fetch(new Request(object.url));
        const objectCookie = nth(header(objectTraded, "set-cookie").split(";"), 0);
        const location = new URL(header(objectTraded, "location"), object.url);
        const objectDocument = await listener.fetch(
            new Request(location.href, { headers: { cookie: objectCookie } }),
        );
        const objectContext = Launched.parse(await contextOf(objectDocument));

        return [location.pathname, objectContext.view, objectContext.target];
    };
    const strongest = await openIn();
    const asked = await openIn("home");

    // refuse a type no view presents
    const unpresented = await refusal(
        login.launch({ space: opened.address, object: { ...note, type: "folder" } }),
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
        root: [root.status, Launched.pick({ view: true }).parse(await contextOf(root)).view],
        missing: [missing.status, missing.headers.get("content-type")],
        strongest,
        asked,
        unpresented,
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
            residencyHome: opened.residencyHome,
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
        root: [200, "home"],
        missing: [404, "text/html; charset=utf-8"],
        strongest: ["/editor/", "editor", note],
        asked: ["/home/", "home", note],
        unpresented: [
            "NOT_FOUND",
            "PRESENTER_NOT_FOUND",
            `presenter 'no view' for folder objects of '${spaceId}' not found`,
        ],
    });
});

test("serve anonymous visitors and refuse forged credentials", async () => {
    // install notes in the owner's home
    const opened = await openHome();
    const { computer, spaceId } = opened;
    const listener = computer.computer;
    const installation = await install(
        opened,
        await notesBuild(computer.environment.crypto),
        "notes",
    );
    const port = new URL(await computer.device.origin()).port;
    const origin = `http://notes--${opened.address}.localhost:${port}`;

    // serve the page without a cookie to an anonymous visitor, and take their telemetry
    const unopened = await listener.fetch(new Request(`${origin}/home`));
    const telemetry = await listener.fetch(
        new Request(`${origin}${OTLP_ORIGIN_PATH}/v1/traces`, {
            method: "POST",
            headers: { origin, "content-type": "application/json" },
            body: "{}",
        }),
    );

    // refuse a launch with a forged token and a session cookie the space did not sign
    const forgedToken = await listener.fetch(new Request(`${origin}/home?token=forged`));
    const forgedSession = await listener.fetch(
        new Request(`${origin}/home`, { headers: { cookie: `${SESSION_COOKIE}=forged` } }),
    );
    expect({
        unopened: [unopened.status, await contextOf(unopened)],
        telemetry: await summary(telemetry),
        forgedToken: forgedToken.status,
        forgedSession: [...(await summary(forgedSession)), header(forgedSession, "set-cookie")],
    }).toEqual({
        unopened: [
            200,
            {
                installation: installation.id,
                space: spaceId,
                account: computer.world.owner.account,
                residencyHome: opened.residencyHome,
                view: "home",
                base: "",
                universe: computer.world.universe.origin,
                catalogs: [],
                user: ANONYMOUS,
            },
        ],
        telemetry: [200, "{}"],
        forgedToken: 401,
        forgedSession: [
            ...answered(
                "UNAUTHORIZED",
                "SESSION_UNAUTHENTICATED",
                401,
                "authentication required: the page's session is invalid or expired",
            ),
            `${SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0`,
        ],
    });
});

test("ask a signed-out member at the issuer for consent and permissions", async () => {
    // install notes in the owner's work space, whose origin the person they shared it with loads signed out
    const opened = await openShared(await openHome());
    const { computer } = opened;
    const { universe } = computer.world;
    const listener = computer.computer;
    await install(opened, await notesBuild(computer.environment.crypto), "notes");
    const port = new URL(await computer.device.origin()).port;
    const origin = `http://notes--${opened.address}.localhost:${port}`;
    const document = { "sec-fetch-mode": "navigate", "sec-fetch-dest": "document" };
    const loaded = await listener.fetch(new Request(`${origin}/`, { headers: document }));

    // sign the member in at the issuer, whose code returns to the origin, which asks them for the grant
    const member = await universe.signIn(MEMBER_EMAIL);
    const follow = async (from: Response, cookie: string) => {
        const signedIn = await universe.approve(header(from, "location"), member);

        return listener.fetch(new Request(signedIn, { headers: { cookie } }));
    };
    const asking = await follow(loaded, firstCookie(loaded));

    // consent to the grant at the issuer, then trade the launch for the origin's session and load the page
    const completed = await follow(asking, firstCookie(asking));
    const traded = await listener.fetch(new Request(header(completed, "location")));
    const session = firstCookie(traded);
    const page = await listener.fetch(
        new Request(new URL(header(traded, "location"), origin).href, {
            headers: { cookie: session },
        }),
    );

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
        asking: [asking.status, new URL(header(asking, "location")).pathname],
        completed: completed.status,
        page: [page.status, Launched.pick({ view: true }).parse(await contextOf(page)).view],
        asked: [asked.status, new URL(header(asked, "location")).pathname],
        permissions: details.authorization_details.flatMap((detail) =>
            detail.permissions.map((permission) => permission.name),
        ),
        returned: new URL(header(returned, "location"), origin).pathname,
    }).toEqual({
        asking: [302, "/oauth/oauth2/authorize"],
        completed: 303,
        page: [200, "home"],
        asked: [302, "/oauth/oauth2/authorize"],
        permissions: ["read", "read", "write", "write"],
        returned: "/editor",
    });
});

test("sign a signed-out page load in at the issuer", async () => {
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
});

test("sign a page load with a foreign session in again, and open a view at its own origin", async () => {
    // install notes in the owner's home
    const opened = await openHome();
    const { computer } = opened;
    const listener = computer.computer;
    await install(opened, await notesBuild(computer.environment.crypto), "notes");
    const port = new URL(await computer.device.origin()).port;
    const origin = `http://notes--${opened.address}.localhost:${port}`;

    // load a page with a session cookie the space did not sign, as an earlier universe's
    const loaded = await listener.fetch(
        new Request(`${origin}/`, {
            headers: {
                cookie: `${SESSION_COOKIE}=forged`,
                "sec-fetch-mode": "navigate",
                "sec-fetch-dest": "document",
            },
        }),
    );

    // open notes' view through the open path of an installation's origin, at its own origin
    const path = ViewAddress.openPath({
        space: opened.address,
        installation: "notes",
        view: "home",
        path: "/",
    });
    const redirected = await listener.fetch(new Request(`${origin}${path}`));

    expect({
        loaded: [loaded.status, new URL(header(loaded, "location")).pathname],
        redirected: [redirected.status, header(redirected, "location")],
    }).toEqual({
        loaded: [302, "/oauth/oauth2/authorize"],
        // at the origin of the machine's public endpoint, where the directory locates the space
        redirected: [303, `https://notes--${opened.address}.destack.space/home`],
    });
});

test("serve a link-shared article to holders of its secret", async () => {
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
        ProcedureRemote.at({
            scope: home.spaceId,
            endpoint: { url: `${page.origin}${PACKAGE_PATH}`, fetch, bookmark: new Bookmark() },
            environment,
        });
    const owner = client(page.fetch);
    const created = Identified.parse(await owner.run(article, "create", { title: "Roadmap" }));

    // publish it to anyone holding a link's secret
    const { secret, digest } = await LinkSecret.create(environment);
    const granted = schema.looseObject({ id: schema.string() }).parse(
        await owner.run(article, "grant", {
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
        refusal(visitor.run(article, "get", { id: created.id }));
    const withLink = Titled.parse(
        await visit(cookie).run(article, "get", { id: created.id }),
    ).title;
    const without = await read(visit(undefined));

    // open the reader by a malformed secret, and by a path a browser reads as another host
    const malformed = await listener.fetch(new Request(`${page.origin}/reader?link=forged`));
    const outside = await listener.fetch(
        new Request(`${page.origin}//evil.example/reader?link=${secret}`),
    );

    // revoke the link, then read by it again
    await owner.run(article, "revoke", { id: created.id, relationshipId: granted.id });
    const revoked = await read(visit(cookie));
    expect({
        linked: [linked.status, header(linked, "location"), header(linked, "set-cookie")],
        outside: header(outside, "location"),
        malformed: await summary(malformed),
        withLink,
        without,
        revoked,
    }).toEqual({
        linked: [303, "/reader", `${LINK_COOKIE}=${secret}; Path=/; HttpOnly; SameSite=Lax`],
        outside: "/evil.example/reader",
        malformed: answered(
            "FORBIDDEN",
            "LINK_SECRET_DENIED",
            403,
            "link secret is forbidden: it does not match",
            "@destack/access",
        ),
        withLink: "Roadmap",
        without: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${home.spaceId}' not found`],
        revoked: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${home.spaceId}' not found`],
    });
});

test("rate-limit anonymous writes by the first proxy's address", async () => {
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
        ProcedureRemote.at({
            scope: home.spaceId,
            endpoint: { url: `${page.origin}${PACKAGE_PATH}`, fetch, bookmark: new Bookmark() },
            environment,
        });
    const owner = client(page.fetch);
    const created = Identified.parse(await owner.run(article, "create", { title: "Roadmap" }));
    await owner.run(article, "grant", {
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
        refusal(visitor(address).run(article, "update", { id: created.id, title }));
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
});

test("call a page's package and declared services under its grant", async () => {
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
    const remote = ProcedureRemote.at({
        scope: home.spaceId,
        endpoint: { url: `${page.origin}${PACKAGE_PATH}`, fetch: page.fetch },
        environment: home.computer.device.environment,
    });
    const created = Identified.parse(await remote.run(task, "create", { title: "Water" }));
    await remote.run(task, "complete", { id: created.id });
    const listed = await remote.list(task);

    // read the installation through the declared stack service, which the grant does not give
    const call = (packageId: string, path: string) =>
        page.fetch(
            new Request(`${page.origin}${PLATFORM_PATH}${PackageMount.path(packageId)}${path}`, {
                headers: { [VERSION_HEADER]: stackPackage.version, [EDITION_HEADER]: "1" },
            }),
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
    const visitor = ProcedureRemote.at({
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
    const unshared = await refusal(visitor.list(task));

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
});

test("end a page's calls once its grant or client is revoked", async () => {
    // open the tasks board and list its tasks
    const home = await openHome();
    const { owner } = home.computer.world;
    await installTasks(home);
    const address = { space: home.address, installation: "tasks", view: "board", path: "/" };
    const page = await openPage(home, address);
    const bookmark = new Bookmark();
    const list = (opened: typeof page) =>
        refusal(
            ProcedureRemote.at({
                scope: home.spaceId,
                endpoint: {
                    url: `${opened.origin}${PACKAGE_PATH}`,
                    fetch: opened.fetch,
                    bookmark,
                },
                environment: home.computer.device.environment,
            }).list(task),
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
        scope: owner.id,
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
});

test("keep a page's session cookie across its computer's restart", async () => {
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
});

test("call a scope's home path within the start page's grant only", async () => {
    // open the start page, whose grant lists the space's installations and rollouts and gives the owner's settings
    const home = await openHome();
    const { owner } = home.computer.world;
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
    const objects = createClient(objectPackage, below(objectPackage.id), home.computer.environment);

    // list the applied rollouts and the enabled installations
    const rollouts = await stack.rollout.list({
        spaceId: home.spaceId,
        where: { appliedRevisionId: { isNotNull: true } },
    });
    const installations = await objects.installation.list({
        scope: home.spaceId,
        where: { status: "enabled" },
    });

    // push revoking the owner's client, which the start page's grant does not allow
    const client = schema.identifier("client").parse(page.signedIn.row.clientId);
    const call = accountObject.client.call("revoke", {
        ...accountObject.client.target({ scope: user.identifier(owner.id), id: client }),
    });
    const response = await page.fetch(
        new Request(`${page.origin}${HOME_PATH}/${owner.id}/store/push`, {
            method: "POST",
            headers: { [VERSION_HEADER]: accountPackage.version, [EDITION_HEADER]: "1" },
            body: JSON.stringify({
                scope: owner.id,
                mutations: [{ id: RequestId.create(home.computer.environment), calls: [call] }],
            }),
        }),
    );

    // list each enabled installation with its applied rollout, and refuse the push at the universe as outside the grant
    expect({
        installations: installations.items.map((row) => row.name).toSorted(),
        rollouts: rollouts.items.map((row) => row.installationId).toSorted(),
        pushed: [response.status, await response.json()],
    }).toEqual({
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
        rollouts: installations.items.map((row) => row.id).toSorted(),
        pushed: [
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
        ],
    });
});

test("register a browser client and bind pages to it by proof", async () => {
    // open the start page and the tasks board
    const home = await openHome();
    await installTasks(home);
    const page = await openPage(home, {
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
    const { environment } = home.computer.device;
    const person = (await home.computer.device.launcher(home.login)).user.subject.id;
    const client = Identifier.create("client", environment);
    const registered = await registerClient(page, client);
    const refused = await registerClient(board, Identifier.create("client", environment));

    // register the client's key from the start page
    const { pair } = await KeyPair.generate(environment.crypto);
    const now = () => environment.clock.now();
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
        scope: home.computer.world.owner.id,
        id: client,
        requestId: RequestId.create(environment),
    });
    const removed = await bind(url);

    expect({
        registered: [registered.status, await registered.json()],
        refused: await summary(refused),
        keyed: keyed.status,
        bound: [bound.status, await bound.json()],
        document: document.status,
        elsewhere: await summary(elsewhere),
        removed: await summary(removed),
    }).toEqual({
        registered: [201, { client }],
        refused: answered(
            "NOT_FOUND",
            "SCOPE_NOT_FOUND",
            404,
            `scope '${person}' not found`,
            "@destack/object",
        ),
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
});

test("launch and call a space another machine serves", async () => {
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

    // create and list tasks in the work space through the route the CLI's platform calls take
    const remote = ProcedureRemote.at({
        scope: work.spaceId,
        endpoint: await login.installation(work.spaceId, "tasks", tasks.id),
        environment: home.computer.device.environment,
    });
    await remote.run(task, "create", { title: "Write the report" });
    const listed = await remote.list(task);

    expect({
        host: new URL(launched.url).host,
        traded: [traded.status, header(traded, "location")],
        document: document.status,
        listed: listed.map((row) => [row["title"], row["status"]]),
    }).toEqual({
        host: `tasks--${work.address}.${DOMAINS.space}`,
        traded: [303, "/board"],
        document: 200,
        listed: [["Write the report", "open"]],
    });
});

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

test("serve a library's catalogs and assets at its dependent's origin", async () => {
    // publish a widget library with its German catalog and a font
    const { crypto } = testHost();
    const widgets = await TestViewBuild.of(
        WIDGETS,
        {},
        {
            files: new Map([
                ["locale/de.json", catalog(WIDGETS.id, "Schließen")],
                ["browser/asset/type.woff2", new TextEncoder().encode("type")],
            ]),
        },
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
    await opened.cloud.publish(await world.distribution.lock(widgets), world.owner);
    await install(opened, greeting, "greeting", [widgets]);

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

    // read the library's font below the asset path, cached for good under its name
    const font = await page.fetch(new Request(new URL("/.destack/asset/type.woff2", address).href));
    expect({
        served,
        missing,
        font: [font.status, font.headers.get("cache-control"), await font.text()],
    }).toEqual({
        served: [
            [GREETING.id, "Willkommen zurück"],
            [WIDGETS.id, "Schließen"],
        ],
        missing: [404, 404],
        font: [200, "public, max-age=31536000, immutable", "type"],
    });
});

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
        version: "2026.9.0-dev.20260912100001.0123456789ab",
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

test("serve a site at a verified hostname's route", async () => {
    // serve a space of the owner's account on the residency's cloud machine, which its hostnames reach
    const home = await openHome();
    const { computer } = home;
    const { owner } = computer.world;
    const login = (await computer.device.signedIn(home.login)).row;
    const spaceId = await computer.space(login.id, owner.account, "site");
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
    const site = await install(opened, await siteBuild(opened.computer.environment.crypto), "site");
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
                    await computer.computer.fetch(new Request(`https://${hostname}/assets/site.js`))
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
});
