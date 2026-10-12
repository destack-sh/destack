import type { ServiceClient } from "@destack/service/client";
import {
    FORWARDED_URL_HEADER,
    type Hibernation,
    type InstallationContext,
    PARTITION_HEADER,
    ROUTE_PATH,
} from "@destack/service/workload";
import type { accountService } from "@destack/account/service";
import { account, directoryShape, key, machine } from "@destack/account/object";
import { principal } from "@destack/access";
import { Uplink } from "@destack/sync";
import type { DatabaseConnection } from "@destack/db";
import type { AuditDestination } from "@destack/audit/server";
import { ObjectStore, Subscriber } from "@destack/object/server";
import { Clock, type Identifier, schema } from "@destack/schema";
import { Egress } from "@destack/service";
import { ServiceError } from "@destack/service/error";
import type { ServiceContext, ServiceImplementation } from "@destack/service/server";
import { connection, tunnel } from "../object/index.ts";
import { relayService } from "../service/index.ts";
import { MachineName } from "../session/index.ts";
import {
    DirectoryController,
    NameController,
    Relay,
    RELAY_PACKAGE,
    type RelayOptions,
} from "./relay.ts";
import { ADMISSION_HEADER, TUNNELS_PATH } from "./tunnel.ts";
import { Admission } from "./tunnel.ts";
import { Tunnels } from "./tunnels.ts";

/** What the relay's service runs as. */
export interface RelayServiceOptions extends Omit<RelayOptions, "publisher" | "store" | "itself"> {
    /** The relay's database, which its copies follow into. */
    readonly database: DatabaseConnection;
    /** The audit history the journal delivers tunnel openings and closes to. */
    readonly history?: AuditDestination;
    /** The universe's account service, called as the installation. */
    readonly accounts: ServiceClient<typeof accountService>;
    /** The relay installation, its partition, and how it reaches itself. */
    readonly installation: Pick<
        InstallationContext,
        "id" | "scope" | "partition" | "at" | "publisherAt"
    >;
    /** The WebSockets the instance's host keeps open for it, its tunnels' connections. */
    readonly hibernation: Hibernation;
    /** The clock the store and its tunnels run at. */
    readonly clock: Clock;
}

/** The relay's service with its copies, the router of names and the tunnels the instance holds. */
export interface RelayImplementation extends ServiceImplementation {
    /** The relay installation's store: its copies, and its tunnels and their connections. */
    readonly objects: ObjectStore;
    /** The router of names to its machines' tunnels. */
    readonly relay: Relay;
    /** The tunnels the instance holds. */
    readonly tunnels: Tunnels;
}

/** Implement the relay's service. */
export function implementRelay(options: RelayServiceOptions): RelayImplementation {
    // follow the account service's copies at the root, or the partition's chain from the root
    const { database, accounts, installation } = options;
    const below = installation.scope;
    const isRoot = installation.partition === undefined || installation.partition === below;
    const self = principal.installation.reference(below, installation.id);
    const uplink = Uplink.of(() => accounts.replica);
    const objects: ObjectStore = new ObjectStore({
        principal: self,
        objects: { tunnel, connection },
        policies: [account, machine, key],
        shapes: [directoryShape],
        database,
        callKey: options.callKey,
        clock: options.clock,
        origin: { package: RELAY_PACKAGE, service: relayService.name },
        ...(installation.partition === undefined ? {} : { partition: installation.partition }),
        subscriber: isRoot
            ? Subscriber.of(uplink, async () => [
                  ...(await objects.source.servedSubscriptions(below)),
                  Relay.subscription(below),
              ])
            : {
                  subscriptions: async () => {
                      // follow the partition's chain from the installation's root
                      const root = installation.publisherAt(
                          Egress.address(installation.id, below),
                          RELAY_PACKAGE.id,
                      );
                      const partition = schema.string().parse(installation.partition);
                      const chain = await objects.source.subscriptions(partition, {
                          isHome: false,
                      });

                      return chain.map((subscription) => ({ subscription, publisher: root }));
                  },
              },
    });

    // reach the tunnels in their partitions and the router at the root, through the installation itself
    const itself = installation.at(Egress.address(installation.id, below));
    const relay = new Relay({ ...options, publisher: uplink, store: objects, itself });
    const tunnels = new Tunnels({
        store: objects,
        hibernation: options.hibernation,
        url: relay.url,
        tokens: options.tokens,
        domains: relay.domains,
        report: options.report,
        withdraw: (machineId) => relay.withdrawAtRoot(machineId),
    });

    // run the router's controllers at the root and the lapses everywhere
    const controllers = isRoot
        ? [
              ...objects.controllers(),
              new NameController(relay),
              new DirectoryController(relay),
              relay.journal.controller(options.history),
              tunnels.controller,
          ]
        : [...objects.controllers(), tunnels.controller];

    return {
        ...objects.implement(relayService, objects.routed, controllers, {}),
        fetch: (request, context) => serveRelay(request, context, { relay, tunnels, self }),
        router: {},
        objects,
        relay,
        tunnels,
    };
}

