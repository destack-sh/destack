import { Catalog } from "@destack/locale";
import * as accountObject from "@destack/account/object";
import { AccountError } from "@destack/account/error";
import { Failure, HTTP_DOMAIN } from "@destack/error";
import { PackageId, Package, Sandbox } from "@destack/package";
import { Digest, schema, type Identifier, present } from "@destack/schema";
import {
    InstallationMount,
    PACKAGE_PATH,
    PackageDescription,
    PackageMount,
    SpaceMount,
    STORE_PATHS,
    RequestBody,
    refusal,
} from "@destack/object";
import {
    Build,
    InstallationRevision,
    Rollout,
    type InstallationRollout,
} from "@destack/stack/object";
import { Snapshot } from "@destack/db";
import type { ExchangeSubject } from "./gateway.ts";
import { OtlpSignal } from "@destack/telemetry/otlp";
import { ANONYMOUS, Authentication, GrantClaim, LinkSecret, principal } from "@destack/access";
import { IdentityError, Jwt, Proof, PROOF_HEADER } from "@destack/identity";
import { copyRequest, Forwarded, HostAddress } from "@destack/environment";
import stackPackage from "@destack/stack/package";
import type { BrowserClosure } from "@destack/build/store";
import { ObjectReference, Scope, Subject, RequestId } from "@destack/sync";
import accountPackage from "@destack/account/package";
import {
    type BuildReader,
    ImportMap,
    PackageManifest,
    ViewDescription,
} from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { createClient, type ClientOf } from "@destack/object/client";
import {
    CLIENT_PATH,
    HOME_PATH,
    LINK_PARAMETER,
    OPEN_PATH,
    PLATFORM_PATH,
    TARGET_PARAMETER,
    TOKEN_PARAMETER,
    ViewContext,
} from "@destack/view/declare";
import { LINK_SECRET_HEADER } from "@destack/object/server";
import { ScopePrimary } from "./primary.ts";
import type { GatewaySpace, Gateway } from "./gateway.ts";
import { MachineError } from "../error/index.ts";
import { OriginCookie } from "./cookie.ts";
import { StaticHost } from "./host.ts";

/** The path a view's dependency builds are served below, each under its digest. */
const BUILD_PATH = "/.destack/build/";

/** The cookie with a page's session on its installation's origin, as a backend for frontend. */
export const SESSION_COOKIE = "destack-session";

/** The cookie with the secrets of the links a visitor opened the origin's views by. */
export const LINK_COOKIE = "destack-link";

/** The label the space's key deriver derives the key signing its pages' sessions under. */
const SESSION_LABEL = { label: "@destack/machine/page-session", context: "" } as const;

/** The label the key telling a space's anonymous visitors apart derives under. */
const VISITOR_LABEL = { label: "@destack/machine/visitor", context: "" } as const;

/** The bytes of a visitor's HMAC its key takes, 128 bits. */
const VISITOR_KEY_BYTES = 16;

/** The largest client registration a page sends, in bytes. */
const REGISTRATION_BYTES = 16 * 1024;

/** The status a bind of a client the person removed answers, RFC 9110 15.5.11. */
const GONE = 410;

/** The most bytes of a posted launch form, which has one token: 16 KiB. */
const LAUNCH_FORM_BYTES = 16 * 1024;

/** How long a page's session lasts from its launch, in seconds: two weeks, as GitHub's sign-in. */
const SESSION_SECONDS = 14 * 24 * 60 * 60;

/** The most link secrets a visitor's cookie has: 32 secrets of 65 bytes fit its 4 KiB. */
const MAX_LINKS = 32;

/** The methods a page's files and documents answer, as static hosts answer them. */
const FILE_METHODS = "GET, HEAD";

/** The origins of a space's installations: their views' launch, pages' session and calls. */
export class ViewOrigin {
    /** The space whose installations' origins it serves. */
    readonly #space: GatewaySpace;
    /** The gateway routing a page's calls into the space, exchanging its person's tokens. */
    readonly #gateway: Pick<Gateway, "serve" | "record" | "exchange" | "act">;
    /** The static site of each build served so far, none for a build without, by build and view. */
    readonly #sites = new Map<string, Promise<StaticHost | undefined>>();
    /** The build of each revision served so far, which a revision never changes, by revision. */
    readonly #builds = new Map<string, Promise<Pick<Page, "manifest" | "reader">>>();
    /** The browser import map closure of each build served so far, by build and base. */
    readonly #closures = new Map<string, Promise<BrowserClosure>>();

    /** Serve the origins of a space's installations through its gateway. */
    constructor(
        space: GatewaySpace,
        gateway: Pick<Gateway, "serve" | "record" | "exchange" | "act">,
    ) {
        this.#space = space;
        this.#gateway = gateway;
    }

