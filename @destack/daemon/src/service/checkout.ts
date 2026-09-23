import { PackageId } from "@destack/package/package";
import definition from "../../destack.json" with { type: "json" };
import { identifier, schema } from "@destack/schema";
import { Creation, defineProcedure } from "@destack/service/procedure";
import { eventIterator } from "@destack/service";
import { defineOperation } from "@destack/service/operation";
import { page, PageRequest } from "@destack/service/page";

/** A registered Git working directory. */
export const Checkout = schema.object({
    /** Stable local registration identifier. */
    id: identifier("checkout"),
    /** Registered repository identity, absent for an independent local repository. */
    repositoryId: identifier("repository").nullable(),
    /** Canonical absolute working directory. */
    directory: schema.string().min(1),
    /** Registration time in UTC epoch milliseconds. */
    createdAt: schema.number().int(),
});

/** A registered Git working directory. */
export type Checkout = schema.Infer<typeof Checkout>;

/** Register a repository without moving its files or changing its Git configuration. */
export const CheckoutRegistration = schema.object({
    /** Existing working directory, resolved to its Git root. */
    directory: schema.string().min(1),
    /** Repository identity established by the repository service. */
    repositoryId: identifier("repository").optional(),
});

/** Inputs used to register a working directory. */
export type CheckoutRegistration = schema.Infer<typeof CheckoutRegistration>;

/** Observed Git state; Git remains authoritative for branches and working changes. */
export const CheckoutStatus = schema.discriminatedUnion("status", [
    schema.object({
        /** Registered working directory. */
        checkout: Checkout,
        /** An unavailable checkout has no inferred Git state. */
        status: schema.enum(["missing", "invalid"]),
    }),
    schema.object({
        /** Registered working directory. */
        checkout: Checkout,
        /** Availability after external directory changes. */
        status: schema.literal("available"),
        /** Current branch, absent for detached or unavailable checkouts. */
        branch: schema.string().nullable(),
        /** Current commit, absent for unborn or unavailable checkouts. */
        commit: schema.string().nullable(),
        /** Whether the index or working directory contains changes. */
        dirty: schema.boolean(),
        /** Git operation requiring completion. */
        operation: schema.enum(["merge", "rebase", "cherry-pick", "revert"]).nullable(),
    }),
]);

/** Observable repository initialization or download. */
export const CheckoutOperation = defineOperation(
    Checkout,
    schema.object({
        /** Destination directory retained through completion and failure. */
        directory: schema.string(),
    }),
);

/** Register and inspect editable repositories on this host. */
export const checkout = {
    /** Initialize an ordinary Git repository in the selected directory. */
    create: access("create")
        .route({ method: "POST", path: "/checkouts/create" })
        .input(Creation.extend({ directory: schema.string().min(1) }))
        .output(CheckoutOperation.operation),
    /** Clone through ordinary Git transport, preserving the selected remote. */
    clone: access("create")
        .route({ method: "POST", path: "/checkouts/clone" })
        .input(
            Creation.extend({
                directory: schema.string().min(1),
                remote: schema.string().min(1),
                ref: schema.string().min(1).optional(),
                repositoryId: identifier("repository").optional(),
            }),
        )
        .output(CheckoutOperation.operation),
    /** Observe and cancel repository creation through the shared operation model. */
    operation: {
        get: access("read")
            .route({ method: "GET", path: "/checkout-operations/{id}" })
            .input(schema.object({ id: schema.uuid() }))
            .output(CheckoutOperation.operation),
        watch: access("read")
            .route({ method: "GET", path: "/checkout-operations/{id}/watch" })
            .input(schema.object({ id: schema.uuid() }))
            .output(eventIterator(CheckoutOperation.operation)),
        cancel: access("create")
            .route({ method: "POST", path: "/checkout-operations/{id}/cancel" })
            .input(Creation.extend({ id: schema.uuid() }))
            .output(CheckoutOperation.operation),
    },
    /** Rebind a registration to a repository moved by the user. */
    relocate: access("register")
        .route({ method: "POST", path: "/checkouts/{checkoutId}/relocate" })
        .input(
            Creation.extend({
                checkoutId: identifier("checkout"),
                directory: schema.string().min(1),
            }),
        )
        .output(Checkout),
    /** Read Git and directory state without relying on previously observed events. */
    status: access("read")
        .route({ method: "GET", path: "/checkouts/{checkoutId}/status" })
        .input(schema.object({ checkoutId: identifier("checkout") }))
        .output(CheckoutStatus),
    /** Send an initial snapshot and refresh it after native Git or filesystem changes. */
    watch: access("read")
        .route({ method: "GET", path: "/checkouts/{checkoutId}/watch" })
        .input(schema.object({ checkoutId: identifier("checkout") }))
        .output(eventIterator(CheckoutStatus)),
    list: access("read")
        .route({ method: "GET", path: "/checkouts" })
        .input(PageRequest)
        .output(page(Checkout)),
    get: access("read")
        .route({ method: "GET", path: "/checkouts/{checkoutId}" })
        .input(schema.object({ checkoutId: identifier("checkout") }))
        .output(Checkout),
    register: access("register")
        .route({ method: "POST", path: "/checkouts" })
        .input(CheckoutRegistration)
        .output(Checkout),
    unregister: access("unregister")
        .route({ method: "DELETE", path: "/checkouts/{checkoutId}" })
        .input(schema.object({ checkoutId: identifier("checkout") }))
        .output(schema.object({})),
};

/** Require local host authority before accessing working directories. */
function access(action: "read" | "register" | "unregister" | "create") {
    return defineProcedure({
        authentication: "host",
        permission: { packageId: PackageId.parse(definition.id), type: "checkout", name: action },
        audit: action !== "read",
    });
}
