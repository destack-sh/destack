import type { GrantClaim } from "@destack/access";
import { Snapshot } from "@destack/db";
import type { Setting } from "@destack/setting";
import { setting, SettingValue } from "@destack/setting/object";
import { spaceAliases } from "@destack/account/declare";
import { HOME_SPACE, RESIDENCY_SPACE } from "@destack/account/object";
import { GrantPermission, PermissionSet, principal } from "@destack/access";
import { InstallationOrigin, SpaceAddress } from "@destack/account/address";
import type { PlacementEndpoint } from "@destack/account/object";
import type { Login } from "@destack/account/native";
import accountPackage from "@destack/account/package";
import {
    type Installation,
    InstallationMount,
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
import { connectBunSqlite } from "@destack/db/bun";
import { Package, PackageId } from "@destack/package";
import {
    type ViewCategory,
    ViewDescription,
    ViewPresentationPriority,
} from "@destack/package/manifest";
import { type Identifier, nth, schema, Text } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { InstallationView } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { type Address, TARGET_PARAMETER, VIEW_KIND, ViewAddress } from "@destack/view/declare";
import { MachineError } from "../../error/index.ts";
import type { Device } from "./device.ts";
import { Visit } from "./object/index.ts";

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

    /** Connect to an installation's package below the machine serving its space. */
    async installation(
        space: Identifier<"space"> | SpaceAddress,
        name: string,
        packageId: PackageId,
    ): Promise<ClientOptions> {
        // send the universe's token for the installation's package bound to the space, which its machine verifies
        const { url, partition } = await this.#mount(space, name, packageId);

        return { url, fetch: (request) => this.login.send(request, packageId, { partition }) };
    }

    /** Find an installation's package below the machine serving its space, and the space. */
    async #mount(
        space: Identifier<"space"> | SpaceAddress,
        name: string,
        packageId: PackageId,
    ): Promise<{ readonly url: string; readonly partition: Identifier<"space"> }> {
        // locate the space, then the installation's mount below its machine
        const located = await this.device.locate(space);
        const partition = located.placement.space;
        const url = `${InstallationMount.url(located.url, partition, name)}${PackageMount.path(packageId)}`;

        return { url, partition };
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
        const { store } = this.#clients(await this.device.locate(home));
        const views = await Session.#views(store);

        // take the view of the category, the first by installation and name
        const [found] = views
            .filter(({ description }) => description.categories.includes(category))
            .toSorted(
                (left, right) =>
                    Text.compare(left.installation, right.installation) ||
                    Text.compare(left.view, right.view),
            );
        if (found === undefined) {
            throw MachineError.of("VIEW_NOT_FOUND", `${home} has no ${category} view`);
        }

        return { space: home, installation: found.installation, view: found.view, path };
    }

    /** Open a scope's objects as the person, following their moves, until the stack closes. */
    async open<const Objects extends Readonly<Record<string, ObjectType>>>(
        location:
            | { readonly scope: string }
            | {
                  readonly space: Identifier<"space"> | SpaceAddress;
                  readonly installation?: string;
              },
        objects: Objects,
        stack: AsyncDisposableStack,
    ): Promise<ObjectClient<Objects>> {
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
            scopeId = located.placement.space;
            reached = atMachine(async () => located);
        }
        // call a scope at the person's universe, such as their account, with an access token
        else {
            scopeId = location.scope;
            reached = this.login.universe(serving.id);
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
            reconnect = () => atMachine(async () => this.device.locate(location.space));
        }
        // call the scope again at the person's universe
        else {
            endpoint = reached;
            reconnect = () => reached;
        }

        // store the objects in memory, closed after the client
        const { environment } = this.device;
        const database = await connectBunSqlite(":memory:", ObjectClient.tables(objects), {
            environment,
        });
        stack.defer(() => database.close());

        // follow them as the person until the stack closes
        return ObjectClient.start(
            {
                environment,
                database,
                objects,
                scope: scopeId,
                caller: this.login.subject,
                endpoint,
                reconnect,
                homes: (scope) => this.store(scope),
            },
            stack,
        );
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
            const { space } = place.placement;
            await Visit.record(this.device.visits, this.login.subject, space, address.object);
        }

        // exchange the person's token for the installation's package acting under the grant in the space
        const token = await this.login.token(PackageId.parse(found.packageId), {
            partition: place.placement.space,
            grants: [grant.id],
        });
        const origin = Session.#origin(view.space, view.installation, place.url);

        return { url: ViewAddress.url(view, origin, token), target: found.id };
    }

    /** Sign a proof of the client the person registered on this computer. */
    prove(): Promise<string> {
        return this.login.prove();
    }

    /** Resolve one of the person's settings from a replica of their values. */
    async setting<Value extends schema.Schema>(
        declared: Setting<Value>,
    ): Promise<schema.Infer<Value>> {
        // follow the person's values at the universe as the person
        const scope = this.login.subject.id;
        await using stack = new AsyncDisposableStack();
        const settings = await this.open({ scope }, { setting }, stack);

        // wait for the values and the access rows of the person's chain
        await settings.query.setting.findMany({});

        // resolve the value along the chain
        return await SettingValue.resolve(Snapshot.live(settings.database), declared, { scope });
    }

    /** Resolve the space a person types: an address, identifier, alias, or one of their spaces. */
    async resolve(name: string): Promise<Identifier<"space">> {
        // take an identifier as given, and find an address's space
        const address = SpaceAddress.safeParse(name);
        const id = schema.identifier("space").safeParse(name);
        if (id.success) {
            return id.data;
        } else if (address.success) {
            return (await this.device.locate(address.data)).placement.space;
        }

        // take the person's own name for a space
        const alias = (await this.setting(spaceAliases))[name];
        if (alias !== undefined) {
            return (await this.device.locate(alias)).placement.space;
        }

        // take the person's home and their residency's space
        if (name === HOME_SPACE) {
            return this.login.home();
        } else if (name === RESIDENCY_SPACE) {
            return this.login.residency();
        }

        // find the person's one space of that name among every account's
        const matching = (await this.login.spaces()).filter(
            (each) => SpaceAddress.origin(each).space === name,
        );
        if (matching.length === 1) {
            return (await this.device.locate(nth(matching, 0))).placement.space;
        } else if (matching.length === 0) {
            throw MachineError.of("SPACE_NOT_FOUND", `no space of yours is called ${name}`);
        } else {
            throw MachineError.of(
                "SPACE_AMBIGUOUS",
                `${name} matches several of your spaces: use one of ${matching.join(", ")}`,
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
            const placed = await this.device.locate(Session.#space(schema.string().parse(rest[0])));
            const packageId = PackageId.parse(rest[1]);

            return this.send(below(rest.slice(2), UNRESOLVED_ORIGIN), packageId, placed);
        }
        // send to an installation's package at its origin, with the token bound to its space
        else if (kind === "installation") {
            const space = Session.#space(schema.string().parse(rest[0]));
            const name = schema.string().parse(rest[1]);
            const packageId = PackageId.parse(rest[2]);
            const { url, partition } = await this.#mount(space, name, packageId);

            return this.login.send(below(rest.slice(3), url), packageId, { partition });
        }
        // refuse any other target
        else {
            throw MachineError.of(
                "TARGET_INVALID",
                `${String(kind)} is no target: use universe/<package>, space/<space>/<package> or installation/<space>/<installation>/<package>`,
            );
        }
    }

    /** Send a request to a package: at the person's issuer, or in a space. */
    async send(
        request: Request,
        packageId: PackageId,
        at: "universe" | PlacementEndpoint,
    ): Promise<Response> {
        const url = new URL(request.url);
        const below = `${PackageMount.path(packageId)}${url.pathname}${url.search}`;

        // send to a universe package at the person's issuer
        if (at === "universe") {
            return this.login.send(
                new Request(new URL(below, this.login.issuer.url).href, request),
                packageId,
            );
        }
        // prove the person's client to this computer's machine below the space's mount
        else if (at.placement.machine === (await this.device.loopback.computer()).machineId) {
            const path = `${SpaceMount.path(at.placement.space)}${PackageMount.path(packageId)}`;

            return this.#sendOwn(request, path, at.url);
        }
        // send the universe's token bound to the space to another machine
        else {
            const target = new URL(`${SpaceMount.path(at.placement.space)}${below}`, at.url);

            return this.login.send(new Request(target.href, request), packageId, {
                partition: at.placement.space,
            });
        }
    }

    /** Send a request below a path at this computer's machine, proving the person's client. */
    async #sendOwn(request: Request, path: string, origin: string): Promise<Response> {
        // address the target below the path with a fresh proof of the person's client
        const url = new URL(request.url);
        const target = new URL(`${path}${url.pathname}${url.search}`, origin);
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
            store: ObjectClient.store(endpoint(stackPackage.id), this.device.environment),
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
        const spaceId = place.placement.space;
        const { store, stack, objects } = this.#clients(place);

        // choose the view presenting an object among the views the space serves
        let view: ViewAddress;
        if ("object" in address) {
            const views = await Session.#views(store);
            const presented = Session.#present(views, address.object, address.view);
            const query = new URLSearchParams({
                [TARGET_PARAMETER]: JSON.stringify(address.object),
            });
            view = { space: address.space, ...presented, path: `/?${query.toString()}` };
        }
        // take the addressed view
        else {
            view = address;
        }

        // find the installation by its name, and open its view
        const { items } = await objects.installation.list({
            scope: spaceId,
            where: { name: view.installation },
        });
        const [found] = items;
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
                `${object.scope} has no presenter ${missing} for ${object.type} objects`,
            );
        }

        return { installation: chosen.installation, view: chosen.view };
    }

    /** Read the views a space's named installations serve, as its store describes them. */
    static async #views(store: ReturnType<typeof ObjectClient.store>): Promise<SpaceView[]> {
        const description = await ObjectStoreDescription.read(store);

        return description.installations.flatMap((installation) => {
            const { name } = installation;

            return name === undefined
                ? []
                : description.declarations(installation, VIEW_KIND).map((declared) => ({
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
