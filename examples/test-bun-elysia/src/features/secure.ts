import { Elysia, t } from "elysia";

export const secureFeature = new Elysia({ name: "secure" }).guard(
    {
        query: t.Object({
            token: t.String({ minLength: 8 })
        })
    },
    (secured) =>
        secured.group("/secure", (group) =>
            group
                .get("/profile", ({ query, noteGuardHit }) => {
                    noteGuardHit();
                    const tokenPreview = `${query.token.slice(0, 4)}****`;

                    return {
                        tokenPreview,
                        issuedAt: new Date().toISOString()
                    };
                })
                .post(
                    "/audit",
                    ({ body, noteGuardHit }) => {
                        noteGuardHit();

                        return {
                            received: body,
                            processedAt: new Date().toISOString()
                        };
                    },
                    {
                        body: t.Object({
                            eventId: t.String(),
                            severity: t.Union([
                                t.Literal("info"),
                                t.Literal("warn"),
                                t.Literal("critical")
                            ]),
                            details: t.Optional(t.String())
                        })
                    }
                )
        )
);
