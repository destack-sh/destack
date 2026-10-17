import { defineSchema, schema, Version } from "@destack/schema";
import { PermissionRequest } from "@destack/access";
import { DeclarationName, PackageId, Sandbox } from "@destack/package";
import { ObjectTypeReference } from "@destack/sync";

/** A JSON Schema. */
const JsonSchema = schema.jsonObject();

/** A procedure's value or event-stream schema. */
export const PayloadDescription = defineSchema(
    schema.union([
        schema.object({
            /** A single validated value. */
            kind: schema.literal("value"),
            /** The value's JSON Schema. */
            schema: JsonSchema,
        }),
        schema.object({
            /** An event iterator. */
            kind: schema.literal("stream"),
            /** The yielded event schema. */
            yields: JsonSchema,
            /** The return value schema. */
            returns: JsonSchema.exactOptional(),
        }),
    ]),
);

/** A procedure's address, payloads and errors. */
export const ProcedureDescription = defineSchema(
    schema.object({
        /** The procedure's key path. */
        name: schema.array(schema.string().min(1)),
        /** The HTTP method. */
        method: schema.string().exactOptional(),
        /** The HTTP path. */
        path: schema.string().exactOptional(),
        /** The OpenAPI operation identifier. */
        operationId: schema.string().exactOptional(),
        /** The procedure annotations. */
        metadata: schema.jsonObject().exactOptional(),
        /** The input. */
        input: PayloadDescription.exactOptional(),
        /** The output. */
        output: PayloadDescription.exactOptional(),
        /** The error codes and payloads. */
        errors: schema.record(
            schema.string(),
            schema.object({
                /** The HTTP status. */
                status: schema.number().int().exactOptional(),
                /** The default error message. */
                message: schema.string().exactOptional(),
                /** The error payload schema. */
                data: JsonSchema.exactOptional(),
            }),
        ),
    }),
);
/** A procedure's address, payloads and errors. */
export type ProcedureDescription = schema.Infer<typeof ProcedureDescription>;

/** The procedures of a package. */
export const RouterDescription = defineSchema(
    schema.object({
        /** The name of the package routing them. */
        name: schema.string().min(1),
        /** The procedures. */
        procedures: schema.array(ProcedureDescription),
    }),
);
/** The procedures of a package. */
export type RouterDescription = schema.Infer<typeof RouterDescription>;

/** A route to an object type, with the phases the package's code attaches by method. */
export const ObjectRoute = defineSchema(
    schema.object({
        /** The object type. */
        type: ObjectTypeReference,
        /** The phases the package's code attaches to the type's methods, by method name. */
        hooks: schema.record(schema.string().min(1), schema.array(schema.string().min(1))),
    }),
);
/** A route to an object type, with the phases the package's code attaches by method. */
export type ObjectRoute = schema.Infer<typeof ObjectRoute>;

/** The path below a package's mount an adapter answers, one or more segments such as /npm. */
export const AdapterPath = defineSchema(schema.string().regex(/^(\/[^/]+)+$/u));

/** A finance feature a capability needs the account's plan to include, by its package and name. */
export const FeatureReference = defineSchema(
    schema.object({
        /** The package declaring the feature. */
        packageId: PackageId,
        /** The feature's name in its package. */
        name: schema.string().min(1),
    }),
);
/** A finance feature, by its package and name. */
export type FeatureReference = schema.Infer<typeof FeatureReference>;

/** Something a package's installations can do beyond its own request, and what it needs. */
export const Capability = defineSchema(
    schema
        .object({
            /** Why the package needs it, shown when someone allows it. */
            reason: schema.string().min(1),
            /** Whether an installation may go without it, asking for it once its code needs it. */
            optional: schema.boolean().exactOptional(),
            /** The permissions it needs of the installation's space and of each user. */
            permissions: PermissionRequest.exactOptional(),
            /** The sandbox abilities it unlocks for the package's workloads and views. */
            sandbox: Sandbox.exactOptional(),
            /** The features the account's plan must include for it, as finance sells them. */
            requires: schema.array(FeatureReference).exactOptional(),
        })
        .strict(),
);
/** Something a package's installations can do, and what it needs. */
export type Capability = schema.Infer<typeof Capability>;

/** The kind of the declaration `definePackage` makes, as manifests list it. */
export const PACKAGE_DECLARATION = "package";

/** A declared package: the API its objects derive, its adapters and its views. */
export const PackageDescription = defineSchema(
    schema.object({
        /** The oldest caller release the package serves, every release when absent. */
        since: Version.exactOptional(),
        /** The procedures its objects derive. */
        api: RouterDescription,
        /** The object types it serves with its code's phases, by route name. */
        objects: schema.record(schema.string().min(1), ObjectRoute),
        /** The adapters of protocols others define, by name. */
        adapters: schema.record(
            DeclarationName,
            schema.object({
                /** The path below the package's mount the adapter answers. */
                path: AdapterPath,
            }),
        ),
        /** The names of the views it offers. */
        views: schema.array(DeclarationName),
        /** The permissions its installations always request, by delegator. */
        permissions: PermissionRequest,
        /** What its installations can do beyond its own request, by name. */
        capabilities: schema.record(DeclarationName, Capability),
    }),
);
/** A declared package: its API, objects, adapters, views and requested permissions. */
export type PackageDescription = schema.Infer<typeof PackageDescription>;
