import { none, through } from "@destack/access";
import { defineObject, device, field } from "@destack/object";
import { schema } from "@destack/schema";
import { MachineError } from "../../../error/index.ts";
import { controlled } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** How a person signs in on this computer: in the browser, with a device code, or by mail. */
export const SignInMethod = schema.discriminatedUnion("kind", [
    /** Sign in in the browser, which redirects back to this computer. */
    schema.object({ kind: schema.literal("browser") }),
    /** Enter a code at a page in any browser. */
    schema.object({ kind: schema.literal("code") }),
    /** Enter the code mailed to an address. */
    schema.object({ kind: schema.literal("email"), email: schema.email() }),
    /** Sign up with the code mailed to an address, the sign-up read when the machine starts it. */
    schema.object({
        kind: schema.literal("signUp"),
        email: schema.email(),
        signUp: schema.jsonObject(),
    }),
]);
/** How a person signs in on this computer. */
export type SignInMethod = schema.Infer<typeof SignInMethod>;

/** Where a sign-in stands, with what the person sees or answers at each step. */
export const SignInState = schema.discriminatedUnion("kind", [
    /** The machine starts the sign-in. */
    schema.object({ kind: schema.literal("starting") }),
    /** The person opens a page to sign in. */
    schema.object({ kind: schema.literal("open"), url: schema.url() }),
    /** The person enters a code at a page. */
    schema.object({ kind: schema.literal("enter"), url: schema.url(), userCode: schema.string() }),
    /** The sign-in waits for the code mailed to the person. */
    schema.object({ kind: schema.literal("waiting"), email: schema.email() }),
    /** The person answered the mailed code. */
    schema.object({
        kind: schema.literal("answered"),
        email: schema.email(),
        code: schema.string(),
    }),
    /** The person is signed in as a login. */
    schema.object({ kind: schema.literal("signedIn"), login: schema.identifier("login") }),
    /** The sign-in failed. */
    schema.object({ kind: schema.literal("failed"), reason: schema.string() }),
]);
/** Where a sign-in stands. */
export type SignInState = schema.Infer<typeof SignInState>;

/** A person signing in on this computer, which its machine runs. */
export const signIn = defineObject(
    {
        name: "sign-in",
        plural: "sign-ins",
        title: t`${plural(argument("count"), { one: "Sign-in", other: "Sign-ins" })}`,
        scope: device,
        storage: "local",
        fields: {
            /** How the person signs in. */
            method: field.json(SignInMethod).immutable(),
            /** Where the sign-in stands. */
            state: field.json(SignInState).default({ kind: "starting" }),
        },
        access: {
            // let the programs of this computer start, follow, answer and remove its sign-ins
            permissions: {
                read: through("device", "use"),
                start: through("device", "use"),
                answer: through("device", "use"),
                remove: through("device", "use"),
                record: none(),
            },
        },
        traits: [
            // let the machine run each sign-in and write where it stands
            controlled(),
        ],
    },
    (method) => ({
        /** Start signing a person in. */
        start: method.create("start", { fields: ["method"] }),
        /** Answer the code mailed to the person. */
        answer: method.mutation({
            title: t`Answer`,
            permission: "answer",
            input: schema.object({
                /** The mailed code. */
                code: schema.string().min(1),
            }),
        }),
        /** Remove a sign-in once read. */
        remove: method.delete("remove"),
        /** Record where the sign-in stands, as the machine running it. */
        advance: method.update("record", { visibility: "package", fields: ["state"] }),
    }),
).implement({
    methods: {
        answer: (call) => {
            // answer a sign-in waiting for its mailed code alone
            const { state } = call.target;
            if (state.kind !== "waiting") {
                throw MachineError.of(
                    "SIGN_IN_BLOCKED",
                    `cannot take a code for sign-in '${call.target.id}': it waits for no code`,
                );
            }

            return call.update({
                state: { kind: "answered", email: state.email, code: call.input.code },
            });
        },
    },
});
