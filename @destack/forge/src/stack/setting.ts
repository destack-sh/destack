import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

/** The GitHub App a forge opens connected GitHub repositories through. */
export const ForgeGitHub = schema.discriminatedUnion("kind", [
    schema.object({
        /** No GitHub App: the forge hosts platform and Git repositories alone. */
        kind: schema.literal("none"),
    }),
    schema.object({
        /** A GitHub App, whose private key and webhook secret the installation binds. */
        kind: schema.literal("app"),
        /** The app's client ID or app ID. */
        appId: schema.string().min(1),
    }),
]);
/** The GitHub App a forge opens connected GitHub repositories through. */
export type ForgeGitHub = schema.Infer<typeof ForgeGitHub>;

/** The forge installation's GitHub App, which its stack places. */
export const github = defineSetting({
    name: "github",
    title: "GitHub App",
    description: "The GitHub App the forge opens repositories connected at GitHub through.",
    schema: ForgeGitHub,
    default: { kind: "none" },
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
});
