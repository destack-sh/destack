import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { application } from "../object/application.ts";
import { OAuthConsent } from "../object/oauth-consent.ts";

/** The random bytes of a client identifier, 192 bits in 32 base64url characters. */
const CLIENT_ID_BYTES = 24;

/** Implement applications. */
export function implementApplication(): typeof application {
    return application.implement({
        methods: {
            create,
            // end the consents and tokens the provider granted it with the client
            delete: async (call, next) => {
                await OAuthConsent.withdraw(call, call.target.clientId);

                return next();
            },
        },
    });
}

/** Register a client under a new random public identifier. */
async function create(
    context: MethodContextOf<typeof application, "create">,
    next: NextOf<typeof application, "create">,
): Promise<ResultOf<typeof application, "create">> {
    // draw the public identifier
    const bytes = context.environment.random.bytes(CLIENT_ID_BYTES);
    const clientId = bytes.toBase64({ alphabet: "base64url", omitPadding: true });

    return next(context.with({ input: { ...context.input, clientId } }));
}
