import { PackageId } from "@destack/package";
import * as accountObject from "@destack/account/object";
import { type Select, unique } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { StackEvaluation, Submission } from "@destack/space/object";

/** A stack package directory in a checkout, the space building it, and the arguments evaluating it. */
export const StackSource = schema.object({
    /** The space whose stack follows the checkout, building there. */
    space: schema.identifier("space"),
    /** The person building and submitting the stack. */
    login: schema.identifier("login"),
    /** The stack package directory, relative to the working tree's root. */
    directory: schema.string().min(1),
    /** The exported space declaration. */
    export: schema.string().min(1),
    /** Arguments for a parameterised definition. */
    parameters: schema.record(schema.string(), schema.json()),
});
/** A stack package directory in a checkout to submit. */
export type StackSource = schema.Infer<typeof StackSource>;

/** The canonical directory and Git working tree root a checkout's creation resolves. */
export const WorkingTree = schema.object({
    /** The registered directory's absolute canonical path. */
    directory: schema.string(),
    /** The root of the directory's Git working tree. */
    root: schema.string(),
});
/** The canonical directory and Git working tree root a checkout's creation resolves. */
export type WorkingTree = schema.Infer<typeof WorkingTree>;

/** Where a checkout's working tree comes from: the directory as it is, a new empty repository, or a clone of a remote. */
export const WorkingTreeSource = schema.discriminatedUnion("kind", [
    schema.object({
        /** Register the Git working tree already at the directory. */
        kind: schema.literal("existing"),
    }),
    schema.object({
        /** Initialize an empty Git repository at the directory. */
        kind: schema.literal("empty"),
    }),
    schema.object({
        /** Clone a remote Git repository into the directory. */
        kind: schema.literal("clone"),
        /** The remote Git repository. */
        remote: schema.string().min(1),
        /** The branch, tag or commit to check out, else the remote's default branch. */
        ref: schema.string().min(1).exactOptional(),
    }),
]);
/** Where a checkout's working tree comes from. */
export type WorkingTreeSource = schema.Infer<typeof WorkingTreeSource>;

/** The Destack packages of a checkout's workspace at or below its directory, relative to its root, each building as its own build. */
export const Members = schema.array(schema.string().min(1));

/** A stack a checkout built and the submission it hands its space. */
export const CheckoutSubmission = schema.object({
    /** The stack package the checkout built. */
    packageId: PackageId,
    /** The space's build and its evaluation to submit to the space's stack installation. */
    submission: Submission.extend({ evaluation: StackEvaluation }),
});
/** A stack a checkout built and the submission it hands its space. */
export type CheckoutSubmission = schema.Infer<typeof CheckoutSubmission>;

/** A Git working directory registered on a machine. */
export const checkout = defineObject({
    name: "checkout",
    plural: "checkouts",
    scope: accountObject.machine,
    fields: {
        /** The registered directory's absolute canonical path. */
        directory: field.string(schema.string().min(1)),
        /** The root of the directory's Git working tree. */
        root: field.string(schema.string().min(1)),
    },
    constraints: (columns) => [unique("checkout_directory").on(columns.scope, columns.directory)],
    permissions: ["read", "create", "relocate", "delete", "submit"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", {
            fields: ["directory"],
            input: schema.object({
                /** Where the working tree comes from, the directory as it is by default. */
                source: WorkingTreeSource.default({ kind: "existing" }),
            }),
            prepared: WorkingTree,
            isPredicted: false,
        }),
        relocate: method.update("relocate", {
            fields: ["directory"],
            prepared: WorkingTree,
            isPredicted: false,
        }),
        delete: method.delete("delete", { prepared: schema.string() }),
        /** List the Destack packages of the workspace at or below the directory, relative to the root, as its lockfile names them now. */
        members: method.query({
            permission: "read",
            output: Members,
            prepared: Members,
        }),
        submit: method.query({
            permission: "submit",
            input: StackSource,
            output: CheckoutSubmission,
            prepared: CheckoutSubmission,
        }),
    }),
});
/** A checkout as its table stores it. */
export type Checkout = Select<typeof checkout.table>;
