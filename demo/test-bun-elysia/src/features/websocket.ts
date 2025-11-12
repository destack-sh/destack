import { Elysia } from "elysia";
import { metricsState } from "./metrics";

export const BROADCAST_CHANNEL = "destack-demo";

export const websocketFeature = new Elysia({ name: "websocket" })
    .ws("/ws/echo", {
        open(ws) {
            metricsState.wsConnections += 1;
            ws.subscribe(BROADCAST_CHANNEL);
            ws.send("connected to destack demo channel");
        },
        message(ws, message) {
            ws.publish(BROADCAST_CHANNEL, `broadcast:${message}`);
            ws.send(`echo:${message}`);
        },
        close() {
            metricsState.wsConnections = Math.max(0, metricsState.wsConnections - 1);
        }
    })
    .get("/ws/publish/:message", ({ params: { message }, server }) => {
        server?.publish(BROADCAST_CHANNEL, `server:${message}`);

        return {
            message,
            broadcastChannel: BROADCAST_CHANNEL
        };
    });
