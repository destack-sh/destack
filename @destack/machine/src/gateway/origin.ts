import { Catalog } from "@destack/locale";
import * as accountObject from "@destack/account/object";
import { AccountError } from "@destack/account/error";
import { Failure, HTTP_DOMAIN } from "@destack/error";
import { OriginPath, PackageId, PackageVersion, Sandbox } from "@destack/package";
import { Digest, schema, type Identifier, present, Uuid } from "@destack/schema";
import { InstallationOrigin, SpaceAddress } from "@destack/account/address";
import { Issuer, Session, UserIdentity } from "@destack/account/client";
import type { GrantRequest } from "@destack/account/authentication";
import { TokenCache } from "@destack/access/token";
import { Launcher, type LauncherMachine } from "../launcher/index.ts";
import {
    InstallationMount,
    PackageDescription,
    PackageMount,
    SpaceMount,
    CallFrame,
    CONNECTION_PATH,
    RequestBody,
    refusal,
} from "@destack/object";
import { Build, InstallationRevision, Installation, installation } from "@destack/stack/object";
import type { ExchangeSubject, Gateway, GatewaySpace } from "./gateway.ts";
import { PageCredential } from "./credential.ts";
import { OtlpSignal } from "@destack/telemetry/otlp";
import {
    ANONYMOUS,
    Authentication,
    GrantClaim,
    PermissionReference,
    principal,
    StepUp,
    LinkSecret,
} from "@destack/access";
import { IdentityError, Jwt, type JwtClaims, Proof, PROOF_HEADER } from "@destack/identity";
import { copyRequest, Forwarded, HostAddress, WebSocketUpgrade } from "@destack/environment";
import stackPackage from "@destack/stack/package";
import { ObjectReference, Scope, Subject, RequestId } from "@destack/sync";
import accountPackage from "@destack/account/package";
import {
    type BuildReader,
    ImportMap,
    PackageManifest,
    ViewDescription,
} from "@destack/package/manifest";
import type { PackageFile } from "@destack/package/file";
import { Bookmark, createClient, type ClientOf } from "@destack/object/client";
import {
    BUILD_PARAMETER,
    INSTALLATION_PARAMETER,
    LINK_PARAMETER,
    RELOAD_EVENT,
    ScopePath,
    TARGET_PARAMETER,
    TOKEN_PARAMETER,
    ViewAddress,
    FRAME_VIEW,
    ViewContext,
    ViewPath,
    ViewState,
} from "@destack/view/declare";
import { LINK_SECRET_HEADER } from "@destack/object/server";
import { ScopePrimary } from "./primary.ts";
import { MachineError } from "../error/index.ts";
import { OriginCookie } from "./cookie.ts";
import { StaticHost, AssetIndex } from "./host.ts";

/** The cookie with a page's session on its installation's origin, as a backend for frontend. */
export const SESSION_COOKIE = "destack-session";

/** The authorization detail type of a grant an installation asks (RFC 9396 2). */
const GRANT_DETAIL = "destack_grant";

/** The server-sent event a reload stream ends with, as live reload servers send it. */
const RELOAD_FRAME = `event: ${RELOAD_EVENT}\ndata:\n\n`;

/** The comment a reload stream sends to keep its idle connection open, which readers skip. */
const KEEPALIVE_FRAME = ":\n\n";

/** How often a reload stream sends its comment: half the machine's idle timeout of 31 s. */
const KEEPALIVE_MILLISECONDS = 15_000;

/** The cookie remembering a sign-in an origin started at the issuer, until it returns. */
const SIGN_IN_COOKIE = "destack-sign-in";

/** How long a sign-in waits for the person: ten minutes, as the issuer keeps the request. */
const SIGN_IN_SECONDS = 10 * 60;

/** The scopes an origin signs a person in with: who they are and their profile. */
const ORIGIN_SCOPES = ["openid", "profile"];

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
export class InstallationOriginServer {
    /** The space whose installations' origins it serves. */
    readonly #space: GatewaySpace;
    /** The gateway routing a page's calls into the space, exchanging its person's tokens. */
    readonly #gateway: Pick<Gateway, "serve" | "record" | "exchange" | "act">;
    /** The static site of each build served so far, none for a build without, by build and view. */
    readonly #sites = new Map<string, Promise<StaticHost | undefined>>();
    /** The build of each revision served so far, which a revision never changes, by revision. */
    readonly #builds = new Map<string, Promise<Pick<Page, "image" | "reader">>>();
    /** The browser import map of each image served so far, by image and base. */
    readonly #maps = new Map<string, Promise<ImportMap>>();
    /** The builds each image's page loads, its own and its map's, by image and base. */
    readonly #loaded = new Map<string, Promise<ReadonlySet<Digest>>>();
    /** The assets of the builds each image's map points into, by mount base, indexed once. */
    readonly #assets = new Map<string, Promise<AssetIndex>>();
    /** The catalog paths each build's pages load, by build and base. */
    readonly #catalogs = new Map<string, Promise<string[]>>();
    /** The reader of each build a served page reaches, opened once, by build. */
    readonly #readers = new Map<string, Promise<BuildReader>>();

    /** Serve the origins of a space's installations through its gateway. */
    constructor(
        space: GatewaySpace,
        gateway: Pick<Gateway, "serve" | "record" | "exchange" | "act">,
    ) {
        this.#space = space;
        this.#gateway = gateway;
    }