    /** Serve an installation's origin, or its views below a route, to a person or a visitor. */
    async serve(given: Request, found: InstallationRollout, mount?: ViewMount): Promise<Response> {
        try {
            // serve the request below a route's base as at the origin's root
            const request = ViewOrigin.#below(given, mount);
            const url = new URL(request.url);

            // set a launch's token as the origin's cookie, sending the person on without it
            const launch = await this.#launchToken(request, url);
            if (launch !== undefined) {
                return await this.#launch(request, url, found, launch, mount);
            }

            // store a link's secret in the origin's cookie, sending the visitor on without it
            const link = url.searchParams.get(LINK_PARAMETER);
            if (link !== null) {
                return await this.#link(request, url, link, mount);
            }

            // serve a page's own paths as its visitor, and the installation's own package
            const page = await this.#page(request, url, found, mount);
            if (url.pathname === PACKAGE_PATH || url.pathname.startsWith(`${PACKAGE_PATH}/`)) {
                return await this.#package(request, url, page);
            }
            // serve the platform's packages
            else if (url.pathname.startsWith(`${PLATFORM_PATH}/`)) {
                return await this.#platform(request, url, page);
            }
            // forward a call to its scope's primary
            else if (url.pathname.startsWith(`${HOME_PATH}/`)) {
                return await this.#primary(request, url, page);
            }
            // register or bind the page's client
            else if (url.pathname === CLIENT_PATH || url.pathname.startsWith(`${CLIENT_PATH}/`)) {
                return await this.#client(request, url, page);
            }
            // append the page's telemetry
            else if (OtlpSignal.at(url.pathname) !== undefined) {
                return await this.#pageTelemetry(request, url, page);
            }

            // serve a file of the installation's build, else the document of the view its path names
            return await this.#view(request, url, page);
        } catch (error) {
            // refuse, clearing a session cookie the space did not sign
            const refused = refusal(error, this.#space.machine.environment.telemetry);
            if (MachineError.is(error, "SESSION_INVALID")) {
                const url = new URL(given.url);
                refused.headers.append("set-cookie", new OriginCookie(SESSION_COOKIE, url).clear());
            }

            return refused;
        }
    }

    /** Read a launch's token from its address, or from a form a remote sign-in posts. */
    async #launchToken(request: Request, url: URL): Promise<string | undefined> {
        // take the token in a launch's address
        const addressed = url.searchParams.get(TOKEN_PARAMETER);
        if (addressed !== null) {
            return addressed;
        }

        // take the token a sign-in posts to the open path as a form (OpenID form_post)
        if (request.method !== "POST" || url.pathname !== OPEN_PATH) {
            return undefined;
        }
        const form = await RequestBody.text(request, "a launch form", LAUNCH_FORM_BYTES);
        const posted = new URLSearchParams(form).get(TOKEN_PARAMETER);
        if (posted === null) {
            throw MachineError.of(
                "LAUNCH_TOKEN_MISSING",
                `the launch request is missing ${TOKEN_PARAMETER}`,
            );
        }

        return posted;
    }

