import { PackageId } from "@destack/package/package";
import packageDefinition from "../../destack.json" with { type: "json" };
import { identifier, schema } from "@destack/schema";
import { Creation, defineProcedure } from "@destack/service/procedure";
import { page, PageRequest } from "@destack/service/page";
import { eventIterator } from "@destack/service";
import { ResourceName } from "@destack/resource";
import { OperationError } from "@destack/service/operation";
import { ViewDefinition } from "@destack/package/view";
import { PackagePath } from "@destack/package/file";

/** A named frontend compiled within a development preview. */
export const PreviewView = ViewDefinition.extend({
    /** Package-declared view name. */
    name: ResourceName,
    /** BuildService preview serving this frontend, absent before compilation. */
    buildPreviewId: schema.uuid().nullable(),
});

/** Source and identity retained for one editable preview. */
const selection = schema.object({
    /** Local preview identifier. */
    id: identifier("preview"),
    /** Registered source checkout. */
    checkoutId: identifier("checkout"),
    /** Package directory relative to the checkout root. */
    packageDirectory: schema.union([schema.literal("."), PackagePath]),
    /** Space supplying development resources and permissions. */
    spaceId: identifier("space"),
    /** Retained universe identity authorizing development. */
    loginId: identifier("login"),
    /** Concurrent workload executions, independent of open windows. */
    instanceIds: schema.array(identifier("instance")),
    /** Named frontends discovered in the selected package. */
    views: schema.array(PreviewView),
});

/** A build watcher and workloads running against an explicit development space. */
export const Preview = schema.discriminatedUnion("status", [
    selection.extend({
        /** Lifecycle state without an available frontend. */
        status: schema.enum(["starting", "stopping", "stopped"]),
    }),
    selection.extend({
        /** The preview is ready for an authenticated browser connection. */
        status: schema.literal("running"),
    }),
    selection.extend({
        /** Startup or runtime failed. */
        status: schema.literal("failed"),
        /** Public failure details. */
        error: OperationError,
    }),
]);

/** Editable previews retain explicit package and space selection. */
export const preview = {
    /** Open a named frontend without creating another backend development instance. */
    view: {
        list: access("preview", "read")
            .route({ method: "GET", path: "/previews/{previewId}/views" })
            .input(schema.object({ previewId: identifier("preview") }))
            .output(schema.array(PreviewView)),
        open: access("preview", "open")
            .route({ method: "POST", path: "/previews/{previewId}/views/{view}/open" })
            .input(
                Creation.extend({
                    previewId: identifier("preview"),
                    view: ResourceName,
                    path: schema.string().regex(/^\/(?!\/)[^\\\r\n]*$/),
                }),
            )
            .output(
                schema.object({
                    /** Single-use authorized frontend location. */
                    url: schema.httpUrl(),
                    /** Redemption expiry, in UTC milliseconds. */
                    expiresAt: schema.number().int(),
                }),
            ),
    },
    list: access("preview", "read")
        .route({ method: "GET", path: "/previews" })
        .input(PageRequest)
        .output(page(Preview)),
    get: access("preview", "read")
        .route({
            method: "GET",
            path: "/previews/{previewId}",
        })
        .input(schema.object({ previewId: identifier("preview") }))
        .output(Preview),
    start: access("preview", "start")
        .route({ method: "POST", path: "/previews" })
        .input(
            Creation.extend({
                checkoutId: identifier("checkout"),
                packageDirectory: schema.union([schema.literal("."), PackagePath]),
                spaceId: identifier("space"),
                loginId: identifier("login"),
            }),
        )
        .output(Preview),
    stop: access("preview", "stop")
        .route({
            method: "POST",
            path: "/previews/{previewId}/stop",
        })
        .input(Creation.extend({ previewId: identifier("preview") }))
        .output(Preview),
    /** Stream complete preview state through startup, edits, failure and shutdown. */
    watch: access("preview", "read")
        .route({ method: "GET", path: "/previews/{previewId}/watch" })
        .input(schema.object({ previewId: identifier("preview") }))
        .output(eventIterator(Preview)),
};

/** Require authenticated host access before inspecting paths or starting preview code. */
function access(resource: string, action: string) {
    return defineProcedure({
        authentication: "host",
        permission: {
            packageId: PackageId.parse(packageDefinition.id),
            type: resource,
            name: action,
        },
        audit: action !== "read",
    });
}
