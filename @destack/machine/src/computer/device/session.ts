import type { GrantClaim } from "@destack/access";
import { GrantPermission, PermissionSet, principal } from "@destack/access";
import { DOMAINS, InstallationOrigin, SpaceAddress } from "@destack/account/address";
import type { PlacementEndpoint } from "@destack/account/object";
import type { Login } from "@destack/account/native";
import accountPackage from "@destack/account/package";
import {
    type Installation,
    InstallationMount,
    type ObjectType,
    PACKAGE_PATH,
    PackageMount,
    SpaceMount,
} from "@destack/object";
import objectPackage from "@destack/object/package";
import { type ClientOptions, createClient } from "@destack/object/client";
import { Package, PackageId } from "@destack/package";
import {
    type ViewCategory,
    ViewDescription,
    ViewPresentationPriority,
} from "@destack/package/manifest";
import { type Identifier, schema, Text } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { Declaration } from "@destack/stack/client";
import { InstallationView, type InstallationRollout } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { type Address, TARGET_PARAMETER, View, ViewAddress } from "@destack/view/declare";
import { MachineError } from "../../error/index.ts";
import type { Device, ScopeReplica } from "./device.ts";
import { Visit } from "./object/index.ts";
import { HostAddress } from "@destack/environment";

/** The domain of the installation origins a computer's machine serves on loopback. */
export const LOOPBACK_DOMAIN = "localhost";

/** The origin of a client before the machine it calls, never resolving (RFC 2606). */
const UNRESOLVED_ORIGIN = "https://unresolved.invalid";

/** A person signed in on this computer's device: their calls to spaces and the views they open. */
export class Session {
    /** The device the person is signed in on. */
    readonly device: Device;
    /** The person's sign-in, with their session at the universe. */
    readonly login: Login;

    /** Act as a person signed in on a device. */
    constructor(device: Device, login: Login) {
        this.device = device;
        this.login = login;
    }

    /** Read the endpoint of a package: at the person's issuer, or in a space. */
    endpoint(packageId: PackageId, at: "universe" | PlacementEndpoint): ClientOptions {
        return {
            url: UNRESOLVED_ORIGIN,
            fetch: (request) => this.send(request, packageId, at),
        };
    }

    /** Connect to a platform package of a residency's home by its installation's name. */
    async platform(
        packageId: PackageId,
        name: string,
        home: Identifier<"space">,
    ): Promise<ClientOptions> {
        // find the machine serving the residency's home
        const located = await this.device.locate(home);

        // send the universe's token for the package bound to the home, which its machine verifies
        return {
            url: `${InstallationMount.url(located.url, home, name)}${PackageMount.path(packageId)}`,
            fetch: (request) => this.login.send(request, packageId, { partition: home }),
        };
    }

    /** Read the home of the residency an account stores its data in. */
    residency(accountId: Identifier<"account">): Promise<Identifier<"space">> {
        return this.login.accounts().residency.locate({ account: accountId });
    }

    /** Connect to the home of a scope: its machine for a space, else the person's issuer. */
    store(scope: string): ClientOptions {
        // call a space's store through the stack package at the machine serving it
        const spaceId = schema.identifier("space").safeParse(scope);
        if (spaceId.success) {
            return {
                url: UNRESOLVED_ORIGIN,
                fetch: async (request) =>
                    this.send(request, stackPackage.id, await this.device.locate(spaceId.data)),
            };
        }

        // call any other scope at the person's issuer, which stores the universe's rows
        return this.endpoint(accountPackage.id, "universe");
    }

    /** Connect to an installation's package at its origin. */
    async installation(
        address: SpaceAddress,
        name: string,
        packageId: PackageId,
    ): Promise<ClientOptions> {
        // find the installation's origin below its space's machine
        const place = await this.device.locate(address);
        const origin = Session.#origin(address, name, place.url);

        // call from the origin with the universe's token for the installation's package bound to the space
        return {
            url: `${origin}${PACKAGE_PATH}`,
            fetch: (request) =>
                this.login.send(request, packageId, { partition: place.placement.id }),
        };
    }

    /** Read the address of a space the person reads: its name and its account's handle. */
    async address(spaceId: Identifier<"space">): Promise<SpaceAddress> {
        // find the space's account, then read its name and the account's handle
        const located = await this.device.locate(spaceId);
        const { address } = await this.login.accounts().space.address({
            accountId: located.placement.scope,
            id: spaceId,
        });

        return SpaceAddress.parse(address);
    }

    /** Read the address of the home of the person's own account. */
    async home(): Promise<SpaceAddress> {
        return this.address(await this.login.home());
    }

