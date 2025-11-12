import { Elysia } from "elysia";
import { cookiesFeature } from "./features/cookies";
import { metricsFeature } from "./features/metrics";
import { parserFeature } from "./features/parser";
import { profilesFeature } from "./features/profiles";
import { rootFeature } from "./features/root";
import { secureFeature } from "./features/secure";
import { streamsFeature } from "./features/streams";
import { teapotFeature } from "./features/teapot";
import { websocketFeature } from "./features/websocket";

const port = Number.parseInt(process.env.PORT ?? "3000", 10);

export const app = new Elysia({
    cookie: {
        secrets: "destack-bun-elysia",
        sign: ["session"]
    }
})
    .use(metricsFeature)
    .use(parserFeature)
    .use(profilesFeature)
    .use(cookiesFeature)
    .use(secureFeature)
    .use(streamsFeature)
    .use(websocketFeature)
    .use(teapotFeature)
    .use(rootFeature)
    .listen(port, ({ hostname, port: boundPort }) => {
        console.log(`🦊 Elysia is running at http://${hostname}:${boundPort}`);
    });

export type App = typeof app;
