import { PackageInspection } from "@destack/package/code";
import { PackageId } from "@destack/package";
import { contained, principal, through, union } from "@destack/access";
import * as accountObject from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { StackEvaluation, Submission } from "@destack/stack/object";
import { WorkingTree, WorkingTreeSource } from "./tree.ts";

/** A stack package directory of a checkout, the space that builds it, and its arguments. */
export const CheckoutStack = schema.object({
    /** The space whose stack follows the checkout, building there. */
    space: schema.identifier("space"),
    /** The person building and submitting the stack. */
    login: schema.identifier("login"),
    /** The stack package directory, relative to the working tree's root. */
    directory: schema.string().min(1),
    /** The exported stack. */
    export: schema.string().min(1),
    /** Arguments for a parameterised definition. */
    parameters: schema.jsonObject(),
});
/** A stack package directory of a checkout to build and submit. */
export type CheckoutStack = schema.Infer<typeof CheckoutStack>;

/** A stack a checkout built, and the submission it hands its space's stack installation. */
const CheckoutSubmission = schema.object({
    /** The stack package the checkout built. */
    packageId: PackageId,
    /** The build with the export and arguments its space evaluates. */
    submission: Submission.extend({
        stack: StackEvaluation.pick({ export: true, parameters: true }),
    }),
});

/** A Git working directory registered on a machine. */
export const checkout = defineObject(
    {
        name: "checkout",
        plural: "checkouts",
        scope: accountObject.machine,
        fields: {
            /** The registered directory's absolute canonical path. */
            directory: field.string(schema.string().min(1)),
            /** The root of the directory's Git working tree. */
            root: field.string(schema.string().min(1)),
        },
        // let the machine's local client and the people who operate the machine work its checkouts
        permissions: {
            read: union(contained(principal.machine), through("scope", "operate")),
            create: union(contained(principal.machine), through("scope", "operate")),
            relocate: union(contained(principal.machine), through("scope", "operate")),
            delete: union(contained(principal.machine), through("scope", "operate")),
            submit: union(contained(principal.machine), through("scope", "operate")),
        },
        constraints: {
            directory: { kind: "unique", on: ["scope", "directory"] },
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
            onConflict: "directory",
        }),
        relocate: method.update("relocate", {
            fields: ["directory"],
            prepared: WorkingTree,
            isPredicted: false,
        }),
        delete: method.delete("delete", { prepared: schema.string() }),
        /** List the Destack packages of the workspace at or below the directory. */
        members: method.query({
            permission: "read",
            output: schema.array(schema.string().min(1)),
            prepared: schema.array(schema.string().min(1)),
        }),
        /** Inspect a package directory of the checkout for one of its outputs. */
        inspect: method.query({
            permission: "read",
            input: schema.object({
                /** The package directory, relative to the checkout's root. */
                directory: schema.string().min(1),
                /** The output whose runtime the inspection compiles for. */
                output: schema.string().min(1),
            }),
            output: PackageInspection,
            prepared: PackageInspection,
        }),
        /** Build a stack package directory of the checkout in a space, answering its submission. */
        submit: method.mutation({
            permission: "submit",
            input: CheckoutStack,
            output: CheckoutSubmission,
            prepared: CheckoutSubmission,
        }),
    }),
);
/** A checkout as its table stores it. */
export type Checkout = Select<typeof checkout.table>;