    /** Trade a launch's token for the origin's session cookie, dropping it from the address. */
    async #launch(
        request: Request,
        url: URL,
        found: InstallationRollout,
        token: string,
        mount: ViewMount | undefined,
    ): Promise<Response> {
        // require the installation's own origin, as a route's hostname serves other installations too
        if (mount !== undefined) {
            throw MachineError.of(
                "INSTALLATION_ORIGIN_REQUIRED",
                "a route serves anonymous visitors: open the view at its installation's origin",
            );
        }

        // require a person acting through the installation under one grant from their client
        const { caller, credential } = await this.#verify(request, token, found);
        const [grant, ...further] = caller.grants;
        if (credential.kind !== "client" || grant === undefined || further.length > 0) {
            throw MachineError.of(
                "VIEW_GRANT_REQUIRED",
                "a person can only open a view under their grant, from their client",
            );
        }

        // set the launching client in the origin's cookie and send the person on without the token
        const now = Math.floor(this.#space.machine.clock.now() / 1000);
        const session = await this.#sign({
            sub: schema.identifier("user").parse(caller.subject.id),
            client: schema.identifier("client").parse(credential.id),
            grant,
            installation: found.id,
            exp: now + SESSION_SECONDS,
        });
        url.searchParams.delete(TOKEN_PARAMETER);
        const path = url.pathname === OPEN_PATH ? "/" : `${url.pathname}${url.search}`;

        return new Response(null, {
            status: 303,
            headers: {
                location: ViewOrigin.#location(path),
                "cache-control": "no-store",
                "set-cookie": new OriginCookie(SESSION_COOKIE, url).write(session, SESSION_SECONDS),
            },
        });
    }

    /** Read a value once per key, forgetting a failed read so a later one tries again. */
    static #memo<Value>(
        known: Map<string, Promise<Value>>,
        key: string,
        read: () => Promise<Value>,
    ): Promise<Value> {
        // reuse the key's read, else start it and forget it should it fail
        let found = known.get(key);
        if (found === undefined) {
            found = read();
            known.set(key, found);
            found.catch(() => known.delete(key));
        }

        return found;
    }

    /** Read a page's request: its installation at its revision, and its visitor. */
    async #page(
        request: Request,
        url: URL,
        found: InstallationRollout,
        mount: ViewMount | undefined,
    ): Promise<Page> {
        // serve a route's visitors anonymously, else the person whose session the cookie has
        const visitor =
            mount === undefined ? await this.#visitor(request, url, found) : ANONYMOUS_VISITOR;
        const linkSecrets = mount === undefined ? ViewOrigin.#linkSecrets(request, url) : [];

        // read the revision the installation follows and its build
        const revision = await InstallationRevision.followed(this.#space.database, found);
        if (revision === undefined) {
            throw MachineError.of(
                "REVISION_MISSING",
                `${found.installation.name} has no revision to serve yet`,
            );
        }
        const { manifest, reader } = await ViewOrigin.#memo(this.#builds, revision.id, async () => {
            const built = await Build.require(
                Snapshot.live(this.#space.database),
                this.#space.id,
                revision.buildId,
            );
            const pulled = await this.#space.builds.pull(
                this.#space.machine.releases,
                PackageId.parse(found.installation.packageId),
                built.manifest,
            );

            return { manifest: built.manifest, reader: pulled };
        });

        return { rollout: found, manifest, reader, visitor, linkSecrets, mount: mount ?? ROOT };
    }

    /** Read the person of a session cookie, anonymous without one, refusing an invalid one. */
    async #visitor(request: Request, url: URL, found: InstallationRollout): Promise<PageVisitor> {
        // serve an anonymous visitor without a cookie, as Kubernetes serves a request without credentials
        const cookie = new OriginCookie(SESSION_COOKIE, url).read(request);
        if (cookie === undefined) {
            return ANONYMOUS_VISITOR;
        }

        // refuse a session the space did not sign for this installation
        const session = await this.#read(cookie);
        if (session === undefined || session.installation !== found.id) {
            throw MachineError.of("SESSION_INVALID", "the page's session is invalid or expired");
        }

        return {
            kind: "person",
            subject: principal.user.reference(Scope.universe.id, session.sub),
            grants: [session.grant],
            client: session.client,
        };
    }

    /** Verify a person's token toward an installation's package in the space. */
    async #verify(
        request: Request,
        token: string,
        found: InstallationRollout,
    ): Promise<Authentication> {
        // verify the token as the request's bearer, without the session cookie a reopened browser sends beside it
        const headers = new Headers(request.headers);
        headers.delete("cookie");
        headers.set("authorization", `Bearer ${token}`);
        const authentication = await this.#space.machine.authenticate(
            new Request(request.url, { headers }),
            PackageId.parse(found.installation.packageId),
            this.#space.id,
        );

        // refuse any caller but a person
        if (authentication === undefined || !principal.user.is(authentication.caller.subject)) {
            throw MachineError.of("PERSON_REQUIRED", "only a person can open a view");
        }

        return authentication;
    }

    /** Call the space's object API or the installation's own package, as the page's person. */
    async #package(request: Request, url: URL, page: Page): Promise<Response> {
        // answer the object API through the stack package, else the installation's
        ViewOrigin.#requireSameOrigin(request, url);
        const { rollout: found } = page;
        const below = url.pathname.slice(PACKAGE_PATH.length) || "/";
        const isObjectApi = STORE_PATHS.has(below);
        const packageId = PackageId.parse(
            isObjectApi ? stackPackage.id : found.installation.packageId,
        );

        // act as the visitor toward that package, with the links they opened the page by
        const authentication = await this.#authentication(page, packageId, request);
        const headers = ViewOrigin.#linked(request.headers, page);
        headers.delete("cookie");

        // call the store's procedures in the space
        if (isObjectApi) {
            const mounted = copyRequest(request, { headers }, `${url.origin}${below}${url.search}`);

            return this.#space.call(packageId, mounted, authentication);
        }
        // call the installation's package below its mount
        else {
            const mount = InstallationMount.url(request.url, this.#space.id, found.id);
            const mounted = copyRequest(
                request,
                { headers },
                `${mount}${PackageMount.path(packageId)}${below}${url.search}`,
            );

            return await this.#gateway.serve(mounted, authentication);
        }
    }

    /** Call a platform service a view declares, as the page's person under their grant. */
    async #platform(request: Request, url: URL, page: Page): Promise<Response> {
        // require a service mount that a view of the installation declares, called from the page's own origin
        ViewOrigin.#requireSameOrigin(request, url);
        const below = new URL(url);
        below.pathname = url.pathname.slice(PLATFORM_PATH.length);
        const routed = PackageMount.of(new Request(below.href, request));
        if (routed === undefined) {
            throw MachineError.of(
                "PLATFORM_SERVICE_MISSING",
                `no platform service at ${url.pathname}`,
            );
        }
        const packageId = PackageId.parse(routed.packageId);
        const views = Object.values(ViewDescription.list(page.reader.manifest));
        if (!views.some((view) => view.packages.includes(packageId))) {
            throw MachineError.of(
                "VIEW_PACKAGE_UNDECLARED",
                `no view of ${page.rollout.installation.name} declares the ${packageId} package`,
            );
        }

        // call it as the visitor, with the links they opened the page by
        const authentication = await this.#authentication(page, packageId, routed.request);
        const headers = ViewOrigin.#linked(routed.request.headers, page);
        headers.delete("cookie");

        return this.#space.call(
            packageId,
            new Request(routed.request, { headers }),
            authentication,
        );
    }

    /** Forward a page's call below a scope's home path to that scope's primary, as its person. */
    async #primary(request: Request, url: URL, page: Page): Promise<Response> {
        // call a space's stack at the machine serving it, else the account package at the universe
        ViewOrigin.#requireSameOrigin(request, url);
        const scope = ScopePrimary.scopeOf(url);
        const spaceId = schema.identifier("space").safeParse(scope);
        const { machine } = this.#space;
        const { directory, universe } = this.#space.host.machine;
        const located = spaceId.success
            ? (await directory.locate(spaceId.data, machine.clock.now())).url
            : undefined;
        const reached =
            located === undefined
                ? {
                      package: Package.of(accountPackage),
                      url: PackageMount.url(new URL(universe.issuer).origin, accountPackage.id),
                      home: { kind: "universe" as const },
                  }
                : {
                      package: Package.of(stackPackage),
                      url: new URL(
                          `${SpaceMount.path(scope)}${PackageMount.path(stackPackage.id)}`,
                          located,
                      ).href,
                      home: { kind: "space" as const, origin: new URL(located).origin },
                  };

        // send as the visitor with their links, exchanging a person's token per package called
        const { visitor } = page;
        const primary = new ScopePrimary(
            { scope, ...reached },
            async (sent, audience) => {
                // attach the visitor's links to calls into the serving space alone
                const isServing = spaceId.success && spaceId.data === this.#space.id;
                const headers = isServing
                    ? ViewOrigin.#linked(sent.headers, page)
                    : new Headers(sent.headers);
                if (visitor.kind === "person") {
                    const token = await this.#gateway.exchange(this.#acting(page), visitor, {
                        audience,
                        ...(spaceId.success ? { scope: spaceId.data } : {}),
                    });
                    headers.set("authorization", `Bearer ${token}`);
                }

                return machine.send(new Request(sent, { headers }));
            },
            machine.environment,
        );

        return primary.forward(request, url);
    }

    /** Register the browser as the person's client or a key of it, or bind the page to it. */
    async #client(request: Request, url: URL, page: Page): Promise<Response> {
        // require a POST from the page's own origin
        if (request.method !== "POST") {
            return new Response(null, { status: 405, headers: { allow: "POST" } });
        }
        ViewOrigin.#requireSameOrigin(request, url);

        // act as the page's person, refusing a visitor
        const { visitor } = page;
        if (visitor.kind !== "person") {
            throw MachineError.of("PERSON_REQUIRED", "only a person can register a client");
        }
        const accounts = this.#accounts(page, visitor);
        const userId = schema.identifier("user").parse(visitor.subject.id);
        const requestId = RequestId.create(this.#space.machine.environment);

        // bind the page's session to a client by a proof of its key, which has no body
        if (url.pathname === `${CLIENT_PATH}/bind`) {
            return await this.#bind(request, url, accounts, userId);
        }

        // register a client or its key
        const body = await RequestBody.json(request, "a client registration", REGISTRATION_BYTES);
        if (url.pathname === `${CLIENT_PATH}/key`) {
            const {
                client: parent,
                publicKey,
                proof,
            } = ViewOrigin.#registration(accountObject.KeyRegistration, body);
            await accounts.key.create({
                scope: userId,
                requestId,
                parent: { ...accountObject.client.typeReference, id: parent },
                publicKey,
                proof,
            });

            return Response.json({});
        }
        // refuse another path below the client's
        else if (url.pathname !== CLIENT_PATH) {
            throw MachineError.of(
                "CLIENT_PATH_NOT_FOUND",
                `${url.pathname} not found below ${CLIENT_PATH}`,
            );
        }

        // register the browser as a client of the person
        const { id, name, platform } = ViewOrigin.#registration(
            accountObject.ClientRegistration,
            body,
        );
        const created = await accounts.client.create({
            id,
            userId,
            requestId,
            name,
            kind: "browser",
            ...(platform === undefined ? {} : { platform }),
        });

        return Response.json({ client: created.id }, { status: 201 });
    }

    /** Bind the page's session to its person's client by a proof of its key (RFC 9449). */
    async #bind(
        request: Request,
        url: URL,
        accounts: ClientOf<typeof accountPackage>,
        userId: Identifier<"user">,
    ): Promise<Response> {
        // read the proof the client's key signed for this request
        const proof = request.headers.get(PROOF_HEADER);
        if (proof === null) {
            throw MachineError.of(
                "PROOF_REQUIRED",
                `binding a client needs a ${PROOF_HEADER} proof of its key`,
            );
        }
        const client = schema.identifier("client").parse(Proof.read(proof).subject);

        // prove the key for this request, answering a removed or unknown client as gone (RFC 9110 15.5.11)
        try {
            await accounts.client.prove({
                userId,
                id: client,
                requestId: RequestId.create(this.#space.machine.environment),
                proof,
                request: { method: request.method, url: url.href },
            });
        } catch (error) {
            if (ViewOrigin.#isGone(error)) {
                return new Response(null, { status: GONE });
            }
            throw error;
        }

        // sign the page's session again within the bound client, keeping its grant and expiry
        const cookie = new OriginCookie(SESSION_COOKIE, url);
        const session = present(
            await this.#read(present(cookie.read(request), "the page's session cookie")),
            "the page's session",
        );
        const bound = await this.#sign({ ...session, client });
        const remaining = session.exp - Math.floor(this.#space.machine.clock.now() / 1000);

        return Response.json(
            { client },
            {
                headers: {
                    "cache-control": "no-store",
                    "set-cookie": cookie.write(bound, remaining),
                },
            },
        );
    }

    /** Report whether a proof failed for a client the person removed or never had. */
    static #isGone(error: unknown): boolean {
        const failure = Failure.of(error);
        const isHidden = failure.domain === HTTP_DOMAIN && failure.reason === "NOT_FOUND";

        return AccountError.is(error, "CREDENTIAL_REVOKED") || isHidden;
    }

    /** Sign a page's session with the space's derived session key, as a compact JWS (RFC 7515). */
    async #sign(session: PageSession): Promise<string> {
        const { crypto } = this.#space.machine.environment;

        return Jwt.sign(
            PageSession.parse(session),
            { alg: "HS256" },
            await this.#sessionKey(),
            crypto,
        );
    }

    /** Read a page's session from its cookie, absent for one the space did not sign. */
    async #read(cookie: string): Promise<PageSession | undefined> {
        // verify the cookie, taking any refused token as no session
        const { clock, crypto } = this.#space.machine.environment;
        const options = { algorithms: ["HS256"] as const, now: clock.now() };
        try {
            const verified = await Jwt.verify(cookie, await this.#sessionKey(), options, crypto);

            return PageSession.parse(verified.claims);
        } catch (error) {
            if (IdentityError.is(error) && error.code === "UNAUTHORIZED") {
                return undefined;
            }
            throw error;
        }
    }

    /** Key an anonymous visitor by the address their first proxy wrote, none without one. */
    async #visitorKey(request: Request): Promise<string | undefined> {
        // take the client the request's first hop names
        const address = Forwarded.first(request).for;
        if (address === undefined) {
            return undefined;
        }

        // key the visitor by the HMAC of the address's network under the space's visitor key, never the address
        const { crypto } = this.#space.machine.environment;
        const { database, id } = this.#space;
        const deriver = present(
            await this.#space.host.keystore.deriver(database, id),
            `the keys of ${id}`,
        );
        const key = await crypto.importKey(
            "raw",
            await deriver.derive(VISITOR_LABEL),
            { name: "HMAC", hash: "SHA-256" },
            false,
            ["sign"],
        );
        const network = new TextEncoder().encode(HostAddress.network(address));
        const signed = await crypto.sign("HMAC", key, network);

        return new Uint8Array(signed).slice(0, VISITOR_KEY_BYTES).toHex();
    }

    /** Derive the key the space signs its pages' sessions with. */
    async #sessionKey(): Promise<Uint8Array<ArrayBuffer>> {
        // derive through the space's keys
        const { database, id } = this.#space;
        const { keystore } = this.#space.host;
        const deriver = present(await keystore.deriver(database, id), `the keys of ${id}`);

        return deriver.derive(SESSION_LABEL);
    }

    /** Take a page's telemetry export as its installation's, under the build it serves. */
    async #pageTelemetry(request: Request, url: URL, page: Page): Promise<Response> {
        // require a POST from the page's own origin
        if (request.method !== "POST") {
            return new Response(null, { status: 405, headers: { allow: "POST" } });
        }
        ViewOrigin.#requireSameOrigin(request, url);
        if (page.visitor.kind !== "person") {
            throw MachineError.of("PERSON_REQUIRED", "only a person's page can send telemetry");
        }

        // record it as the installation, under the build the page serves
        const signal = OtlpSignal.at(url.pathname);
        if (signal === undefined) {
            throw MachineError.of("SIGNAL_UNKNOWN", "no OTLP signal at this path");
        }
        const body = await RequestBody.json(request, `an OTLP/JSON ${signal} export`);
        await this.#gateway.record(
            { installation: page.rollout.id, build: page.manifest },
            signal,
            body,
        );

        return Response.json({});
    }

    /** Serve a file of the installation's build, else the document of the view its path names. */
    async #view(request: Request, url: URL, page: Page): Promise<Response> {
        // answer reads only, as static hosts answer their files and documents
        if (request.method !== "GET" && request.method !== "HEAD") {
            return new Response(null, { status: 405, headers: { allow: FILE_METHODS } });
        }
        const pathname = ViewOrigin.#decoded(url);

        // serve a file of a dependency build the installation's import map reaches, unchanging under its digest
        const { reader, rollout: found } = page;
        const policy = await this.#policy(page);
        const { base } = page.mount;
        const closure = await ViewOrigin.#memo(this.#closures, `${page.manifest} ${base}`, () =>
            this.#space.builds.browser(page.manifest, `${base}${BUILD_PATH}`),
        );
        if (pathname.startsWith(BUILD_PATH)) {
            const [digest = "", ...path] = pathname.slice(BUILD_PATH.length).split("/");
            const reached = Digest.safeParse(digest);
            if (!reached.success || !closure.builds.has(reached.data)) {
                throw MachineError.of("VIEW_NOT_FOUND", `the views reach no build ${digest}`);
            }
            const dependency = await this.#space.builds.contents(reached.data);
            const file = await dependency.file(path.join("/"));

            return ViewOrigin.#file(request, file, () => dependency.load(file.path), {
                "cache-control": "public, max-age=31536000, immutable",
                "permissions-policy": policy,
            });
        }

        // redirect as the static site's host configuration has it, before serving any file
        const files = ViewOrigin.#publicFiles(reader.manifest, await reader.files());
        const site = await ViewOrigin.#memo(
            this.#sites,
            `${page.manifest} ${page.mount.view}`,
            () =>
                StaticHost.open(reader.manifest, files, page.mount.view, (file) =>
                    reader.load(file.path),
                ),
        );
        const redirected = site?.redirect(pathname);
        if (redirected !== undefined) {
            // send a path below the mount's base, collapsing leading slashes
            const { location } = redirected;
            const isPath = location.startsWith("/") || location.startsWith("\\");

            return new Response(null, {
                status: redirected.status,
                headers: {
                    location: isPath ? `${base}${ViewOrigin.#location(location)}` : location,
                },
            });
        }

        // serve a public file of the build, a browser output's or its catalogs, under the installation's browser features
        const headers = { "cache-control": "no-cache", "permissions-policy": policy };
        const file = files.get(pathname.slice(1));
        if (file !== undefined) {
            return ViewOrigin.#file(request, file, () => reader.load(file.path), headers);
        }

        // serve a static view's page or asset with its host's headers, or send to its canonical path
        const resolved = site?.resolve(pathname);
        const siteHeaders = {
            "cache-control": headers["cache-control"],
            ...site?.headersOf(pathname),
            "permissions-policy": policy,
        };
        if (resolved?.kind === "file") {
            const { file: asset } = resolved;
            const load = () => reader.load(asset.path);

            return asset.path.endsWith(".html")
                ? ViewOrigin.#file(request, asset, load, siteHeaders, {
                      map: closure.map,
                      build: page.manifest,
                  })
                : ViewOrigin.#file(request, asset, load, siteHeaders);
        } else if (resolved?.kind === "redirect") {
            const location = ViewOrigin.#location(`${base}${resolved.path}${url.search}`);

            return new Response(null, { status: 307, headers: { location } });
        }

        // serve a release dependency's catalog from its runtime build the views reach
        const catalog = Catalog.at(pathname.slice(1));
        if (catalog?.dependency !== undefined) {
            const dependency = await this.#reached(closure, catalog.dependency);
            const path = Catalog.path(
                { package: catalog.dependency, locale: catalog.locale },
                catalog.dependency,
            );
            const served = await dependency.file(path);

            return ViewOrigin.#file(request, served, () => dependency.load(served.path), {
                "cache-control": "no-cache",
            });
        }

        // find the view a route fixes, else the one the path's first segment names
        const [, segment = ""] = pathname.split("/");
        const name = page.mount.view ?? segment;
        const view = ViewDescription.list(reader.manifest)[name];
        const missing = site?.missing;
        const isMounted = view?.render.mode === "client";
        // answer a site's not-found page for a path no mounted view serves
        if (!isMounted && missing !== undefined) {
            const load = () => reader.load(missing.path);

            return ViewOrigin.#file(request, missing, load, siteHeaders, {
                map: closure.map,
                build: page.manifest,
                status: 404,
            });
        }
        // refuse a path no mounted view serves
        else if (view === undefined || !isMounted) {
            throw MachineError.of(
                "VIEW_NOT_FOUND",
                `${found.installation.name} mounts no view ${name}`,
            );
        }
        const target = ViewOrigin.#targetOf(url) ?? page.mount.target;

        // mount it in its context, for the person in their home or for an anonymous visitor
        const { visitor } = page;
        const space = present(
            await accountObject.Space.read(Snapshot.live(this.#space.database), this.#space.id),
            `the row of ${this.#space.id}`,
        );
        const document = ViewContext.document(
            {
                installation: found.id,
                space: this.#space.id,
                account: space.scope,
                view: name,
                base,
                ...(visitor.kind === "person"
                    ? {
                          user: visitor.subject,
                          home: accountObject.Account.home(
                              accountObject.Account.personal(
                                  schema.identifier("user").parse(visitor.subject.id),
                              ),
                          ),
                      }
                    : { user: ANONYMOUS }),
                ...(target === undefined ? {} : { target }),
            },
            view.entrypoint,
            view.stylesheets,
            closure.map,
        );
        document.headers.set("permissions-policy", policy);

        return request.method === "HEAD" ? new Response(null, document) : document;
    }

    /** Find a package's runtime build among those a page's views reach, refusing others. */
    async #reached(closure: BrowserClosure, packageId: PackageId): Promise<BuildReader> {
        for (const digest of closure.builds) {
            const reader = await this.#space.builds.contents(digest);
            if (PackageManifest.package(reader.manifest).id === packageId) {
                return reader;
            }
        }

        throw MachineError.of("VIEW_NOT_FOUND", `the views reach no build of ${packageId}`);
    }

    /** Write the Permissions Policy of an installation's pages from its views and capabilities. */
    async #policy(page: Page): Promise<string> {
        const views = Object.values(ViewDescription.list(page.reader.manifest));
        const description = await PackageDescription.of(page.reader);

        return Sandbox.policy(
            Sandbox.merge(
                ...views.map((view) => view.sandbox),
                Rollout.sandbox(page.rollout.installation, description),
            ),
        );
    }

    /** Connect to the account package as a page's person under their grant to the installation. */
    #accounts(page: Page, person: PagePerson): ClientOf<typeof accountPackage> {
        // address the account package at the universe
        const target = { audience: PackageId.parse(accountPackage.id) };
        const { machine } = this.#space;
        const { universe } = this.#space.host.machine;

        return createClient(
            accountPackage,
            {
                url: PackageMount.url(new URL(universe.issuer).origin, accountPackage.id),
                fetch: async (request) => {
                    // exchange the person's token for the account package
                    const token = await this.#gateway.exchange(this.#acting(page), person, target);
                    const headers = new Headers(request.headers);
                    headers.set("authorization", `Bearer ${token}`);

                    return machine.send(new Request(request, { headers }));
                },
            },
            machine.environment,
        );
    }

    /** The installation a page acts as for its person. */
    #acting(page: Page): Subject {
        return principal.installation.reference(this.#space.id, page.rollout.id);
    }

    /** Authenticate a page's call to a package in the space, an anonymous visitor by address. */
    async #authentication(
        page: Page,
        audience: PackageId,
        request: Request,
    ): Promise<Authentication | undefined> {
        // act as a person through the installation, as the universe vouches
        const { visitor } = page;
        if (visitor.kind === "person") {
            return await this.#gateway.act(this.#acting(page), visitor, {
                audience,
                scope: this.#space.id,
            });
        }

        // tell an anonymous visitor apart by their address, so limits count each one
        const key = await this.#visitorKey(request);
        const now = this.#space.machine.environment.clock.now();

        return key === undefined
            ? undefined
            : Authentication.visitor(key, audience, now, SESSION_SECONDS * 1000);
    }

    /** Copy a page's call's headers with the secrets of the links its visitor opened it by. */
    static #linked(given: Headers, page: Page): Headers {
        const headers = new Headers(given);
        if (page.linkSecrets.length > 0) {
            headers.set(LINK_SECRET_HEADER, page.linkSecrets.join(","));
        }

        return headers;
    }

    /** Store a link's secret in the origin's cookie, dropping it from the address. */
    async #link(
        request: Request,
        url: URL,
        secret: string,
        mount: ViewMount | undefined,
    ): Promise<Response> {
        // require the installation's own origin, as a route's hostname serves other installations too
        if (mount !== undefined) {
            throw MachineError.of(
                "INSTALLATION_ORIGIN_REQUIRED",
                "a route serves anonymous visitors: open links at the installation's origin",
            );
        }

        // refuse a malformed secret, then store it with the newest others once
        await LinkSecret.digest(secret, this.#space.machine.environment.crypto);
        const others = ViewOrigin.#linkSecrets(request, url).filter((each) => each !== secret);
        const secrets = [...others, secret].slice(-MAX_LINKS);

        // send the visitor on without it, out of the address and its referrers
        url.searchParams.delete(LINK_PARAMETER);

        return new Response(null, {
            status: 303,
            headers: {
                location: ViewOrigin.#location(`${url.pathname}${url.search}`),
                "cache-control": "no-store",
                "set-cookie": new OriginCookie(LINK_COOKIE, url).write(secrets.join(".")),
            },
        });
    }

    /** Read the secrets of the links a visitor opened the origin's views by. */
    static #linkSecrets(request: Request, url: URL): string[] {
        const cookie = new OriginCookie(LINK_COOKIE, url).read(request);

        return cookie === undefined ? [] : cookie.split(".");
    }

    /** Write a redirect's location on the same origin, collapsing leading slashes. */
    static #location(path: string): string {
        return `/${path.replace(/^[/\\]+/u, "")}`;
    }

    /** Copy a request below a route's base as at the origin's root, as it is without a route. */
    static #below(given: Request, mount: ViewMount | undefined): Request {
        // take the request as it is at the installation's own origin
        if (mount === undefined || mount.base === "") {
            return given;
        }

        // require the base as the path's first segments, as the machine matched them
        const below = new URL(given.url);
        const { base } = mount;
        if (below.pathname !== base && !below.pathname.startsWith(`${base}/`)) {
            throw MachineError.of(
                "ROUTE_BASE_MISMATCH",
                `${below.pathname} is outside the route's path ${base}`,
            );
        }
        below.pathname = below.pathname.slice(base.length) || "/";

        return copyRequest(given, {}, below.href);
    }

    /** Read an address's path decoded, refusing an encoded slash and a malformed escape. */
    static #decoded(url: URL): string {
        try {
            return url.pathname
                .split("/")
                .map((segment) => {
                    const decoded = decodeURIComponent(segment);
                    if (decoded.includes("/") || decoded.includes("\\")) {
                        throw new URIError(`${segment} encodes a separator`);
                    }

                    return decoded;
                })
                .join("/");
        } catch (error) {
            throw MachineError.of("PATH_INVALID", `invalid path ${url.pathname}`, {
                cause: error,
            });
        }
    }

    /** Index a build's public files by path: its browser outputs' and its own catalogs. */
    static #publicFiles(
        manifest: PackageManifest,
        files: readonly PackageFile[],
    ): ReadonlyMap<string, PackageFile> {
        // take the files below a browser output's directory, and the build's own catalogs
        const directories = Object.values(manifest.outputs)
            .filter((output) => output.runtime === "browser")
            .map((output) => `${output.directory}/`);
        const shown = files.filter((file) => {
            const catalog = Catalog.at(file.path);
            const isOwnCatalog = catalog !== undefined && catalog.dependency === undefined;

            return isOwnCatalog || directories.some((directory) => file.path.startsWith(directory));
        });

        return new Map(shown.map((file) => [file.path, file]));
    }

    /** Answer a file under its ETag: unchanged with 304, else its bytes or a page with its map. */
    static async #file(
        request: Request,
        file: PackageFile,
        load: () => Promise<Uint8Array<ArrayBuffer>>,
        headers: Readonly<Record<string, string>>,
        page?: { readonly map: ImportMap; readonly build: Digest; readonly status?: number },
    ): Promise<Response> {
        // answer an unchanged file as such, a page's map following from its build
        const status = page?.status ?? 200;
        const etag = page === undefined ? `"${file.digest}"` : `"${file.digest}.${page.build}"`;
        const described = {
            ...headers,
            "content-type": file.mediaType,
            "x-content-type-options": "nosniff",
            etag,
        };
        if (status === 200 && request.headers.get("if-none-match") === etag) {
            return new Response(null, { status: 304, headers: described });
        }

        // answer its headers alone for HEAD
        if (request.method === "HEAD") {
            return new Response(null, { status, headers: described });
        }
        // answer a page with the map at the start of its head
        else if (page !== undefined) {
            const body = ImportMap.insert(new TextDecoder().decode(await load()), page.map);

            return new Response(body, { status, headers: described });
        }
        // answer a file's bytes
        else {
            return new Response(await load(), {
                status,
                headers: { ...described, "content-length": String(file.size) },
            });
        }
    }

    /** Read a page's registration body, refusing one its schema rejects as a bad request. */
    static #registration<Shape extends schema.Schema>(
        shape: Shape,
        body: unknown,
    ): schema.Infer<Shape> {
        const parsed = shape.safeParse(body);
        if (!parsed.success) {
            throw MachineError.of("REGISTRATION_INVALID", "invalid registration", {
                cause: parsed.error,
            });
        }

        return parsed.data;
    }

    /** Refuse a page's call sent from any origin but its own, by Origin or Fetch Metadata. */
    static #requireSameOrigin(request: Request, url: URL): void {
        const origin = request.headers.get("origin");
        const isSame =
            origin === null
                ? request.headers.get("sec-fetch-site") === "same-origin"
                : origin === url.origin;
        if (!isSame) {
            throw MachineError.of(
                "ORIGIN_MISMATCH",
                "a page's origin can only be called from the page itself",
            );
        }
    }

    /** Read the object a view's address opens it for, absent for none, refusing an invalid one. */
    static #targetOf(url: URL): ObjectReference | undefined {
        // take an absent parameter as no object
        const json = url.searchParams.get(TARGET_PARAMETER);
        if (json === null) {
            return undefined;
        }

        // parse the reference, refusing anything else
        let value: unknown;
        try {
            value = JSON.parse(json);
        } catch (error) {
            throw MachineError.of(
                "TARGET_INVALID",
                `the ${TARGET_PARAMETER} parameter is not JSON`,
                {
                    cause: error,
                },
            );
        }
        const parsed = ObjectReference.safeParse(value);
        if (!parsed.success) {
            throw MachineError.of(
                "TARGET_INVALID",
                `the ${TARGET_PARAMETER} parameter is not an object`,
            );
        }

        return parsed.data;
    }
}

