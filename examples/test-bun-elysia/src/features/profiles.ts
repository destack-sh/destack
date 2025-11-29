import { Elysia, t } from "elysia";

const profileSchema = t.Object({
    id: t.Number({ minimum: 1 }),
    username: t.String(),
    tags: t.Array(t.String()),
    source: t.Optional(t.String())
});

export const profilesFeature = new Elysia({ name: "profiles" }).post(
    "/profiles",
    ({ body, store }) => {
        store.metrics.lastProfileId = body.id;

        return {
            id: body.id,
            normalizedName: body.username.toLowerCase(),
            tags: body.tags,
            source: body.source ?? "json"
        };
    },
    {
        body: profileSchema,
        transform({ body }) {
            body.username = body.username.trim();
            body.tags = [...new Set(body.tags.map((tag) => tag.trim()))];
            body.source ??= "json";
        }
    }
);
