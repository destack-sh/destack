import { Elysia, t } from "elysia";

const cookieSchema = t.Cookie({
    session: t.Optional(t.String())
});

export const cookiesFeature = new Elysia({ name: "cookies" })
    .get(
        "/cookies/session",
        ({ cookie: { session } }) => {
            if (!session.value) {
                session.value = crypto.randomUUID();
                session.maxAge = 60 * 15;
            }

            return {
                session: session.value,
                signed: true
            };
        },
        { cookie: cookieSchema }
    )
    .get(
        "/cookies/session/verify",
        ({ cookie: { session } }) => ({
            hasSession: Boolean(session.value),
            session: session.value ?? null
        }),
        { cookie: cookieSchema }
    );
