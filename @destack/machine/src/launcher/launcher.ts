import type { DatabaseConnection } from "@destack/db";
import type { GrantClaim, PermissionReference } from "@destack/access";
import { Account, HOME_SPACE, RESIDENCY_SPACE } from "@destack/account/object";
import { GrantPermission, principal } from "@destack/access";
import { InstallationOrigin, SpaceAddress } from "@destack/account/address";
import { AccountError } from "@destack/account/error";
import {
    ConsentRequest,
    type Decision,
    type GrantRequest,
    RequestedPermission,
} from "@destack/account/authentication";
import { Failure } from "@destack/error";
import type { PlacementEndpoint } from "@destack/account/object";
import type { Session, UserIdentity } from "@destack/account/client";
import type { Environment } from "@destack/environment";
import accountPackage from "@destack/account/package";
import {
    type Installation,
    type Fetch,
    InstallationMount,
    ListPage,
    type ObjectType,
    PackageMount,
    SpaceMount,
} from "@destack/object";
import objectPackage from "@destack/object/package";
import {
    type ClientOptions,
    createClient,
    ObjectClient,
    ObjectStoreDescription,
} from "@destack/object/client";
import { PackageVersion, PackageId } from "@destack/package";
import {
    type ViewCategory,
    ViewDescription,
    ViewPresentationPriority,
} from "@destack/package/manifest";
import { type Identifier, nth, schema, Text } from "@destack/schema";
import { ObjectReference, RequestId, Subject } from "@destack/sync";
import { InstallationView } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import {
    type Address,
    TARGET_PARAMETER,
    TOKEN_PARAMETER,
    VIEW_KIND,
    ViewAddress,
} from "@destack/view/declare";
import { MachineError } from "../error/index.ts";

/** The origin of a client before the machine it calls, never resolving (RFC 2606). */
const UNRESOLVED_ORIGIN = "https://unresolved.invalid";

/** How a launcher's calls answer a grant request: deciding it as the person's consent page does. */
export interface LauncherConsent {
    /** Decide a grant request a call needs, absent to answer the call's consent error. */
    readonly decide?: (request: GrantRequest) => Promise<Decision>;
}

/** The machine a launcher acts from: it finds spaces and serves its own to the person. */
export interface LauncherMachine {
    /** The machine. */
    readonly id: Identifier<"machine">;
    /** The environment the person's calls run in. */
    readonly environment: Environment;
    /** Find where a space is placed and the machine serving it. */
    readonly locate: (space: Identifier<"space"> | SpaceAddress) => Promise<PlacementEndpoint>;
    /** Serve a request to a package of one of this machine's own spaces as the person. */
    readonly serve: (
        place: PlacementEndpoint,
        packageId: PackageId,
        request: Request,
    ) => Promise<Response>;
}

/** A signed-in user's launcher: it routes their calls and opens the views they address. */
export class Launcher {
    /** The machine the launcher acts from. */
    readonly machine: LauncherMachine;
    /** The user's calls to packages, with their tokens. */
    readonly session: Session<UserIdentity>;

    /** Launch for a signed-in user from a machine, calling through their session. */
    constructor(machine: LauncherMachine, session: Session<UserIdentity>) {
        this.machine = machine;
        this.session = session;
    }

    /** The signed-in user's tokens. */
    get user(): UserIdentity {
        return this.session.credential;
    }

    /** Find the user's personal account, which they signed up with. */
    async account(): Promise<Identifier<"account">> {
        // list the user's accounts of kind personal
        const { items } = await this.session.accounts().account.list({
            scope: this.user.userId,
            where: { kind: "personal" },
        });
        const [personal] = items;
        if (personal === undefined) {
            throw AccountError.of(
                "PERSONAL_ACCOUNT_NOT_FOUND",
                "personal account of the person not found",
            );
        }

        return personal.id;
    }

    /** List the addresses of the spaces of every account the user belongs to. */
    async spaces(): Promise<SpaceAddress[]> {
        // list the user's accounts, then each account's spaces
        const accounts = this.session.accounts();
        const owned = await Array.fromAsync(
            ListPage.all(accounts.account.list, { scope: this.user.userId }),
        );
        const spaces = await Promise.all(
            owned.map(async (account) => {
                const listed = await Array.fromAsync(
                    ListPage.all(accounts.space.list, { accountId: account.id }),
                );

                return listed.map((entry) =>
                    SpaceAddress.parse(
                        SpaceAddress.of({ space: entry.name, handle: account.handle }),
                    ),
                );
            }),
        );

        return spaces.flat().toSorted();
    }

