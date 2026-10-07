import { DOMAINS } from "@destack/account/address";
import { schema } from "@destack/schema";
import { TunnelProtocol } from "../../session/index.ts";
import { Tunnel, TUNNEL_PATH } from "../../server/index.ts";
import { machineTokens } from "../../test/token.ts";
import {
    DurableObjectTunnel,
    type DurableObjectTunnelNamespace,
    type DurableObjectTunnelState,
    DurableObjectTunnelHost,
} from "../tunnel.ts";
import { DESTINATION, JURISDICTION_PATH, MACHINE, NAME, RENAME_PATH } from "./fixture.ts";

/** The test relay's bindings. */
interface Environment {
    /** The machines' objects. */
    readonly TUNNEL: DurableObjectTunnelNamespace;
    /** The relay's tunnel URL the machine dials. */
    readonly TUNNEL_URL: string;
}

/** The test machine's object, renewing tokens that name their lifetime. */
export class TestTunnel extends DurableObjectTunnel {
    /** Keep the machine's tunnel at the test relay's URL. */
    constructor(state: DurableObjectTunnelState, environment: Environment) {
        super(state, environment, {
            url: environment.TUNNEL_URL,
            tokens: machineTokens(MACHINE),
            report: (error) => {
                throw error;
            },
            domains: DOMAINS,
        });
    }
}

/** The jurisdictions the test relay reached its machines' objects in, kept by its isolate. */
const reached = new Set<string>();

/** Record each narrowing of a namespace and keep its objects in it, as workerd implements no jurisdiction. */
function recorded(namespace: DurableObjectTunnelNamespace): DurableObjectTunnelNamespace {
    return {
        idFromName: (name) => namespace.idFromName(name),
        get: (id) => namespace.get(id),
        jurisdiction: (name) => {
            reached.add(name);

            return recorded(namespace);
        },
    };
}

/** The test relay's Worker: admitting tunnels by their tokens, telling the machine renames and forwarding every other request to it. */
export default {
    /** Route a request to the machine's object. */
    async fetch(request: Request, environment: Environment): Promise<Response> {
        const tunnels = new DurableObjectTunnelHost(recorded(environment.TUNNEL), { eu: "eu" });
        const url = new URL(request.url);

        // list the jurisdictions the machines' objects were reached in
        if (url.pathname === JURISDICTION_PATH) {
            return Response.json([...reached]);
        }

        // admit a tunnel by its offered token under the test name
        else if (url.pathname === TUNNEL_PATH) {
            const token = TunnelProtocol.token(request.headers.get("sec-websocket-protocol"));
            if (token === undefined) {
                return new Response(null, { status: 401 });
            }
            const { machineId, caller } = await Tunnel.admit(machineTokens(MACHINE), token);

            return tunnels.open(
                { ...DESTINATION, machineId, lapsesAt: caller.lapsesAt, name: NAME },
                request,
            );
        }
        // tell the machine its new name
        else if (url.pathname === RENAME_PATH) {
            await tunnels.rename(DESTINATION, schema.string().parse(url.searchParams.get("name")));

            return new Response(null, { status: 204 });
        }
        // forward any other request for the machine's name
        else {
            return tunnels.fetch(DESTINATION, request);
        }
    },
};
