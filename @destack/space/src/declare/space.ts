import { ResourceName } from "@destack/resource";
import { defineSchema, schema } from "@destack/schema";
import { SpaceError } from "../error/index.ts";
import { SpaceInstallation } from "./installation.ts";
import { SpaceResource, SpaceSecret } from "./resource.ts";
import { SpaceRole, SpaceRoleBinding } from "./permission.ts";
import { SpaceRoute } from "./route.ts";
import { SpacePolicies } from "../policy/index.ts";

/** Source-managed space objects, keyed independently of display names and provider identifiers. */
export const SpaceDefinition = defineSchema(
    schema.object({
        /** Source-managed package and network policies. */
        policies: SpacePolicies.optional(),
        /** Resources created or adopted by the configuration. */
        resources: schema.record(ResourceName, SpaceResource).optional(),
        /** Secret metadata; secret values never occur in this definition. */
        secrets: schema.record(ResourceName, SpaceSecret).optional(),
        /** Independently configured package installations. */
        installations: schema.record(ResourceName, SpaceInstallation).optional(),
        /** Space-scoped role definitions. */
        roles: schema.record(ResourceName, SpaceRole).optional(),
        /** Grants evaluated using existing authority before application. */
        bindings: schema.record(ResourceName, SpaceRoleBinding).optional(),
        /** Routes under domains administered by the account. */
        routes: schema.record(ResourceName, SpaceRoute).optional(),
    }),
);
/** Desired space contents produced from a package export. */
export type SpaceDefinition = schema.Infer<typeof SpaceDefinition>;

/** Define a configuration and check references without contacting hosts or provisioning resources. */
export function defineSpace(value: schema.Input<typeof SpaceDefinition>): SpaceDefinition {
    const configuration = SpaceDefinition.parse(value);

    // omitted collections declare no managed objects
    const {
        resources = {},
        secrets: secretDefinitions = {},
        installations = {},
        roles = {},
        bindings = {},
        routes: routeDefinitions = {},
    } = configuration;

    // prevent two definitions from adopting the same durable resource
    const adopted = new Set<string>();
    for (const resource of Object.values(resources)) {
        if (resource.adopt) {
            const key = `${resource.adopt.space}/${resource.adopt.resource}`;
            if (adopted.has(key)) {
                throw new SpaceError("INVALID_DEFINITION", `Duplicate resource adoption: ${key}`);
            }
            adopted.add(key);
        }
    }

    // require each declared secret to select a vault in this configuration
    const secrets = new Set<string>();
    for (const secret of Object.values(secretDefinitions)) {
        if (resources[secret.vault]?.declaration.kind !== "vault") {
            throw new SpaceError("INVALID_DEFINITION", `Unknown vault: ${secret.vault}`);
        }
        const key = `${secret.vault}/${secret.name}`;
        if (secrets.has(key)) {
            throw new SpaceError("INVALID_DEFINITION", `Duplicate secret: ${key}`);
        }
        secrets.add(key);
    }

    // require unique aliases and existing local binding targets
    const aliases = new Set<string>();
    for (const installation of Object.values(installations)) {
        if (aliases.has(installation.alias)) {
            throw new SpaceError("INVALID_DEFINITION", `Duplicate alias: ${installation.alias}`);
        }
        aliases.add(installation.alias);
        for (const binding of Object.values(installation.resources).flatMap(Object.values)) {
            if ("resource" in binding && !Object.hasOwn(resources, binding.resource)) {
                throw new SpaceError("INVALID_DEFINITION", `Unknown resource: ${binding.resource}`);
            }
        }
        for (const binding of Object.values(installation.secrets).flatMap(Object.values)) {
            if (
                !("space" in binding.target) &&
                !Object.hasOwn(secretDefinitions, binding.target.secret)
            ) {
                throw new SpaceError(
                    "INVALID_DEFINITION",
                    `Unknown secret: ${binding.target.secret}`,
                );
            }
        }
    }

    // resolve role targets and workload installation references within the configuration
    const collections = {
        resource: resources,
        secret: secretDefinitions,
        installation: installations,
    };
    for (const role of Object.values(roles)) {
        for (const permission of role.permissions) {
            const target = permission.target;
            if (
                target &&
                "kind" in target &&
                !Object.hasOwn(collections[target.kind], target.name)
            ) {
                throw new SpaceError(
                    "INVALID_DEFINITION",
                    `Unknown ${target.kind}: ${target.name}`,
                );
            }
        }
    }
    for (const binding of Object.values(bindings)) {
        if (typeof binding.role === "string" && !Object.hasOwn(roles, binding.role)) {
            throw new SpaceError("INVALID_DEFINITION", `Unknown role: ${binding.role}`);
        }
        if (
            "installation" in binding.subject &&
            typeof binding.subject.installation === "string" &&
            !Object.hasOwn(installations, binding.subject.installation)
        ) {
            throw new SpaceError(
                "INVALID_DEFINITION",
                `Unknown installation: ${binding.subject.installation}`,
            );
        }
    }

    // reject overlapping definitions of the same route and missing installations
    const routes = new Set<string>();
    for (const route of Object.values(routeDefinitions)) {
        const key = `${route.domain}:${route.match}:${route.path}`;
        if (routes.has(key)) {
            throw new SpaceError("INVALID_DEFINITION", `Duplicate route: ${key}`);
        }
        routes.add(key);
        if (
            "installation" in route.destination &&
            typeof route.destination.installation === "string" &&
            !Object.hasOwn(installations, route.destination.installation)
        ) {
            throw new SpaceError(
                "INVALID_DEFINITION",
                `Unknown route installation: ${route.destination.installation}`,
            );
        }
    }

    return configuration;
}
