import { Message } from "@destack/locale";
import { ReasonDescription } from "@destack/error";
import { schema, Version } from "@destack/schema";
import {
    GrantDeclaration,
    PermissionReference,
    PermissionScope,
    type PermissionSet,
} from "@destack/access";
import { defineKind, DeclarationName, type graph, PackageId, Sandbox } from "@destack/package";
import { SwatchName } from "@destack/color";
import type { BuildReader } from "@destack/package/manifest";
import { ObjectTypeReference } from "@destack/sync";

/** A JSON Schema. */
const JsonSchema = schema.jsonObject();

/** A procedure's value or event-stream schema. */
export const PayloadDescription = schema.union([
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
]);

/** A procedure's address, payloads and errors. */
export const ProcedureDescription = schema.object({
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
});
/** A procedure's address, payloads and errors. */
export type ProcedureDescription = schema.Infer<typeof ProcedureDescription>;

/** A route to an object type, with the phases the package's code attaches by method. */
export const ObjectRoute = schema.object({
    /** The object type. */
    type: ObjectTypeReference,
    /** The phases the package's code attaches to the type's methods, by method name. */
    hooks: schema.record(schema.string().min(1), schema.array(schema.string().min(1))),
});
/** A route to an object type, with the phases the package's code attaches by method. */
export type ObjectRoute = schema.Infer<typeof ObjectRoute>;

/** The path below a package's path a mount answers, one or more segments such as /npm. */
export const MountPath = schema.string().regex(/^(\/[^/]+)+$/u);

/** A finance feature a capability needs the account's plan to include, by its package and name. */
export const FeatureReference = schema.object({
    /** The package declaring the feature. */
    packageId: PackageId,
    /** The feature's name in its package. */
    name: schema.string().min(1),
});
/** A finance feature, by its package and name. */
export type FeatureReference = schema.Infer<typeof FeatureReference>;

/** Something a package's installations can do beyond its request, and what it needs. */
export const Capability = schema
    .object({
        /** Why the package needs it, shown when someone allows it. */
        reason: Message,
        /** Whether an installation may go without it, asking for it once its code needs it. */
        optional: schema.boolean().exactOptional(),
        /** The permissions it needs of the installation's space and of each user. */
        permissions: GrantDeclaration.exactOptional(),
        /** The sandbox abilities it unlocks for the package's workloads and views. */
        sandbox: Sandbox.exactOptional(),
        /** The features the account's plan must include for it, as finance sells them. */
        features: schema.array(FeatureReference).exactOptional(),
        /** The packages the space must have installed for it, which its code imports. */
        packages: schema.array(PackageId).exactOptional(),
    })
    .strict();
/** Something a package's installations can do, and what it needs. */
export type Capability = schema.Infer<typeof Capability>;

/** The export a build's package declaration comes from: its module's default export. */
export const PACKAGE_EXPORT = "default";

/** The subpath a package's build exports its server under. */
export const SERVER_EXPORT = "./server";

/** A Phosphor icon's name, in kebab case such as `calendar-blank`. */
export const IconName = schema.string().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/u);

/** A declared package: its identity, the API its objects derive, its mounts and its views. */
const packageDescription = schema.object({
    /** The package's name as people read it. */
    title: Message,
    /** What the package does, in one sentence. */
    description: Message,
    /** The Phosphor icon its app's tile draws. */
    icon: IconName,
    /** The swatch the stack draws it in wherever it appears. */
    color: SwatchName,
    /** The oldest caller release the package serves, every release when absent. */
    since: Version.exactOptional(),
    /** The object types it serves with its code's phases, by route name. */
    objects: schema.record(schema.string().min(1), ObjectRoute),
    /** The packages whose protocols it implements, whose tables its installation owns. */
    protocols: schema.array(PackageId),
    /** The mounts of protocols others define, by name. */
    mounts: schema.record(
        DeclarationName,
        schema.object({
            /** The path below the package's path the mount answers. */
            path: MountPath,
        }),
    ),
    /** The names of the views it offers. */
    views: schema.array(DeclarationName),
    /** The permissions its installations always request, by delegator. */
    permissions: GrantDeclaration,
    /** What its installations can do beyond its request, by name. */
    capabilities: schema.record(DeclarationName, Capability),
    /** The reasons it fails with, by domain, reason and call error code. */
    errors: schema.array(ReasonDescription),
});

/** The kind of the declaration `definePackage` makes, which object defines. */
export const PACKAGE_KIND = defineKind({ name: "package", description: packageDescription });

/** A declared package: the API its objects derive, its mounts and its views. */
export const PackageDescription = Object.assign(packageDescription, {
    /** Read the package a build declares, absent for a library. */
    async of(build: BuildReader): Promise<schema.Infer<typeof packageDescription> | undefined> {
        const [declared] = await PACKAGE_KIND.read(build);

        return declared?.description;
    },

    /** Read the package some declarations declare, absent for a library. */
    find(
        declarations: readonly Pick<graph.Declaration, "kind" | "kindPackageId" | "description">[],
    ): schema.Infer<typeof packageDescription> | undefined {
        const declared = declarations.find((each) => PACKAGE_KIND.has(each));

        return declared === undefined ? undefined : packageDescription.parse(declared.description);
    },

    /** List the packages only a package's optional capabilities need installed. */
    optionalPackages(
        description: schema.Infer<typeof packageDescription> | undefined,
    ): PackageId[] {
        return Object.values(description?.capabilities ?? {}).flatMap((capability) =>
            capability.optional === true ? (capability.packages ?? []) : [],
        );
    },

    /** List the permissions a package asks of a delegator with some capabilities enabled. */
    asks(
        description: Pick<schema.Infer<typeof packageDescription>, "permissions" | "capabilities">,
        enabled: readonly string[],
        delegator: keyof GrantDeclaration,
    ): PermissionSet {
        // gather the package's set and each enabled capability's
        const sets: PermissionSet[] = [
            description.permissions[delegator] ?? {},
            ...Object.entries(description.capabilities)
                .filter(([name]) => enabled.includes(name))
                .map(([, capability]) => capability.permissions?.[delegator] ?? {}),
        ];

        // merge them scope by scope, each permission once
        return Object.fromEntries(
            PermissionScope.options.flatMap((scope) => {
                const merged = sets.flatMap((set) => set[scope] ?? []);
                const once = merged.filter(
                    (permission, index) =>
                        merged.findIndex(
                            (other) =>
                                PermissionReference.key(other) ===
                                PermissionReference.key(permission),
                        ) === index,
                );

                return once.length === 0 ? [] : [[scope, once]];
            }),
        );
    },
});
/** A declared package: its API, objects, mounts, views and requested permissions. */
export type PackageDescription = schema.Infer<typeof PackageDescription>;
