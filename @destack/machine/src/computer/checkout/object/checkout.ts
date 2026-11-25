import { contained, principal, through, union } from "@destack/access";
import * as accountObject from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Commit, schema } from "@destack/schema";
import { GitAuthor } from "@destack/git";
import { Lease } from "@destack/resource";
import { WorkingTree, WorkingTreeSource } from "./tree.ts";
import { argument, plural, t } from "@destack/locale";

/** A checkout's working tree pushed to its reference: the commit and the reference's name. */
export const CheckoutPush = schema.object({
    /** The commit of the working tree as it was: HEAD itself while clean. */
    commit: Commit,
    /** The reference pointed at the commit, refs/checkouts/<machine>/<checkout>. */
    reference: schema.string().min(1),
});
/** A checkout's working tree pushed to its reference. */
export type CheckoutPush = schema.Infer<typeof CheckoutPush>;

/** A Git working tree registered on a machine. */
export const checkout = defineObject(
    {
        name: "checkout",
        plural: "checkouts",
        title: t`${plural(argument("count"), { one: "Checkout", other: "Checkouts" })}`,
        scope: accountObject.machine,
        fields: {
            /** The working tree's root, absolute and canonical. */
            directory: field.string(schema.string().min(1)),
            /** The URL of the working tree's origin as registered, absent without one. */
            remote: field.string().optional(),
        },
        constraints: {
            directory: { kind: "unique", on: ["scope", "directory"] },
        },
        access: {
            // let the machine's local client and the people who operate the machine work its checkouts
            permissions: {
                read: union(contained(principal.machine), through("scope", "operate")),
                create: union(contained(principal.machine), through("scope", "operate")),
                relocate: union(contained(principal.machine), through("scope", "operate")),
                delete: union(contained(principal.machine), through("scope", "operate")),
                push: union(contained(principal.machine), through("scope", "operate")),
            },
        },
    },
    (method) => ({
        create: method.create("create", {
            fields: ["directory"],
            input: schema.object({
                /** Where the working tree comes from. */
                source: WorkingTreeSource,
            }),
            prepared: WorkingTree,
            isPredicted: false,
            onConflict: { key: "directory", update: ["remote"] },
        }),
        relocate: method.update("relocate", {
            fields: ["directory"],
            prepared: WorkingTree,
            isPredicted: false,
        }),
        delete: method.delete("delete", { prepared: schema.string() }),
        /** List the Destack packages of the workspace at or below the directory. */
        members: method.query({
            title: t`List members`,
            permission: "read",
            output: schema.array(schema.string().min(1)),
            prepared: schema.array(schema.string().min(1)),
        }),
        /** Commit the working tree as it is and point the checkout's reference at the commit. */
        push: method.mutation({
            title: t`Push`,
            permission: "push",
            input: schema.object({
                /** The lease to write the repository, which the person takes from its forge. */
                lease: Lease,
                /** What to push: the working tree as it is, or HEAD with no uncommitted change. */
                of: schema.enum(["tree", "head"]),
                /** The author of the snapshot commit. */
                author: GitAuthor,
            }),
            output: CheckoutPush,
            prepared: CheckoutPush,
        }),
    }),
);
/** A checkout as its table stores it. */
export type Checkout = Select<typeof checkout.table>;