    /** Serve an installation's origin, or its views below a route, to a person or a visitor. */
    async serve(given: Request, found: Installation, at: ServedAt): Promise<Response> {
        const mount = at.kind === "mount" ? at.mount : undefined;
        try {
            // serve the request below a route's base as at the origin's root
            const request = InstallationOriginServer.#below(given, mount);
            const url = new URL(request.url);

            // complete a sign-in the issuer returns to the open path with its code
            const code = url.searchParams.get("code");
            if (request.method === "GET" && url.pathname === OriginPath.open && code !== null) {
                return await this.#complete(request, url, code, found, mount);
            }

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
            const scoped = ScopePath.parse(url.pathname);
            if (scoped !== undefined) {
                return await this.#call(request, url, page, scoped);
            }
            // request the page's person's permission a call needs, or a fresh sign-in
            else if (url.pathname === OriginPath.request) {
                return await this.#request(url, page);
            }
            // register or bind the page's client
            else if (
                url.pathname === OriginPath.session ||
                url.pathname.startsWith(`${OriginPath.session}/`)
            ) {
                return await this.#client(request, url, page);
            }
            // stream a reload once the origin serves another build than the page's
            else if (url.pathname === OriginPath.reload) {
                return await this.#reload(request, url, page);
            }
            // append the page's telemetry
            else if (OtlpSignal.at(url.pathname) !== undefined) {
                return await this.#pageTelemetry(request, url, page);
            }
            // open another installation's view at its own origin
            else if (
                url.pathname === OriginPath.open &&
                url.searchParams.has(INSTALLATION_PARAMETER)
            ) {
                return await this.#open(url);
            }

            // send a path naming a child to the child's origin, the installation's own views first
            const child =
                at.kind === "origin"
                    ? await this.#child(request, url, at.origin, found, page)
                    : undefined;
            if (child !== undefined) {
                return child;
            }

            // serve a file of the installation's build, else the document of the view its path names
            return await this.#view(request, url, page);
        } catch (error) {
            // refuse, clearing a session cookie the space did not sign
            const refused = refusal(error, this.#space.machine.environment.telemetry);
            if (MachineError.is(error, "SESSION_UNAUTHENTICATED")) {
                const url = new URL(given.url);
                refused.headers.append("set-cookie", new OriginCookie(SESSION_COOKIE, url).clear());
            }

            return ViewState.isPageLoad(given) ? await ViewState.document(refused) : refused;
        }
    }

    /** Send a page load to an installation's view at its own origin, which signs the person in. */
    async #open(url: URL): Promise<Response> {
        // read the view the address names, refusing an incomplete one
        const address = ViewAddress.safeParseOpenPath(url);
        if (!address.success) {
            throw MachineError.of(
                "REQUEST_INVALID",
                "request is invalid: it must name a space, an installation and a view",
            );
        }

        // find the machine serving the space, then the installation's origin on it
        const { machine } = this.#space;
        const view = address.data;
        const located = await machine.directory.resolve(
            accountObject.space,
            view.space,
            machine.clock.now(),
        );
        const origin = this.#space.universe.installation(
            { ...SpaceAddress.origin(view.space), name: view.installation },
            located.url,
        );

