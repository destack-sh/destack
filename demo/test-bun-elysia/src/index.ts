import { Elysia } from "elysia";
import { cookiesFeature as cookiesApp } from "./features/cookies";
import { metricsFeature as metricsApp } from "./features/metrics";
import { parserFeature as parserApp } from "./features/parser";
import { profilesFeature as profilesApp } from "./features/profiles";
import { rootFeature as rootApp } from "./features/root";
import { secureFeature as secureApp } from "./features/secure";
import { streamsFeature as streamsApp } from "./features/streams";
import { teapotFeature as teapotApp } from "./features/teapot";
import { websocketFeature as websocketApp } from "./features/websocket";

const port = Number.parseInt(process.env.PORT ?? "3000", 10);

export const app = new Elysia({
    cookie: {
        secrets: "destack-bun-elysia",
        sign: ["session"],
    },
})
    .use(metricsApp)
    .use(parserApp)
    .use(profilesApp)
    .use(cookiesApp)
    .use(secureApp)
    .use(streamsApp)
    .use(websocketApp)
    .use(teapotApp)
    .use(rootApp)
    .listen(port, ({ hostname, port: boundPort }) => {
        console.log(`🦊 Elysia is running at http://${hostname}:${boundPort}`);
    });

export type App = typeof app;
