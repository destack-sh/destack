import { DOMAINS } from "@destack/host";
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
import { MACHINE, NAME, RENAME_PATH } from "./fixture.ts";

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

/** The test relay's Worker: admitting tunnels by their tokens, telling the machine renames and forwarding every other request to it. */
export default {
    /** Route a request to the machine's object. */
    async fetch(request: Request, environment: Environment): Promise<Response> {
        const tunnels = new DurableObjectTunnelHost(environment.TUNNEL);
        const url = new URL(request.url);

        // admit a tunnel by its offered token under the test name
        if (url.pathname === TUNNEL_PATH) {
            const token = TunnelProtocol.token(request.headers.get("sec-websocket-protocol"));
            if (token === undefined) {
                return new Response(null, { status: 401 });
            }
            const { machineId, caller } = await Tunnel.admit(machineTokens(MACHINE), token);

            return tunnels.open({ machineId, lapsesAt: caller.lapsesAt, name: NAME }, request);
        }
        // tell the machine its new name
        else if (url.pathname === RENAME_PATH) {
            await tunnels.rename(MACHINE, schema.string().parse(url.searchParams.get("name")));

            return new Response(null, { status: 204 });
        }
        // forward any other request for the machine's name
        else {
            return tunnels.fetch(MACHINE, request);
        }
    },
};