/** Serve a request below the relay's mount. */
async function serveRelay(
    request: Request,
    context: ServiceContext,
    served: {
        readonly relay: Relay;
        readonly tunnels: Tunnels;
        readonly self: { readonly scope: string; readonly id: string };
    },
): Promise<Response | undefined> {
    // route a name's request a route hands the relay, restored to the URL it asked for
    const path = new URL(request.url).pathname;
    if (path === ROUTE_PATH) {
        return served.relay.route(restore(request));
    }
    // leave paths outside the tunnels' calls to the service's routes
    else if (!path.startsWith(`${TUNNELS_PATH}/`)) {
        return undefined;
    }

    // take the tunnels' calls from the installation itself alone
    const { subject } = context.requireCaller();
    if (
        !principal.installation.is(subject) ||
        subject.id !== served.self.id ||
        subject.scope !== served.self.scope
    ) {
        throw new ServiceError("FORBIDDEN", { message: "only the relay calls its tunnels" });
    }

    // serve one call of a machine's tunnel
    const [, called, verb] = path.slice(TUNNELS_PATH.length).split("/");
    const machineId = schema.identifier("machine").parse(called);

    return serveTunnel(request, machineId, verb ?? "", served);
}

/** Serve the router's call of a machine's tunnel. */
async function serveTunnel(
    request: Request,
    machineId: Identifier<"machine">,
    verb: string,
    served: { readonly relay: Relay; readonly tunnels: Tunnels },
): Promise<Response> {
    // accept a machine's admitted WebSocket in its partition
    if (verb === "open") {
        const admission = Admission.parse(
            JSON.parse(request.headers.get(ADMISSION_HEADER) ?? "null"),
        );

        return served.tunnels.accept(admission);
    }
    // forward a request through the tunnel under the host it asked for
    else if (verb === "forward") {
        return served.tunnels.fetch(machineId, restore(request));
    }
    // tell the machine its name
    else if (verb === "rename") {
        const { name } = MachineName.parse(await request.json());
        await served.tunnels.rename(machineId, name);

        return Response.json({});
    }
    // end the tunnel's connections
    else if (verb === "close") {
        return Response.json({ closed: await served.tunnels.end(machineId) });
    }
    // withdraw the endpoint of a machine left without a connection, at the root
    else if (verb === "withdraw") {
        await served.relay.withdraw(machineId);

        return Response.json({});
    }
    // refuse any other call
    else {
        throw new ServiceError("NOT_FOUND", { message: `no tunnel call ${verb}` });
    }
}

/** Restore a request a route or the router handed the relay to the URL it asked for. */
function restore(request: Request): Request {
    // read the URL it asked for
    const url = request.headers.get(FORWARDED_URL_HEADER);
    if (url === null) {
        throw new ServiceError("BAD_REQUEST", { message: "the handed request names no URL" });
    }

    // ask for it as the client did, without the handing headers
    const restored = new Request(url, request);
    restored.headers.delete(FORWARDED_URL_HEADER);
    restored.headers.delete(PARTITION_HEADER);

    return restored;
}