/** A page's request at its origin: the installation's build, its visitor and their links. */
interface Page {
    /** The rollout of the installation the origin serves. */
    readonly rollout: InstallationRollout;
    /** The digest of the manifest of the build its revision follows. */
    readonly manifest: Digest;
    /** The build. */
    readonly reader: BuildReader;
    /** Who the page serves: the person whose client launched it, or an anonymous visitor. */
    readonly visitor: PageVisitor;
    /** The secrets of the links the visitor opened the page by. */
    readonly linkSecrets: readonly string[];
    /** Where the page's pages sit, and the view and object a route fixes. */
    readonly mount: ViewMount;
}

/** Where an origin's pages sit: below a base, for a view and an object a route fixes. */
export interface ViewMount {
    /** The path the pages serve below, empty at the installation's own origin. */
    readonly base: string;
    /** The view a route serves, else the one the path's first segment names. */
    readonly view?: string;
    /** The object a route opens the view for, unless its address names one. */
    readonly target?: ObjectReference;
}

/** A person a page serves, from the client that launched it. */
type PagePerson = ExchangeSubject & {
    /** The kind of visitor. */
    readonly kind: "person";
    /** The person's client that launched the page. */
    readonly client: Identifier<"client">;
};

/** Who a page serves: a person, or an anonymous visitor reading what anyone may. */
type PageVisitor = PagePerson | { readonly kind: "anonymous" };

/** The anonymous visitor, whom a route and an origin without a session serve. */
const ANONYMOUS_VISITOR: PageVisitor = { kind: "anonymous" };

/** The mount of an installation's own origin, its pages at the root. */
const ROOT: ViewMount = { base: "" };

/** A page's session as its origin's cookie states it: the person, their grant and their client. */
const PageSession = schema.object({
    /** The person (RFC 7519 sub). */
    sub: schema.identifier("user"),
    /** The person's client that launched the page, which its tokens live within. */
    client: schema.identifier("client"),
    /** The person's grant to the installation. */
    grant: GrantClaim,
    /** The installation whose origin sets the session. */
    installation: schema.identifier("installation"),
    /** When the session expires, in seconds since the epoch (RFC 7519 exp). */
    exp: schema.number().int(),
});
/** A page's session. */
type PageSession = schema.Infer<typeof PageSession>;