    /** Read the endpoint of a package at the issuer or in a space, deciding its grant requests. */
    endpoint(
        packageId: PackageId,
        at: "universe" | PlacementEndpoint,
        consent: LauncherConsent = {},
    ): ClientOptions {
        return {
            url: UNRESOLVED_ORIGIN,
            fetch: (request) => this.send(request, packageId, at, consent),
        };
    }

    /** Read the home of the residency an account stores its data in. */
    residency(accountId: Identifier<"account">): Promise<Identifier<"space">> {
        return this.session.accounts().residency.locate({ account: accountId });
    }

    /** Connect to the home of a scope: its machine for a space, else the person's issuer. */
    store(scope: string): ClientOptions {
        // call a space's store through the stack package at the machine serving it
        const spaceId = schema.identifier("space").safeParse(scope);
        if (spaceId.success) {
            return {
                url: UNRESOLVED_ORIGIN,
                fetch: async (request) =>
                    this.send(request, stackPackage.id, await this.machine.locate(spaceId.data)),
            };
        }

        // call any other scope at the person's issuer, which stores the universe's rows
        return this.endpoint(accountPackage.id, "universe");
    }

    /** Connect to an installation's package below the machine serving its space. */
    async installation(
        space: Identifier<"space"> | SpaceAddress,
        name: string,
        packageId: PackageId,
    ): Promise<ClientOptions> {
        // send the universe's token for the installation's package bound to the space, which its machine verifies
        const { url, partition } = await this.#mount(space, name, packageId);

        return { url, fetch: this.session.fetch(packageId, { partition }) };
    }

