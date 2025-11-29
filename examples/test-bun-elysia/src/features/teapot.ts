import { Elysia } from "elysia";

export const teapotFeature = new Elysia({ name: "teapot" }).get(
    "/teapot",
    ({ set }) => {
        set.status = 418;
        set.headers["x-powered-by"] = "elysia";

        return {
            message: "short and stout"
        };
    }
);
