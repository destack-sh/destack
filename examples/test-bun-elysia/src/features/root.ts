import { Elysia } from "elysia";
import { BROADCAST_CHANNEL } from "./websocket";

export const rootFeature = new Elysia({ name: "root" }).get("/", ({ metricsSnapshot }) => ({
    message: "Destack Bun + Elysia advanced demo",
    metrics: metricsSnapshot(),
    endpoints: {
        root: "GET /",
        profiles: "POST /profiles",
        signedCookie: "GET /cookies/session",
        verifyCookie: "GET /cookies/session/verify",
        secureGuard: "GET /secure/profile?token=secret-token",
        auditGuard: "POST /secure/audit?token=secret-token",
        stream: "GET /streams/time",
        websocket: "WS /ws/echo",
        publish: `GET /ws/publish/:message -> ${BROADCAST_CHANNEL}`,
        teapot: "GET /teapot"
    }
}));