    /** Find an installation's package below the machine serving its space, and the space. */
    async #mount(
        space: Identifier<"space"> | SpaceAddress,
        name: string,
        packageId: PackageId,
    ): Promise<{ readonly url: string; readonly partition: Identifier<"space"> }> {
        // locate the space, then the installation's mount below its machine
        const located = await this.machine.locate(space);
        const partition = located.placement.space;
        const url = `${InstallationMount.url(located.url, partition, name)}${PackageMount.path(packageId)}`;

        return { url, partition };
    }

    /** Read the address of a space the person reads: its name and its account's handle. */
    async address(spaceId: Identifier<"space">): Promise<SpaceAddress> {
        // find the space's account, then read its name and the account's handle
        const located = await this.machine.locate(spaceId);
        const { address } = await this.session.accounts().space.address({
            accountId: located.placement.scope,
            id: spaceId,
        });

        return SpaceAddress.parse(address);
    }

    /** Read the address of the home of the person's own account. */
    async home(): Promise<SpaceAddress> {
        return this.address(Account.home(await this.account()));
    }

    /** Find the view of a category in the person's home, at a path. */
    async view(category: ViewCategory, path = "/"): Promise<ViewAddress> {
        // read the views the home's installations declare, as the person
        const home = await this.home();
        const { store } = this.#clients(await this.machine.locate(home));
        const views = await Launcher.#views(store);

        // take the view of the category, the first by installation and name
        const [found] = views
            .filter(({ description }) => description.categories.includes(category))
            .toSorted(
                (left, right) =>
                    Text.compare(left.installation, right.installation) ||
                    Text.compare(left.view, right.view),
            );
        if (found === undefined) {
            throw MachineError.of("VIEW_NOT_FOUND", `'${category}' view of '${home}' not found`);
        }

        return { space: home, installation: found.installation, view: found.view, path };
    }

    /** Follow a scope's objects as the person into a database until the stack closes. */
    async follow<const Objects extends Readonly<Record<string, ObjectType>>>(
        location:
            | { readonly scope: string }
            | {
                  readonly space: Identifier<"space"> | SpaceAddress;
                  readonly installation?: string;
              },
        objects: Objects,
        database: DatabaseConnection,
        stack: AsyncDisposableStack,
    ): Promise<ObjectClient<Objects>> {
        // connect as the person to the package serving the objects
        const [first] = Object.values(objects);
        if (first === undefined) {
            throw MachineError.of(
                "OBJECTS_REQUIRED",
                "object is required: there are no objects to open",
            );
        }
        const isSpace = "space" in location && location.installation === undefined;
        const serving = isSpace ? PackageVersion.of(stackPackage) : first.package;
        const atMachine = (served: () => Promise<PlacementEndpoint>): ClientOptions => ({
            url: UNRESOLVED_ORIGIN,
            fetch: async (request) =>
                // send the request below the path of the objects' package at the machine serving the space
                this.send(request, serving.id, await served()),
        });

        // call a space's objects at the machine serving it, else a scope's at its endpoint here
        let scopeId: string;
        let reached: ClientOptions;
        if ("space" in location) {
            const located = await this.machine.locate(location.space);
            scopeId = located.placement.space;
            reached = atMachine(async () => located);
        }
        // call a scope at the person's universe, such as their account, with an access token
        else {
            scopeId = location.scope;
            reached = this.session.endpoint(serving.id);
        }

        // call the objects through the person's installation, or where the space or scope moved
        let endpoint: ClientOptions;
        let reconnect: () => ClientOptions;
        // call through the installation, again at the same endpoint
        if ("installation" in location && location.installation !== undefined) {
            const served = await this.installation(
                location.space,
                location.installation,
                serving.id,
            );
            endpoint = served;
            reconnect = () => served;
        }
        // call the space again at the machine it moved to
        else if ("space" in location) {
            endpoint = reached;
            reconnect = () => atMachine(async () => this.machine.locate(location.space));
        }
        // call the scope again at the person's universe
        else {
            endpoint = reached;
            reconnect = () => reached;
        }

        // follow them as the person until the stack closes
        return ObjectClient.start(
            {
                environment: this.machine.environment,
                database,
                objects,
                scope: scopeId,
                caller: this.user.subject,
                endpoint,
                reconnect,
                homes: (scope) => this.store(scope),
            },
            stack,
        );
    }

    /** Launch a view under the person's grant, with what its page's calls further request. */
    async launch(
        address: Address,
        further: readonly PermissionReference[] = [],
    ): Promise<{
        readonly url: string;
        readonly target: Identifier<"installation">;
        readonly space: Identifier<"space">;
    }> {
        // use the person's grant including what the view asks, growing it once they decided already
        const requested = await this.#request(address, further);
        const { view, place, installation: found, delegate, permissions, granted } = requested;
        let grant: GrantClaim;
        if (granted !== undefined && GrantPermission.includes(granted.permissions, permissions)) {
            grant = granted;
        } else if (Launcher.#isDecided(requested, this.user.subject)) {
            grant = await this.grant(delegate, permissions);
        }
        // ask the person otherwise, each permission as they read it
        else {
            throw MachineError.of(
                "GRANT_DENIED",
                `${view.space} '${view.installation}' is forbidden: it asks the person for a grant`,
                {
                    data: {
                        delegate,
                        installation: view.installation,
                        package: requested.opened.package,
                        space: view.space,
                        permissions: requested.described,
                    },
                },
            );
        }

        // exchange the person's token for the installation's package acting under the grant in the space
        const token = await this.user.token(PackageId.parse(found.packageId), {
            partition: place.placement.space,
            grants: [grant.id],
        });
        const url = new URL(
            ViewAddress.url(view, Launcher.#origin(view.space, view.installation, place.url)),
        );
        url.searchParams.set(TOKEN_PARAMETER, token);

        return { url: url.href, target: found.id, space: place.placement.space };
    }

    /** Address the page of a view, or of the view presenting an object, at its origin. */
    async url(address: Address): Promise<{
        readonly url: string;
        readonly space: Identifier<"space">;
    }> {
        // find where the space serves and the view the address opens
        const { view, place } = await this.#locate(address);
        const origin = Launcher.#origin(view.space, view.installation, place.url);

        return { url: ViewAddress.url(view, origin), space: place.placement.space };
    }

    /** Resolve the space a person types: an address, identifier, alias, or one of theirs. */
    async resolve(
        name: string,
        aliases: Readonly<Record<string, SpaceAddress>>,
    ): Promise<Identifier<"space">> {
        // take an identifier as given, and find an address's space
        const address = SpaceAddress.safeParse(name);
        const id = schema.identifier("space").safeParse(name);
        if (id.success) {
            return id.data;
        } else if (address.success) {
            return (await this.machine.locate(address.data)).placement.space;
        }

        // take the person's own name for a space
        const alias = aliases[name];
        if (alias !== undefined) {
            return (await this.machine.locate(alias)).placement.space;
        }

        // take the person's home and their residency's space
        if (name === HOME_SPACE) {
            return Account.home(await this.account());
        } else if (name === RESIDENCY_SPACE) {
            return this.residency(await this.account());
        }

        // find the person's one space of that name among every account's
        const matching = (await this.spaces()).filter(
            (each) => SpaceAddress.origin(each).space === name,
        );
        if (matching.length === 1) {
            return (await this.machine.locate(nth(matching, 0))).placement.space;
        } else if (matching.length === 0) {
            throw MachineError.of("SPACE_NOT_FOUND", `space '${name}' of yours not found`);
        } else {
            throw MachineError.of(
                "SPACE_INVALID",
                `space '${name}' is invalid: it matches several of your spaces, so use one of '${matching.join(", ")}'`,
            );
        }
    }

    /** Forward a request as the person to a universe, space or installation package. */
    async forward(request: Request, target: readonly string[]): Promise<Response> {
        // read the target's kind and the request below it
        const [kind, ...rest] = target;
        const search = new URL(request.url).search;
        const below = (path: readonly string[], origin: string) => {
            const url = `${origin}${path.map((each) => `/${each}`).join("")}${search}`;

            return new Request(url, request);
        };

        // send to a package at the person's issuer
        if (kind === "universe") {
            const packageId = PackageId.parse(rest[0]);

            return this.send(below(rest.slice(1), UNRESOLVED_ORIGIN), packageId, "universe");
        }
        // send to a package in a space, at the machine serving it
        else if (kind === "space") {
            const placed = await this.machine.locate(
                Launcher.#space(schema.string().parse(rest[0])),
            );
            const packageId = PackageId.parse(rest[1]);

            return this.send(below(rest.slice(2), UNRESOLVED_ORIGIN), packageId, placed);
        }
        // send to an installation's package at its origin, with the token bound to its space
        else if (kind === "installation") {
            const space = Launcher.#space(schema.string().parse(rest[0]));
            const name = schema.string().parse(rest[1]);
            const packageId = PackageId.parse(rest[2]);
            const { url, partition } = await this.#mount(space, name, packageId);

            return this.session.fetch(packageId, { partition })(below(rest.slice(3), url));
        }
        // refuse any other target
        else {
            throw MachineError.of(
                "TARGET_INVALID",
                `target '${String(kind)}' is invalid: use universe/<package>, space/<space>/<package> or installation/<space>/<installation>/<package>`,
            );
        }
    }

    /** Send a request to a package at the issuer or in a space, deciding its grant requests. */
    async send(
        request: Request,
        packageId: PackageId,
        at: "universe" | PlacementEndpoint,
        consent: LauncherConsent = {},
    ): Promise<Response> {
        const { decide } = consent;
        const send: Fetch = (sent) => this.#route(sent, packageId, at);

        return decide === undefined ? send(request) : this.#deciding(request, send, decide);
    }

    /** Read the person's unexpiring grant to a delegate, absent without one. */
    async granted(delegate: Subject): Promise<GrantClaim | undefined> {
        // read the person's one grant to the delegate, an expiring one asking again
        const scope = this.user.subject.id;
        const { items } = await this.session.objects().grant.list({
            scope,
            where: ObjectReference.columns("delegate", delegate),
        });
        const [found] = items;

        return found === undefined || found.expiresAt !== null
            ? undefined
            : { id: found.id, scope, permissions: found.permissions };
    }

    /** Grant a delegate some permissions, growing the person's grant to it to include them. */
    async grant(delegate: Subject, permissions: readonly GrantPermission[]): Promise<GrantClaim> {
        // reuse the person's grant when it includes them
        const found = await this.granted(delegate);
        if (found !== undefined && GrantPermission.includes(found.permissions, permissions)) {
            return found;
        }

        // grant them together with what the person granted already
        const scope = this.user.subject.id;
        const merged = GrantPermission.merge(found?.permissions ?? [], permissions);
        const created = await this.session.objects().grant.create({
            scope,
            requestId: RequestId.create(this.machine.environment),
            delegate,
            permissions: merged,
        });

        return { id: created.id, scope, permissions: merged };
    }

    /** Route a request to a package: the issuer, this machine's own space, or another machine. */
    #route(
        request: Request,
        packageId: PackageId,
        at: "universe" | PlacementEndpoint,
    ): Promise<Response> {
        const url = new URL(request.url);
        const below = `${PackageMount.path(packageId)}${url.pathname}${url.search}`;

        // send to a universe package at the person's issuer
        if (at === "universe") {
            const target = new URL(below, this.user.issuer.url).href;

            return this.session.fetch(packageId)(new Request(target, request));
        }
        // serve a space of the launcher's own machine there
        else if (at.placement.machine === this.machine.id) {
            return this.machine.serve(at, packageId, request);
        }
        // send another machine the universe's token bound to the space
        else {
            const target = new URL(`${SpaceMount.path(at.placement.space)}${below}`, at.url);
            const binding = { partition: at.placement.space };

            return this.session.fetch(packageId, binding)(new Request(target.href, request));
        }
    }

    /** Send a call, granting and sending it again once the person approves what it asks. */
    async #deciding(
        request: Request,
        send: Fetch,
        decide: NonNullable<LauncherConsent["decide"]>,
    ): Promise<Response> {
        // send the call, keeping its body to send it again
        const body = await request.arrayBuffer();
        const copy = () =>
            new Request(request.url, {
                method: request.method,
                headers: request.headers,
                ...(body.byteLength === 0 ? {} : { body }),
            });
        const response = await send(copy());

        // leave a response asking no consent, and one the person declines
        const refusal = response.ok ? undefined : await Failure.read(response.clone());
        const [asked] =
            refusal?.reason === "CONSENT_REQUIRED"
                ? ConsentRequest.parse(refusal.data).authorization_details
                : [];
        if (asked === undefined || (await decide(asked)) === "deny") {
            return response;
        }

        // grant the delegate what it asks, then send the call again
        const permissions = asked.permissions.map((each) => RequestedPermission.permission(each));
        await this.grant(asked.delegate, permissions);

        return await send(copy());
    }

    /** Call a space's stack and object packages as the person. */
    #clients(place: PlacementEndpoint) {
        const endpoint = (packageId: PackageId) => ({
            url: UNRESOLVED_ORIGIN,
            fetch: (request: Request) => this.send(request, packageId, place),
        });

        return {
            store: ObjectClient.store(endpoint(stackPackage.id), this.machine.environment),
            stack: createClient(stackPackage, endpoint(stackPackage.id), this.machine.environment),
            objects: createClient(
                objectPackage,
                endpoint(objectPackage.id),
                this.machine.environment,
            ),
        };
    }

    /** Resolve a view, or the view presenting an object, and what it asks the person to lend. */
    async #request(address: Address, further: readonly PermissionReference[]) {
        // find where the space serves and the view the address opens
        const { view, place } = await this.#locate(address);
        const spaceId = place.placement.space;
        const { stack, objects } = this.#clients(place);

        // find the installation by its name, and open its view
        const { items } = await objects.installation.list({
            scope: spaceId,
            where: { name: view.installation },
        });
        const [found] = items;
        if (found === undefined) {
            throw MachineError.of(
                "INSTALLATION_NOT_FOUND",
                `installation '${view.installation}' of '${view.space}' not found`,
            );
        }
        const opened = InstallationView.parse(
            await stack.rollout.open({
                spaceId,
                id: found.id,
                view: view.view,
                path: view.path,
            }),
        );

        // describe what they lend the installation and what its calls request, each as they read it
        const delegate = principal.installation.reference(spaceId, found.id);
        const requested = await Promise.all(
            further.map((permission) =>
                stack.rollout.request({ spaceId, id: found.id, permission }),
            ),
        );
        const described = [...opened.permissions, ...requested.flat()];
        const permissions = described.map((permission) =>
            RequestedPermission.permission(permission),
        );
        const granted = await this.granted(delegate);

        return {
            view,
            place,
            opened,
            described,
            installation: found,
            home: opened.home,
            delegate,
            permissions,
            granted,
        };
    }

    /** Find where a space serves, and the view an address opens there. */
    async #locate(address: Address): Promise<{
        readonly view: ViewAddress;
        readonly place: PlacementEndpoint;
    }> {
        // call the store of the space at the address as the person
        const place = await this.machine.locate(address.space);
        const { store } = this.#clients(place);

        // choose the view presenting an object among the views the space serves
        let view: ViewAddress;
        if ("object" in address) {
            const views = await Launcher.#views(store);
            const presented = Launcher.#present(views, address.object, address.view);
            const query = new URLSearchParams({
                [TARGET_PARAMETER]: JSON.stringify(address.object),
            });
            view = { space: address.space, ...presented, path: `/?${query.toString()}` };
        }
        // take the addressed view
        else {
            view = address;
        }

        return { view, place };
    }

    /** Report whether a person decided a view's grant already. */
    static #isDecided(
        requested: {
            readonly place: PlacementEndpoint;
            readonly installation: Installation;
            readonly home: Identifier<"space">;
            readonly permissions: readonly GrantPermission[];
        },
        person: Subject,
    ): boolean {
        // decide for an installation in the person's home or one they installed
        const spaceId = requested.place.placement.space;
        const installer = ObjectReference.at("createdBy", requested.installation);
        const isOwn = spaceId === requested.home || Subject.same(installer, person);

        // decide for a view reaching only its space, where it sees what the person sees
        const isWithinSpace = requested.permissions.every((entry) => entry.scope === spaceId);

        return isOwn || isWithinSpace;
    }

    /** Choose the view presenting an object most strongly among builds, or the view asked for. */
    static #present(
        views: readonly SpaceView[],
        object: ObjectReference,
        asked: string | undefined,
    ): { readonly installation: string; readonly view: string } {
        // rank each view of the name asked for presenting the object's type
        const presenters = views
            .filter(({ view }) => asked === undefined || view === asked)
            .flatMap(({ installation, view, description }) =>
                description.presents
                    .filter(
                        (presented) =>
                            presented.packageId === object.packageId &&
                            presented.type === object.type,
                    )
                    .map((presented) => ({ installation, view, priority: presented.priority })),
            );

        // choose the strongest, refusing none
        const [chosen] = presenters.toSorted(
            (left, right) =>
                ViewPresentationPriority.compare(left.priority, right.priority) ||
                Text.compare(left.installation, right.installation) ||
                Text.compare(left.view, right.view),
        );
        if (chosen === undefined) {
            const missing = asked === undefined ? "no view" : `no view ${asked}`;
            throw MachineError.of(
                "PRESENTER_NOT_FOUND",
                `presenter '${missing}' for ${object.type} objects of '${object.scope}' not found`,
            );
        }

        return { installation: chosen.installation, view: chosen.view };
    }

    /** Read the views a space's named installations serve, as its store describes them. */
    static async #views(store: ReturnType<typeof ObjectClient.store>): Promise<SpaceView[]> {
        const description = await ObjectStoreDescription.read(store);

        return description.packages.flatMap((served) => {
            const name = served.installation?.name;

            return name === undefined
                ? []
                : description.declarations(served, VIEW_KIND).map((declared) => ({
                      installation: name,
                      view: declared.name,
                      description: declared.description,
                  }));
        });
    }

    /** Read a target's space: its identifier, else its address. */
    static #space(segment: string): Identifier<"space"> | SpaceAddress {
        const id = schema.identifier("space").safeParse(segment);

        return id.success ? id.data : SpaceAddress.parse(segment);
    }

    /** Write an installation's origin wherever its space's machine answers. */
    static #origin(space: SpaceAddress, installation: string, endpoint: string): string {
        return InstallationOrigin.url(
            { ...SpaceAddress.origin(space), name: installation },
            endpoint,
        );
    }
}

/** A view a space's named installation serves, as its store describes it. */
interface SpaceView {
    /** The installation's name. */
    readonly installation: string;
    /** The view's name. */
    readonly view: string;
    /** The view as its declaration describes it. */
    readonly description: schema.Infer<typeof ViewDescription.declaration>;
}