        return new Response(null, {
            status: 303,
            headers: { location: ViewAddress.url(view, origin), "cache-control": "no-store" },
        });
    }

    /** Send a request whose first segment names a child, not a view, to the child's origin. */
    async #child(
        request: Request,
        url: URL,
        origin: InstallationOrigin,
        found: Installation,
        page: Page,
    ): Promise<Response | undefined> {
        // read the first segment, leaving the installation's views and other methods alone
        const [, segment = ""] = url.pathname.split("/");
        const views = ViewDescription.list(page.reader.manifest);
        if (request.method !== "GET" || segment === "" || Object.hasOwn(views, segment)) {
            return undefined;
        }

        // find the child the segment names
        const named = await Installation.child(this.#space.database.snapshot(), found, segment);
        if (named === undefined) {
            return undefined;
        }

        // send the rest of the path to the child at its own origin on this machine
        const child = { space: origin.space, handle: origin.handle, name: named.name };
        const located = this.#space.universe.installation(child, this.#space.machine.origin);
        const rest = url.pathname.slice(segment.length + 1) || "/";

        return new Response(null, {
            status: 303,
            headers: { location: `${located}${rest}${url.search}`, "cache-control": "no-store" },
        });
    }

    /** Read a launch's token from its address, or from a form a remote sign-in posts. */
    async #launchToken(request: Request, url: URL): Promise<string | undefined> {
        // take the token in a launch's address
        const addressed = url.searchParams.get(TOKEN_PARAMETER);
        if (addressed !== null) {
            return addressed;
        }

        // take the token a sign-in posts to the open path as a form (OpenID form_post)
        if (request.method !== "POST" || url.pathname !== OriginPath.open) {
            return undefined;
        }
        const form = await RequestBody.text(request, "a launch form", LAUNCH_FORM_BYTES);
        const posted = new URLSearchParams(form).get(TOKEN_PARAMETER);
        if (posted === null) {
            throw MachineError.of(
                "LAUNCH_TOKEN_REQUIRED",
                `'${TOKEN_PARAMETER}' is required: the launch request is missing it`,
            );
        }

        return posted;
    }

    /** Request a page's person's declared permission or a fresh sign-in, then return. */
    async #request(url: URL, page: Page): Promise<Response> {
        // read where to return, refusing another origin's address
        const back = ViewPath.safeParse(url.searchParams.get("return") ?? "/");
        if (!back.success) {
            throw MachineError.of(
                "REQUEST_INVALID",
                "request is invalid: it must return to a path of its own origin",
            );
        }

        // request a permission by its package, type and name, or a fresh sign-in at an assurance
        const query = Object.fromEntries(url.searchParams);
        const permission = PermissionReference.safeParse({
            packageId: query["package"],
            type: query["type"],
            name: query["name"],
        });
        const stepUp = StepUp.schema.safeParse({
            assurance: Number(query["assurance"]),
            ...(query["max-age"] === undefined ? {} : { maxAge: Number(query["max-age"]) }),
        });
        if (permission.success) {
            return this.#signIn(url.origin, back.data, page, { requested: [permission.data] });
        } else if (stepUp.success) {
            return this.#signIn(url.origin, back.data, page, {
                requested: [],
                stepUp: stepUp.data,
            });
        } else {
            throw MachineError.of(
                "REQUEST_INVALID",
                "request is invalid: it must ask for a permission or an assurance",
            );
        }
    }

    /** Send a page load to the issuer, signed by the space, asking for a grant or a step-up. */
    async #signIn(
        origin: string,
        path: string,
        page: Page,
        requesting: {
            /** The grant the view's launch needs, signed into the request. */
            readonly grant?: GrantRequest;
            /** The permissions the page's calls request beyond its view's. */
            readonly requested: readonly PermissionReference[];
            /** The fresh sign-in a call requests. */
            readonly stepUp?: StepUp;
        },
    ): Promise<Response> {
        // require the installation's own origin, as a route serves anonymous visitors
        if (page.mount !== ROOT) {
            throw MachineError.of(
                "INSTALLATION_ORIGIN_DENIED",
                "view is forbidden: a route serves anonymous visitors, so open the view at its installation's origin",
            );
        }

        // start an authorization as the origin's own client, signed by its space, proven by PKCE
        const { grant, requested, stepUp } = requesting;
        const { environment } = this.#space.machine;
        const state = Uuid.v4(environment.random);
        const issuer = this.#issuer();
        const identity = this.#space.identityOf(
            principal.installation.reference(this.#space.id, page.installation.id),
        );
        const details =
            grant === undefined
                ? {}
                : { authorization_details: [{ type: GRANT_DETAIL, ...grant }] };
        const authorization = await issuer.authorizeSigned(
            origin,
            `${origin}${OriginPath.open}`,
            state,
            ORIGIN_SCOPES,
            environment.clock.now(),
            (parameters) => identity.request(issuer.url, { ...parameters, ...details }),
            stepUp,
        );

        // remember the attempt in the origin's cookie, then send the browser to the issuer
        const attempt = await this.#sign(SignInAttempt, {
            state,
            verifier: authorization.verifier,
            path,
            requested: [...requested],
            exp: Math.floor(environment.clock.now() / 1000) + SIGN_IN_SECONDS,
        });
        const cookie = new OriginCookie(SIGN_IN_COOKIE, new URL(origin));

        return new Response(null, {
            status: 302,
            headers: {
                location: authorization.url,
                "cache-control": "no-store",
                "set-cookie": cookie.write(attempt, SIGN_IN_SECONDS),
            },
        });
    }

    /** Complete a sign-in the issuer returns with a code: launch the asked page as the person. */
    async #complete(
        request: Request,
        url: URL,
        code: string,
        found: Installation,
        mount: ViewMount | undefined,
    ): Promise<Response> {
        // read the attempt the origin's cookie remembers, refusing another state and the issuer's refusal
        const cookie = new OriginCookie(SIGN_IN_COOKIE, url);
        const remembered = cookie.read(request);
        const attempt =
            remembered === undefined ? undefined : await this.#read(SignInAttempt, remembered);
        const refused = url.searchParams.get("error");
        if (attempt === undefined || attempt.state !== url.searchParams.get("state")) {
            throw MachineError.of(
                "SIGN_IN_UNAUTHENTICATED",
                "authentication required: the sign-in returned without the attempt this origin started",
            );
        } else if (refused !== null) {
            throw MachineError.of(
                "SIGN_IN_UNAUTHENTICATED",
                `authentication required: the issuer refused the sign-in with '${url.searchParams.get("error_description") ?? refused}'`,
            );
        }

        // redeem the code for the person's account token, which lives within their session at the issuer
        const issuer = this.#issuer();
        const redirect = `${url.origin}${OriginPath.open}`;
        const signedIn = await issuer.redeem(url.origin, code, attempt.verifier, redirect);
        const { environment } = this.#space.machine;
        const user = new UserIdentity(
            signedIn.userId,
            issuer,
            { cache: new TokenCache(), issue: () => Promise.resolve(signedIn.tokens.accessToken) },
            environment,
        );
        const session = new Session(user, {
            universe: new URL(issuer.url).origin,
            send: (sent) => this.#space.machine.send(sent),
            bookmark: new Bookmark(),
            environment,
        });

        // launch the view the path asks for as the person, from this machine
        const page = await this.#page(request, url, found, mount);
        const launcher = new Launcher(this.#machine(user), session);
        try {
            // send the person on to trade the launch for the origin's session, forgetting the attempt
            const address = this.#address(url, found, page, attempt.path);
            const launched = await launcher.launch(address, attempt.requested);

            return new Response(null, {
                status: 303,
                headers: [
                    ["location", launched.url],
                    ["cache-control", "no-store"],
                    ["set-cookie", cookie.clear()],
                ],
            });
        } catch (error) {
            // ask the person for a grant the view needs, signed in already
            if (MachineError.is(error, "GRANT_DENIED")) {
                return await this.#signIn(url.origin, attempt.path, page, {
                    grant: error.data,
                    requested: attempt.requested,
                });
            }
            // pass on any other failure
            else {
                throw error;
            }
        }
    }

    /** Call the issuer of the machine's universe through the machine's network. */
    #issuer(): Issuer {
        const { universe, machine } = this.#space;

        return new Issuer(
            universe.issuer,
            (sent) => machine.send(sent),
            machine.environment,
            new Bookmark(),
        );
    }

    /** Launch for a person from this machine, serving its own spaces in process. */
    #machine(user: UserIdentity): LauncherMachine {
        const { machine } = this.#space;

        return {
            id: machine.id,
            environment: machine.environment,
            locate: (space) => machine.locate(space),
            serve: async (place, packageId, sent) => {
                // serve the package of this machine's own space in process, as the person
                const { space } = place.placement;
                const asked = new URL(sent.url);
                const path = `${SpaceMount.path(space)}${PackageMount.path(packageId)}`;
                const target = new URL(`${path}${asked.pathname}${asked.search}`, place.url);
                const headers = new Headers(sent.headers);
                const token = await user.token(packageId, { partition: space });
                headers.set("authorization", `Bearer ${token}`);

                return machine.serve(new Request(target.href, new Request(sent, { headers })));
            },
        };
    }

    /** Address the view a page's path asks for: its first segment's view, else the main view. */
    #address(url: URL, found: Installation, page: Page, asked: string): ViewAddress {
        // read the space's address from the origin, and the views the installation serves
        const origin = present(
            InstallationOrigin.parse(url.hostname, this.#space.machine.domain),
            `the installation origin of ${url.hostname}`,
        );
        const views = ViewDescription.list(page.reader.manifest);
        const [, segment = ""] = asked.split(/[/?]/u);
        const isNamed = Object.hasOwn(views, segment);
        const view = isNamed ? segment : ViewDescription.main(views);
        if (view === undefined) {
            throw MachineError.of("VIEW_NOT_FOUND", `view '${asked}' of '${found.name}' not found`);
        }
        const rest = isNamed ? asked.slice(segment.length + 1) : asked;

        return {
            space: SpaceAddress.of({ space: origin.space, handle: origin.handle }),
            installation: found.name,
            view,
            path: rest.startsWith("/") ? rest : `/${rest}`,
        };
    }

    /** Trade a launch's token for the origin's session cookie, dropping it from the address. */
    async #launch(
        request: Request,
        url: URL,
        found: Installation,
        token: string,
        mount: ViewMount | undefined,
    ): Promise<Response> {
        // require the installation's own origin, as a route's hostname serves other installations too
        if (mount !== undefined) {
            throw MachineError.of(
                "INSTALLATION_ORIGIN_DENIED",
                "view is forbidden: a route serves anonymous visitors, so open the view at its installation's origin",
            );
        }

        // require a person acting through the installation under one grant, within their session or client
        const { caller, credential } = await this.#verify(request, token, found);
        const [grant, ...further] = caller.grants;
        const within = PageCredential.safeParse(credential.id);
        const isWithin = credential.kind === "client" || credential.kind === "session";
        if (!isWithin || !within.success || grant === undefined || further.length > 0) {
            throw MachineError.of(
                "VIEW_GRANT_DENIED",
                "view is forbidden: a person can only open a view under their grant, within their session or client",
            );
        }

        // set the launching client in the origin's cookie and send the person on without the token
        const now = Math.floor(this.#space.machine.clock.now() / 1000);
        const session = await this.#sign(PageSession, {
            sub: schema.identifier("user").parse(caller.subject.id),
            credential: within.data,
            grant,
            installation: found.id,
            exp: now + SESSION_SECONDS,
        });
        url.searchParams.delete(TOKEN_PARAMETER);
        const path = url.pathname === OriginPath.open ? "/" : `${url.pathname}${url.search}`;

        return new Response(null, {
            status: 303,
            headers: {
                location: InstallationOriginServer.#location(path),
                "cache-control": "no-store",
                "set-cookie": new OriginCookie(SESSION_COOKIE, url).write(session, SESSION_SECONDS),
            },
        });
    }

    /** Open a build a served page reaches once, keeping its reader while the space serves. */
    #reader(digest: Digest): Promise<BuildReader> {
        return InstallationOriginServer.#memo(this.#readers, digest, () =>
            this.#space.builds.contents(digest),
        );
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
        found: Installation,
        mount: ViewMount | undefined,
    ): Promise<Page> {
        // serve a route's visitors anonymously, else the person whose session the cookie has
        const visitor =
            mount === undefined ? await this.#visitor(request, url, found) : ANONYMOUS_VISITOR;
        const linkSecrets =
            mount === undefined ? InstallationOriginServer.#linkSecrets(request, url) : [];

        // serve the image of the revision the installation follows
        const { image, reader } = await this.#image(found);

        return { installation: found, image, reader, visitor, linkSecrets, mount: mount ?? ROOT };
    }

    /** Read the image an installation serves and its root, once per revision. */
    async #image(
        found: Installation,
    ): Promise<{ readonly image: Digest; readonly reader: BuildReader }> {
        // read the revision the installation follows
        const revision = await InstallationRevision.followed(this.#space.database, found);
        if (revision === undefined) {
            throw MachineError.of(
                "REVISION_UNAVAILABLE",
                `'${found.name}' is unavailable: it has no revision to serve yet`,
            );
        }

        // read the revision's image and its root once per revision
        return await InstallationOriginServer.#memo(this.#builds, revision.id, async () => {
            // pull the revision's image and open its root
            const built = await Build.require(
                this.#space.database.snapshot(),
                this.#space.id,
                revision.buildId,
            );
            const packageId = PackageId.parse(found.packageId);
            const root = await this.#space.root(packageId, built.image);

            return { image: built.image, reader: root };
        });
    }

    /** Stream a change once the origin serves another build than the page's. */
    async #reload(request: Request, url: URL, page: Page): Promise<Response> {
        // compare the build the origin serves with the page's, at once and after each change of its installation
        const loaded = Digest.parse(url.searchParams.get(BUILD_PARAMETER));
        const { database } = this.#space;
        const { sequence } = await database.log.position();
        const target = { spaceId: this.#space.id, installation: page.installation.id };
        const isLoaded = async () => {
            const current = await Installation.requireServed(database, target);

            return (await this.#image(current)).image === loaded;
        };

        // send one change event once the served build differs, ending with the page's request
        const changes = database.log.follow(
            { tables: [installation.table], after: sequence },
            request.signal,
        );
        const encoder = new TextEncoder();
        const body = new ReadableStream<Uint8Array>({
            start: async (controller) => {
                // keep the idle stream open until it ends
                const ended = new AbortController();
                const keeping = this.#keepAlive(
                    () => controller.enqueue(encoder.encode(KEEPALIVE_FRAME)),
                    AbortSignal.any([ended.signal, request.signal]),
                );
                try {
                    // wait while the page's build is still the served one
                    if (await isLoaded()) {
                        for await (const changed of changes) {
                            if (changed.changes.length > 0 && !(await isLoaded())) {
                                break;
                            }
                        }
                    }
                    controller.enqueue(encoder.encode(RELOAD_FRAME));
                    controller.close();
                } catch (error) {
                    // end quietly once the page leaves, failing the stream otherwise
                    if (request.signal.aborted) {
                        controller.close();
                    } else {
                        controller.error(error);
                    }
                } finally {
                    ended.abort();
                    await keeping;
                }
            },
        });

        return new Response(body, {
            headers: { "content-type": "text/event-stream", "cache-control": "no-store" },
        });
    }

    /** Send a stream's keep-alive at each period on the machine's clock until a signal aborts. */
    async #keepAlive(send: () => void, signal: AbortSignal): Promise<void> {
        const { clock } = this.#space.machine;
        while (!signal.aborted) {
            // send once a full period passed, ending once the signal aborts the sleep
            const isSlept = await clock.sleep(KEEPALIVE_MILLISECONDS, signal).then(
                () => true,
                () => false,
            );
            if (isSlept) {
                send();
            }
        }
    }

    /** Read the person of a session cookie, anonymous on a page load without a valid one. */
    async #visitor(request: Request, url: URL, found: Installation): Promise<PageVisitor> {
        // serve an anonymous visitor without a cookie, as Kubernetes serves a request without credentials
        const cookie = new OriginCookie(SESSION_COOKIE, url).read(request);
        if (cookie === undefined) {
            return ANONYMOUS_VISITOR;
        }

        // read the session the space signed for this installation
        const session = await this.#read(PageSession, cookie);
        const isForeign = session === undefined || session.installation !== found.id;

        // load a page with an expired or foreign session as a visitor, who signs in again
        if (isForeign && ViewState.isPageLoad(request)) {
            return ANONYMOUS_VISITOR;
        }
        // refuse any other request with it
        else if (session === undefined || session.installation !== found.id) {
            throw MachineError.of(
                "SESSION_UNAUTHENTICATED",
                "authentication required: the page's session is invalid or expired",
            );
        }

        return {
            kind: "person",
            subject: principal.user.reference(Scope.universe.id, session.sub),
            grants: [session.grant],
            credential: session.credential,
        };
    }

    /** Verify a person's token toward an installation's package in the space. */
    async #verify(request: Request, token: string, found: Installation): Promise<Authentication> {
        // verify the token as the request's bearer, without the session cookie a reopened browser sends beside it
        const headers = new Headers(request.headers);
        headers.delete("cookie");
        headers.set("authorization", `Bearer ${token}`);
        const authentication = await this.#space.machine.authenticate(
            new Request(request.url, { headers }),
            PackageId.parse(found.packageId),
            this.#space.id,
        );

        // refuse any caller but a person
        if (authentication === undefined || !principal.user.is(authentication.caller.subject)) {
            throw MachineError.of(
                "PERSON_DENIED",
                "view is forbidden: only a person can open a view",
            );
        }

        return authentication;
    }

    /** Call a scope's packages under the page's grant: in-process here, else at its primary. */
    async #call(
        request: Request,
        url: URL,
        page: Page,
        scoped: { readonly scope: string; readonly path: string },
    ): Promise<Response> {
        // call the page's own space in-process, forwarding any other scope to its primary
        return scoped.scope === this.#space.id
            ? await this.#local(request, url, page, scoped.path)
            : await this.#primary(request, url, page);
    }

    /** Call the space's store or one of its packages in-process, as the page's person. */
    async #local(request: Request, url: URL, page: Page, path: string): Promise<Response> {
        // act as the visitor toward the package, with the links they opened the page by
        InstallationOriginServer.#requireSameOrigin(request, url);
        const headers = InstallationOriginServer.#linked(request.headers, page);
        headers.delete("cookie");
        const called = copyRequest(request, { headers }, `${url.origin}${path}${url.search}`);
        const installed = InstallationMount.of(called);
        const mounted = PackageMount.of(installed?.request ?? called);

        // connect to the space's store, vouching for the visitor anew at each call
        if (path === CONNECTION_PATH) {
            return this.#space.call(stackPackage.id, called, async () =>
                this.#authentication(page, stackPackage.id, request),
            );
        }
        // refuse an installation of another space below this space's path
        else if (installed !== undefined && installed.spaceId !== this.#space.id) {
            throw MachineError.of(
                "SCOPE_PATH_NOT_FOUND",
                `scope path not found: installation '${installed.installation}' is not in '${this.#space.id}'`,
            );
        }
        // call an installation's package below its mount, the page's own by its package alone
        else if (
            mounted !== undefined &&
            (installed !== undefined || mounted.packageId === page.installation.packageId)
        ) {
            const packageId = PackageId.parse(mounted.packageId);
            const routed = installed?.installation ?? page.installation.id;
            const mount = InstallationMount.url(request.url, this.#space.id, routed);
            const below = new URL(mounted.request.url);
            const served = copyRequest(
                called,
                {},
                `${mount}${PackageMount.path(packageId)}${below.pathname}${below.search}`,
            );

            return await this.#gateway.serve(served, async () =>
                this.#authentication(page, packageId, request),
            );
        }
        // call a package the space's store serves
        else if (mounted !== undefined) {
            const packageId = PackageId.parse(mounted.packageId);

            return this.#space.call(packageId, mounted.request, async () =>
                this.#authentication(page, packageId, request),
            );
        }
        // refuse a path naming no package
        else {
            throw MachineError.of(
                "SCOPE_PATH_NOT_FOUND",
                `scope path not found: '${path}' names no package of '${this.#space.id}'`,
            );
        }
    }

    /** Forward a page's call below a scope's path to that scope's primary, as its person. */
    async #primary(request: Request, url: URL, page: Page): Promise<Response> {
        // call a space's stack at the machine serving it, else the account package at the universe
        InstallationOriginServer.#requireSameOrigin(request, url);
        const scope = ScopePrimary.scopeOf(url);
        const spaceId = schema.identifier("space").safeParse(scope);
        const { machine } = this.#space;
        const { universe } = this.#space;
        const { directory } = this.#space.machine;
        const located = spaceId.success
            ? (await directory.locate(spaceId.data, machine.clock.now())).url
            : undefined;
        const reached =
            located === undefined
                ? {
                      package: PackageVersion.of(accountPackage),
                      url: PackageMount.url(universe.origin, accountPackage.id),
                      home: { kind: "universe" as const },
                  }
                : {
                      package: PackageVersion.of(stackPackage),
                      url: new URL(
                          `${SpaceMount.path(scope)}${PackageMount.path(stackPackage.id)}`,
                          located,
                      ).href,
                      home: { kind: "space" as const, origin: new URL(located).origin },
                  };

        // send as the visitor with their links, exchanging a person's token per package called
        const { visitor } = page;
        const primary = new ScopePrimary({ scope, ...reached }, async (sent, audience) => {
            // attach the visitor's links to calls into the serving space alone
            const isServing = spaceId.success && spaceId.data === this.#space.id;
            const headers = isServing
                ? InstallationOriginServer.#linked(sent.headers, page)
                : new Headers(sent.headers);

            // take a person's token for the request and for each call of a socket, as the universe vouches for each
            const token =
                visitor.kind === "person"
                    ? () =>
                          this.#gateway.exchange(this.#actor(page), visitor, {
                              audience,
                              ...(spaceId.success ? { scope: spaceId.data } : {}),
                          })
                    : undefined;
            if (token !== undefined) {
                headers.set("authorization", `Bearer ${await token()}`);
            }
            const forwarded = new Request(sent, { headers });

            // forward a socket, signing each of its calls anew, else send the request
            if (WebSocketUpgrade.of(forwarded) === undefined) {
                return machine.send(forwarded);
            }
            const bridge = present(machine.bridge, "the machine's socket bridge");

            return token === undefined
                ? bridge.forward(forwarded)
                : bridge.forward(forwarded, (frame) => CallFrame.authorize(frame, token));
        });

        return primary.forward(request, url);
    }

    /** Register the browser as the person's client or a key of it, or bind the page to it. */
    async #client(request: Request, url: URL, page: Page): Promise<Response> {
        // require a POST from the page's own origin
        if (request.method !== "POST") {
            return new Response(null, { status: 405, headers: { allow: "POST" } });
        }
        InstallationOriginServer.#requireSameOrigin(request, url);

        // act as the page's person, refusing a visitor
        const { visitor } = page;
        if (visitor.kind !== "person") {
            throw MachineError.of(
                "PERSON_DENIED",
                "client registration is forbidden: only a person can register a client",
            );
        }
        const accounts = this.#accounts(page, visitor);
        const userId = schema.identifier("user").parse(visitor.subject.id);
        const requestId = RequestId.create(this.#space.machine.environment);

        // bind the page's session to a client by a proof of its key, which has no body
        if (url.pathname === `${OriginPath.session}/bind`) {
            return await this.#bind(request, url, accounts, userId);
        }

        // register a client or its key
        const body = await RequestBody.json(request, "a client registration", REGISTRATION_BYTES);
        if (url.pathname === `${OriginPath.session}/key`) {
            const {
                client: parent,
                publicKey,
                proof,
            } = InstallationOriginServer.#registration(accountObject.KeyRegistration, body);
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
        else if (url.pathname !== OriginPath.session) {
            throw MachineError.of(
                "CLIENT_PATH_NOT_FOUND",
                `client path '${url.pathname}' not found`,
            );
        }

        // register the browser as a client of the person
        const { id, name, platform } = InstallationOriginServer.#registration(
            accountObject.ClientRegistration,
            body,
        );
        const created = await accounts.client.create({
            id,
            scope: userId,
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
                "PROOF_UNAUTHENTICATED",
                `authentication required: binding a client needs a '${PROOF_HEADER}' proof of its key`,
            );
        }
        const client = schema.identifier("client").parse(Proof.read(proof).subject);

        // prove the key for this request, answering a removed or unknown client as gone (RFC 9110 15.5.11)
        try {
            await accounts.client.prove({
                scope: userId,
                id: client,
                requestId: RequestId.create(this.#space.machine.environment),
                proof,
                request: { method: request.method, url: url.href },
            });
        } catch (error) {
            if (InstallationOriginServer.#isGone(error)) {
                return new Response(null, { status: GONE });
            }
            throw error;
        }

        // sign the page's session again within the bound client, keeping its grant and expiry
        const cookie = new OriginCookie(SESSION_COOKIE, url);
        const session = present(
            await this.#read(
                PageSession,
                present(cookie.read(request), "the page's session cookie"),
            ),
            "the page's session",
        );
        const bound = await this.#sign(PageSession, { ...session, credential: client });
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

        return AccountError.is(error, "CREDENTIAL_BLOCKED") || isHidden;
    }

    /** Sign a cookie's claims with the space's derived session key, as a compact JWS (RFC 7515). */
    async #sign<Claims extends schema.Schema<JwtClaims>>(
        shape: Claims,
        claims: schema.Infer<Claims>,
    ): Promise<string> {
        const { crypto } = this.#space.machine.environment;

        return Jwt.sign(shape.parse(claims), { alg: "HS256" }, await this.#sessionKey(), crypto);
    }

    /** Read a cookie's claims, absent for a cookie the space did not sign. */
    async #read<Claims extends schema.Schema>(
        shape: Claims,
        cookie: string,
    ): Promise<schema.Infer<Claims> | undefined> {
        // verify the cookie, taking any refused token as none
        const { clock, crypto } = this.#space.machine.environment;
        const options = { algorithms: ["HS256"] as const, now: clock.now() };
        try {
            const verified = await Jwt.verify(cookie, await this.#sessionKey(), options, crypto);

            return shape.parse(verified.claims);
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
            await this.#space.keystore.deriver(database, id),
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
        const { keystore } = this.#space;
        const deriver = present(await keystore.deriver(database, id), `the keys of ${id}`);

        return deriver.derive(SESSION_LABEL);
    }

    /** Take a page's telemetry export as its installation's, under the build it serves. */
    async #pageTelemetry(request: Request, url: URL, page: Page): Promise<Response> {
        // require a POST from the page's own origin, of a person or an anonymous visitor alike
        if (request.method !== "POST") {
            return new Response(null, { status: 405, headers: { allow: "POST" } });
        }
        InstallationOriginServer.#requireSameOrigin(request, url);

        // record it as the installation, under the build the page serves
        const signal = OtlpSignal.at(url.pathname);
        if (signal === undefined) {
            throw MachineError.of("SIGNAL_NOT_FOUND", "OTLP signal at this path not found");
        }
        const body = await RequestBody.json(request, `an OTLP/JSON ${signal} export`);
        await this.#gateway.record(
            { installation: page.installation.id, build: page.image },
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
        const pathname = InstallationOriginServer.#decoded(url);

        // serve a file of a dependency build the installation's import map reaches, unchanging under its digest
        const { reader, installation: found } = page;
        const policy = await this.#policy(page);
        const { base } = page.mount;
        const prefix = `${base}${OriginPath.build}/`;
        const map = await InstallationOriginServer.#memo(this.#maps, `${page.image} ${base}`, () =>
            this.#space.imports(page.image, "browser", prefix),
        );
        const builds = await InstallationOriginServer.#memo(
            this.#loaded,
            `${page.image} ${base}`,
            async () => new Set([await reader.digest(), ...ImportMap.builds(map, prefix)]),
        );
        if (pathname.startsWith(`${OriginPath.build}/`)) {
            const [digest = "", ...path] = pathname.slice(`${OriginPath.build}/`.length).split("/");
            const reached = Digest.safeParse(digest);
            if (!reached.success || !builds.has(reached.data)) {
                throw MachineError.of("VIEW_NOT_FOUND", `build '${digest}' of the views not found`);
            }
            const dependency = await this.#reader(reached.data);
            const file = await dependency.file(path.join("/"));

            return InstallationOriginServer.#file(request, file, () => dependency.load(file.path), {
                "cache-control": "public, max-age=31536000, immutable",
                "permissions-policy": policy,
            });
        }

        // serve an asset of the installation's build or of a build its import map reaches, unchanging under its name
        const assets = await InstallationOriginServer.#memo(
            this.#assets,
            `${page.image} ${base}`,
            () => AssetIndex.of(builds, (digest) => this.#reader(digest)),
        );
        const published = assets.find(pathname);
        if (published !== undefined) {
            const { reader: emitter, file: emitted } = published;

            return InstallationOriginServer.#file(
                request,
                emitted,
                () => emitter.load(emitted.path),
                {
                    "cache-control": "public, max-age=31536000, immutable",
                    "permissions-policy": policy,
                },
            );
        }

        // redirect as the static site's host configuration has it, before serving any file
        const files = StaticHost.publicFiles(reader.manifest, await reader.files());
        const site = await InstallationOriginServer.#memo(
            this.#sites,
            `${page.image} ${page.mount.view}`,
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
                    location: isPath
                        ? `${base}${InstallationOriginServer.#location(location)}`
                        : location,
                },
            });
        }

        // serve a public file of the build, a browser output's or its catalogs, under the installation's browser features
        const headers = { "cache-control": "no-cache", "permissions-policy": policy };
        const file = files.get(pathname.slice(1));
        if (file !== undefined) {
            return InstallationOriginServer.#file(
                request,
                file,
                () => reader.load(file.path),
                headers,
            );
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
                ? InstallationOriginServer.#file(request, asset, load, siteHeaders, {
                      map,
                      build: page.image,
                  })
                : InstallationOriginServer.#file(request, asset, load, siteHeaders);
        } else if (resolved?.kind === "redirect") {
            const location = InstallationOriginServer.#location(
                `${base}${resolved.path}${url.search}`,
            );

            return new Response(null, { status: 307, headers: { location } });
        }

        // serve a release dependency's catalog from its runtime build the views import
        const catalog = Catalog.at(pathname.slice(1));
        if (catalog?.dependency !== undefined) {
            const dependency = await this.#build(builds, catalog.dependency);
            const path = Catalog.path(
                { package: catalog.dependency, locale: catalog.locale },
                catalog.dependency,
            );
            const served = (await dependency.files()).find((each) => each.path === path);
            if (served !== undefined) {
                return InstallationOriginServer.#file(
                    request,
                    served,
                    () => dependency.load(served.path),
                    {
                        "cache-control": "no-cache",
                    },
                );
            }
        }

        // refuse a catalog the builds lack, as a package without one has none
        if (catalog !== undefined) {
            throw MachineError.of(
                "VIEW_NOT_FOUND",
                `catalog of the views at '${pathname}' not found`,
            );
        }

        // answer a development build's frame at its reserved path, as a view serving people
        const opened = { request, url, page, policy, map, builds };
        if (pathname === OriginPath.frame) {
            const frame = reader.manifest.outputs["browser"]?.frame;
            if (frame === undefined) {
                throw MachineError.of(
                    "VIEW_NOT_FOUND",
                    `frame of '${found.name}' not found: it runs no development build`,
                );
            }

            return this.#document(opened, { name: FRAME_VIEW, audience: "person", ...frame });
        }

        // find the view a route fixes, else the one the path's first segment names, the main view at the root
        const [, segment = ""] = pathname.split("/");
        const views = ViewDescription.list(reader.manifest);
        const name = page.mount.view ?? (segment === "" ? ViewDescription.main(views) : segment);
        const view = name === undefined ? undefined : views[name];
        const missing = site?.missing;
        const isMounted = view?.render.mode === "client";
        // answer a site's not-found page for a path no mounted view serves
        if (!isMounted && missing !== undefined) {
            const load = () => reader.load(missing.path);

            return InstallationOriginServer.#file(request, missing, load, siteHeaders, {
                map,
                build: page.image,
                status: 404,
            });
        }
        // refuse a path no mounted view serves
        else if (name === undefined || view === undefined || !isMounted) {
            throw MachineError.of(
                "VIEW_NOT_FOUND",
                `view '${name ?? "at its root"}' of '${found.name}' not found`,
            );
        }

        return this.#document(opened, { name, ...view });
    }

    /** Answer a view's or a frame's document in its context, signing a visitor in first. */
    async #document(
        opened: {
            readonly request: Request;
            readonly url: URL;
            readonly page: Page;
            readonly policy: string;
            readonly map: ImportMap;
            readonly builds: ReadonlySet<Digest>;
        },
        shown: {
            readonly name: string;
            readonly audience: ViewDescription["audience"];
            readonly entrypoint: string;
            readonly stylesheets: readonly string[];
        },
    ): Promise<Response> {
        // read the page and what it shows
        const { request, url, page, policy, map, builds } = opened;
        const { reader, installation: found } = page;
        const { base } = page.mount;
        const { name } = shown;
        const view = shown;
        // sign an anonymous visitor in first on a page load of a view serving people
        if (
            view.audience === "person" &&
            page.visitor.kind === "anonymous" &&
            ViewState.isPageLoad(request)
        ) {
            return await this.#signIn(url.origin, `${url.pathname}${url.search}`, page, {
                requested: [],
            });
        }
        const target = InstallationOriginServer.#targetOf(url) ?? page.mount.target;

        // mount it in its context, for the person in their home or for an anonymous visitor
        const { visitor } = page;
        const space = await accountObject.Space.read(
            this.#space.database.snapshot(),
            this.#space.id,
        );
        const document = ViewContext.document(
            {
                installation: found.id,
                space: this.#space.id,
                account: space.scope,
                view: name,
                build: page.image,
                base,
                universe: this.#space.universe.origin,
                catalogs: await InstallationOriginServer.#memo(
                    this.#catalogs,
                    `${page.image} ${base}`,
                    () => this.#catalogPaths(reader, builds),
                ),
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
            map,
        );
        document.headers.set("permissions-policy", policy);

        return request.method === "HEAD" ? new Response(null, document) : document;
    }

    /** List the catalogs a page loads: its build's, and each reached build's own. */
    async #catalogPaths(reader: BuildReader, builds: ReadonlySet<Digest>): Promise<string[]> {
        const owner = PackageManifest.package(reader.manifest).id;
        const listed = await Promise.all(
            [...builds].map(async (digest) => {
                // read a Destack build's catalog files, an npm unit's build having none
                const reached = await this.#reader(digest);
                if (!PackageManifest.isPackage(reached.manifest)) {
                    return [];
                }
                const packageId = reached.manifest.package.id;
                const files = (await reached.files()).filter((file) => Catalog.isPath(file.path));

                // serve the view's build's at their paths, and another build's own below its package
                return files.flatMap((file) => {
                    const at = Catalog.at(file.path);
                    if (packageId === owner) {
                        return [file.path];
                    } else if (at !== undefined && at.dependency === undefined) {
                        return [Catalog.path({ package: packageId, locale: at.locale }, owner)];
                    } else {
                        return [];
                    }
                });
            }),
        );

        return [...new Set(listed.flat())].toSorted();
    }

    /** Find a package's runtime build among the builds a page's views import, refusing others. */
    async #build(builds: ReadonlySet<Digest>, packageId: PackageId): Promise<BuildReader> {
        for (const digest of builds) {
            const reader = await this.#reader(digest);
            if (
                PackageManifest.isPackage(reader.manifest) &&
                reader.manifest.package.id === packageId
            ) {
                return reader;
            }
        }

        throw MachineError.of("VIEW_NOT_FOUND", `build of '${packageId}' in the views not found`);
    }

    /** Write the Permissions Policy of an installation's pages from its views and capabilities. */
    async #policy(page: Page): Promise<string> {
        const views = Object.values(ViewDescription.list(page.reader.manifest));
        const description = await PackageDescription.of(page.reader);

        return Sandbox.policy(
            Sandbox.merge(
                ...views.map((view) => view.sandbox),
                Installation.sandbox(page.installation, description),
            ),
        );
    }

    /** Connect to the account package as a page's person under their grant to the installation. */
    #accounts(page: Page, person: PagePerson): ClientOf<typeof accountPackage> {
        // address the account package at the universe
        const target = { audience: PackageId.parse(accountPackage.id) };
        const { machine } = this.#space;
        const { universe } = this.#space;

        return createClient(
            accountPackage,
            {
                url: PackageMount.url(universe.origin, accountPackage.id),
                fetch: async (request) => {
                    // exchange the person's token for the account package
                    const token = await this.#gateway.exchange(this.#actor(page), person, target);
                    const headers = new Headers(request.headers);
                    headers.set("authorization", `Bearer ${token}`);

                    return machine.send(new Request(request, { headers }));
                },
            },
            machine.environment,
        );
    }

    /** The installation a page acts as for its person. */
    #actor(page: Page): Subject {
        return principal.installation.reference(this.#space.id, page.installation.id);
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
            return await this.#gateway.act(this.#actor(page), visitor, {
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
                "INSTALLATION_ORIGIN_DENIED",
                "link is forbidden: a route serves anonymous visitors, so open links at the installation's origin",
            );
        }

        // refuse a malformed secret, then store it with the newest others once
        await LinkSecret.digest(secret, this.#space.machine.environment.crypto);
        const others = InstallationOriginServer.#linkSecrets(request, url).filter(
            (each) => each !== secret,
        );
        const secrets = [...others, secret].slice(-MAX_LINKS);

        // send the visitor on without it, out of the address and its referrers
        url.searchParams.delete(LINK_PARAMETER);

        return new Response(null, {
            status: 303,
            headers: {
                location: InstallationOriginServer.#location(`${url.pathname}${url.search}`),
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
                "ROUTE_BASE_BROKEN",
                `'${below.pathname}' is outside the route's path '${base}'`,
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
                        throw new URIError(`'${segment}' encodes a separator`);
                    }

                    return decoded;
                })
                .join("/");
        } catch (error) {
            throw MachineError.of(
                "PATH_INVALID",
                `path '${url.pathname}' is invalid: it does not decode`,
                {
                    cause: error,
                },
            );
        }
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
            throw MachineError.of(
                "REGISTRATION_INVALID",
                "registration is invalid: it does not match its schema",
                {
                    cause: parsed.error,
                },
            );
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
                "ORIGIN_DENIED",
                "page origin is forbidden: it can only be called from the page itself",
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
                `the '${TARGET_PARAMETER}' parameter is invalid: it is not JSON`,
                {
                    cause: error,
                },
            );
        }
        const parsed = ObjectReference.safeParse(value);
        if (!parsed.success) {
            throw MachineError.of(
                "TARGET_INVALID",
                `the '${TARGET_PARAMETER}' parameter is invalid: it is not an object`,
            );
        }

        return parsed.data;
    }
}