    /** Find the view of a category in the person's home, at a path. */
    async view(category: ViewCategory, path = "/"): Promise<ViewAddress> {
        // read the views the home's installations declare, as the person
        const home = await this.home();
        const place = await this.device.locate(home);
        const { stack, objects } = this.#clients(place);
        const views = await Declaration.read(stack, objects, place.placement.id, [View.kind]);

        // take the view of the category, the first by installation and name
        const [found] = views
            .filter(({ declaration }) =>
                ViewDescription.declaration
                    .parse(declaration.description)
                    .categories.includes(category),
            )
            .toSorted(
                (left, right) =>
                    Text.compare(left.rollout.installation.name, right.rollout.installation.name) ||
                    Text.compare(left.declaration.name, right.declaration.name),
            );
        if (found === undefined) {
            throw MachineError.of("VIEW_NOT_FOUND", `${home} has no ${category} view`);
        }

        return {
            space: home,
            installation: found.rollout.installation.name,
            view: found.declaration.name,
            path,
        };
    }

    /** Open a scope's objects, following its moves through the directory. */
    async open<const Objects extends Readonly<Record<string, ObjectType>>>(
        location:
            | { readonly endpoint: string; readonly scope: string }
            | { readonly space: SpaceAddress; readonly installation?: string },
        objects: Objects,
    ): Promise<ScopeReplica<Objects>> {
        // connect as the person to the package serving the objects
        const [first] = Object.values(objects);
        if (first === undefined) {
            throw MachineError.of("OBJECTS_MISSING", "no objects to open");
        }
        const isSpace = "space" in location && location.installation === undefined;
        const serving = isSpace ? Package.of(stackPackage) : first.package;
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
            const located = await this.device.locate(location.space);
            scopeId = located.placement.id;
            reached = atMachine(async () => located);
        }
        // call another scope at its endpoint, such as the person's issuer
        else {
            scopeId = location.scope;
            reached = {
                url: UNRESOLVED_ORIGIN,
                fetch: (request) =>
                    this.#sendOwn(request, PackageMount.path(serving.id), location.endpoint),
            };
        }

        // follow the objects as the person through their installation
        const homes = (scope: string) => this.store(scope);
        const caller = this.login.subject;
        if ("installation" in location && location.installation !== undefined) {
            const served = await this.installation(
                location.space,
                location.installation,
                serving.id,
            );

            return this.device.replicate(scopeId, objects, caller, served, () => served, homes);
        }

        // follow the objects as the person, a space's again at the machine it moved to
        const reconnect =
            "space" in location
                ? () => atMachine(async () => this.device.locate(location.space))
                : () => reached;