/** A page's request at its origin: the installation's build, its visitor and their links. */
interface Page {
    /** The installation the origin serves. */
    readonly installation: Installation;
    /** The digest of the image of the build serving the page. */
    readonly image: Digest;
    /** The build. */
    readonly reader: BuildReader;
    /** Who the page serves: the person whose client launched it, or an anonymous visitor. */
    readonly visitor: PageVisitor;
    /** The secrets of the links the visitor opened the page by. */
    readonly linkSecrets: readonly string[];
    /** Where the page's pages sit, and the view and object a route fixes. */
    readonly mount: ViewMount;
}

/** Where a request reaches an installation: at its own origin, or below a route's mount. */
export type ServedAt =
    | { readonly kind: "origin"; readonly origin: InstallationOrigin }
    | { readonly kind: "mount"; readonly mount: ViewMount };

/** Where an origin's pages sit: below a base, for a view and an object a route fixes. */
export interface ViewMount {
    /** The path the pages serve below, empty at the installation's own origin. */
    readonly base: string;
    /** The view a route serves, else the one the path's first segment names. */
    readonly view?: string;
    /** The object a route opens the view for, unless its address names one. */
    readonly target?: ObjectReference;
}

/** A person a page serves, within the session or client that launched it. */
type PagePerson = ExchangeSubject & {
    /** The kind of visitor. */
    readonly kind: "person";
    /** The person's session or client that launched the page. */
    readonly credential: PageCredential;
};

/** Who a page serves: a person, or an anonymous visitor reading what anyone may. */
type PageVisitor = PagePerson | { readonly kind: "anonymous" };

/** The anonymous visitor, whom a route and an origin without a session serve. */
const ANONYMOUS_VISITOR: PageVisitor = { kind: "anonymous" };

/** The mount of an installation's own origin, its pages at the root. */
const ROOT: ViewMount = { base: "" };

/** A sign-in an origin started at the issuer, as its cookie remembers it until it returns. */
const SignInAttempt = schema.object({
    /** The value the issuer returns, binding the return to this browser (RFC 6749 10.12). */
    state: schema.string().min(1),
    /** The PKCE verifier the authorization's challenge commits to. */
    verifier: schema.string().min(43).max(128),
    /** The path and query the person asked for. */
    path: schema.string().startsWith("/"),
    /** The permissions the page's calls request beyond what its view asks. */
    requested: schema.array(PermissionReference),
    /** When the attempt lapses, in seconds since the epoch (RFC 7519 exp). */
    exp: schema.number().int(),
});

/** A page's session as its cookie states it: the person, their grant and their credential. */
const PageSession = schema.object({
    /** The person (RFC 7519 sub). */
    sub: schema.identifier("user"),
    /** The person's session or client that launched the page, which its tokens live within. */
    credential: PageCredential,
    /** The person's grant to the installation. */
    grant: GrantClaim,
    /** The installation whose origin sets the session. */
    installation: schema.identifier("installation"),
    /** When the session expires, in seconds since the epoch (RFC 7519 exp). */
    exp: schema.number().int(),
});
/** A page's session. */
type PageSession = schema.Infer<typeof PageSession>;