        return this.device.replicate(scopeId, objects, caller, reached, reconnect, homes);
    }

    /** Grant a view's installation what the view asks the person to grant. */
    async grant(address: Address): Promise<GrantClaim> {
        const { delegate, permissions } = await this.#request(address);

        return this.login.grant(delegate, permissions);
    }

    /** Launch a view, or one presenting an object, under the person's grant to its installation. */
    async launch(
        address: Address,
    ): Promise<{ readonly url: string; readonly target: Identifier<"installation"> }> {
        // use the person's grant including what the view asks, growing it once they decided already
        const requested = await this.#request(address);
        const { view, place, installation: found, delegate, permissions, granted } = requested;
        let grant: GrantClaim;
        if (granted !== undefined && GrantPermission.includes(granted.permissions, permissions)) {
            grant = { id: granted.id, scope: granted.scope };
        } else if (Session.#isDecided(requested, this.login.subject)) {
            grant = await this.login.grant(delegate, permissions);
        }
        // ask the person otherwise
        else {
            throw MachineError.of(
                "GRANT_REQUIRED",
                `${view.space} ${view.installation} asks the person for a grant`,
                { data: { permissions: [...permissions] } },
            );
        }

        // record the person's visit of an opened object
        if ("object" in address) {
            const { id } = place.placement;
            await Visit.record(this.device.visits, this.login.subject, id, address.object);
        }

        // exchange the person's token for the installation's package acting under the grant in the space
        const token = await this.login.token(PackageId.parse(found.packageId), {
            partition: place.placement.id,
            grants: [grant.id],
        });
        const origin = Session.#origin(view.space, view.installation, place.url);

        return { url: ViewAddress.url(view, origin, token), target: found.id };
    }

    /** Sign a proof of the client the person registered on this computer. */
    prove(): Promise<string> {
        return this.device.keys.prove(schema.identifier("client").parse(this.login.row.clientId));
    }

    /** Send a request to a package: at the person's issuer, or in a space. */
    async send(
        request: Request,
        packageId: PackageId,
        at: "universe" | PlacementEndpoint,
    ): Promise<Response> {
        const named = new URL(request.url);
        const below = `${PackageMount.path(packageId)}${named.pathname}${named.search}`;

        // send to a universe package at the person's issuer
        if (at === "universe") {
            return this.login.send(
                new Request(new URL(below, this.login.issuer.url).href, request),
                packageId,
            );
        }
        // prove the person's client to this computer's machine below the space's mount
        else if (at.placement.machine === (await this.device.loopback.computer()).id) {
            const path = `${SpaceMount.path(at.placement.id)}${PackageMount.path(packageId)}`;

            return this.#sendOwn(request, path, at.url);
        }
        // send the universe's token bound to the space to another machine
        else {
            const target = new URL(`${SpaceMount.path(at.placement.id)}${below}`, at.url);

            return this.login.send(new Request(target.href, request), packageId, {
                partition: at.placement.id,
            });
        }
    }

    /** Send a request below a path at this computer's machine, proving the person's client. */
    async #sendOwn(request: Request, path: string, origin: string): Promise<Response> {
        // address the target below the path with a fresh proof of the person's client
        const named = new URL(request.url);
        const target = new URL(`${path}${named.pathname}${named.search}`, origin);
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${await this.prove()}`);

        return this.device.environment.network.fetch(
            new Request(target.href, new Request(request, { headers })),
        );
    }

    /** Call a space's stack and object packages as the person. */
    #clients(place: PlacementEndpoint) {
        const endpoint = (packageId: PackageId) => ({
            url: UNRESOLVED_ORIGIN,
            fetch: (request: Request) => this.send(request, packageId, place),
        });

        return {
            stack: createClient(stackPackage, endpoint(stackPackage.id), this.device.environment),
            objects: createClient(
                objectPackage,
                endpoint(objectPackage.id),
                this.device.environment,
            ),
        };
    }

    /** Resolve a view, or the view presenting an object, and what it asks the person to lend. */
    async #request(address: Address) {
        // call the stack of the space at the address as the person
        const place = await this.device.locate(address.space);
        const spaceId = place.placement.id;
        const { stack, objects } = this.#clients(place);

        // choose the view presenting an object among the views the space's installations apply
        let view: ViewAddress;
        let found: Installation | undefined;
        if ("object" in address) {
            const views = await Declaration.read(stack, objects, spaceId, [View.kind]);
            const presented = Session.#present(views, address.object, address.view);
            const query = new URLSearchParams({
                [TARGET_PARAMETER]: JSON.stringify(address.object),
            });
            const installation = presented.rollout.installation.name;
            view = {
                space: address.space,
                installation,
                view: presented.view,
                path: `/?${query.toString()}`,
            };
            found = presented.rollout.installation;
        }
        // find the addressed installation by its name
        else {
            view = address;
            const { items } = await objects.installation.list({
                scope: spaceId,
                where: { name: view.installation },
            });
            [found] = items;
        }

        // open its view
        if (found === undefined) {
            throw MachineError.of(
                "INSTALLATION_NOT_FOUND",
                `${view.space} has no installation ${view.installation}`,
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

        // find the person's grant of what they lend the installation in the space, their home and the account
        const delegate = principal.installation.reference(spaceId, found.id);
        const permissions = PermissionSet.grant(opened.permissions, {
            space: spaceId,
            home: [this.login.subject.id, opened.home],
            account: place.placement.scope,
        });
        const granted = await this.login.granted(delegate);

        return {
            view,
            place,
            installation: found,
            home: opened.home,
            delegate,
            permissions,
            granted,
        };
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
        const spaceId = requested.place.placement.id;
        const installer = ObjectReference.at("createdBy", requested.installation);
        const isOwn = spaceId === requested.home || Subject.same(installer, person);

        // decide for a view reaching only its space, where it sees what the person sees
        const isWithinSpace = requested.permissions.every((entry) => entry.scope === spaceId);

        return isOwn || isWithinSpace;
    }

    /** Choose the view presenting an object most strongly among builds, or the view asked for. */
    static #present(
        views: readonly Declaration[],
        object: ObjectReference,
        asked: string | undefined,
    ): {
        readonly rollout: InstallationRollout;
        readonly view: string;
    } {
        // rank each view of the name asked for presenting the object's type
        const presenters = views
            .filter(({ declaration }) => asked === undefined || declaration.name === asked)
            .flatMap(({ rollout, declaration }) =>
                ViewDescription.declaration
                    .parse(declaration.description)
                    .presents.filter(
                        (presented) =>
                            presented.packageId === object.packageId &&
                            presented.type === object.type,
                    )
                    .map((presented) => ({
                        rollout,
                        view: declaration.name,
                        priority: presented.priority,
                    })),
            );

        // choose the strongest, refusing none
        const [chosen] = presenters.toSorted(
            (left, right) =>
                ViewPresentationPriority.compare(left.priority, right.priority) ||
                Text.compare(left.rollout.installation.name, right.rollout.installation.name) ||
                Text.compare(left.view, right.view),
        );
        if (chosen === undefined) {
            const missing = asked === undefined ? "no view" : `no view ${asked}`;
            throw MachineError.of(
                "PRESENTER_NOT_FOUND",
                `${object.scope} has no presenter ${missing} for ${object.type} objects`,
            );
        }

        return { rollout: chosen.rollout, view: chosen.view };
    }

    /** Write an installation's origin: the loopback domain's on loopback, else the space's. */
    static #origin(space: SpaceAddress, installation: string, endpoint: string): string {
        // move a loopback endpoint's port below the loopback domain
        const named = { ...SpaceAddress.origin(space), name: installation };
        const url = new URL(endpoint);
        if (HostAddress.isLoopbackName(url.hostname)) {
            url.hostname = InstallationOrigin.format(named, LOOPBACK_DOMAIN);

            return url.origin;
        }

        // name it one label below the space domain, wherever the space runs
        return `https://${InstallationOrigin.format(named, DOMAINS.space)}`;
    }
}
