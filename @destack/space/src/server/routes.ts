import type { DatabaseConnection } from "@destack/db";
import type { Destination, Endpoint, Routes } from "@destack/host/router";
import type { InstanceSpec } from "@destack/host/runtime";
import type { Identifier } from "@destack/schema";
import { ServiceMount } from "@destack/service";
import { ServiceBindingSpec, ServiceKind } from "@destack/service/declare";
import { ServiceError } from "@destack/service/error";
import { Installation, Instance } from "../object/index.ts";

/** The routes of the spaces a cell serves: its installations' running instances, and the addresses its workloads bind. */
export class SpaceRoutes implements Routes {
    /** The cell's space database. */
    readonly #database: DatabaseConnection;
    /** The services the cell mounts for its workloads, by address, such as `@destack/audit`. */
    readonly #services: ReadonlyMap<string, Extract<Destination, { kind: "service" }>>;
    /** The universe's origin, mounting its services. */
    readonly #universe: string;
    /** Find the space at an address such as `work.acme`, and whether this cell serves it. */
    readonly #locate: (space: string) => Promise<SpaceLocation | undefined>;
    /** Write the service URL of an installation in a space another cell serves, as `notes.work.acme`. */
    readonly #origin: (address: string) => string;

    /** Route a cell's spaces with the services its cell mounts, the universe and the origins of other spaces. */
    constructor(
        database: DatabaseConnection,
        options: {
            /** The services the cell mounts for its workloads, by address. */
            readonly services: ReadonlyMap<string, Extract<Destination, { kind: "service" }>>;
            /** The universe's origin, mounting its services. */
            readonly universe: string;
            /** Find the space at an address, and whether this cell serves it. */
            readonly locate: (space: string) => Promise<SpaceLocation | undefined>;
            /** Write the service URL of an installation in a space another cell serves. */
            readonly origin: (address: string) => string;
        },
    ) {
        // keep the database, the cell's services, the universe, the served spaces and the origins
        this.#database = database;
        this.#services = options.services;
        this.#universe = options.universe;
        this.#locate = options.locate;
        this.#origin = options.origin;
    }

    /** List the running instances serving an installation. */
    endpoints(installationId: Identifier<"installation">): Promise<readonly Endpoint[]> {
        return Instance.endpoints(this.#database, installationId);
    }

    /** Resolve an address an instance's workload calls: a service its cell mounts, or one of its service bindings. */
    async resolve(spec: InstanceSpec, address: string): Promise<Destination> {
        // take a service the cell mounts for every workload
        const mounted = this.#services.get(address);
        if (mounted !== undefined) {
            return mounted;
        }

        // require a service binding with the address, and call the service it declares
        const bound = spec.resources.find(
            (resource) => resource.kind === ServiceKind.name && resource.reference === address,
        );
        if (bound === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `installation ${spec.installationId} binds no ${address}`,
            });
        }
        const audience = ServiceBindingSpec.parse(bound.spec).service.packageId;

        // call a universe service at the universe's mount
        if (address.startsWith("@")) {
            const url = ServiceMount.url(this.#universe, audience);

            return { kind: "remote", scope: spec.scope, url, audience };
        }

        // find the space of the address: the caller's own, or another one
        const [alias, ...labels] = address.split(".");
        const space =
            labels.length === 0
                ? { id: spec.scope, isServed: true }
                : await this.#locate(labels.join("."));
        if (space === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no space answers at ${address}` });
        }

        // call an installation of a space another cell serves at its origin
        if (!space.isServed) {
            return { kind: "remote", scope: space.id, url: this.#origin(address), audience };
        }

        // call an installation of a served space, refusing one of another package
        const found = await Installation.find(this.#database, space.id, alias!);
        if (found === undefined || found.installation.packageId !== audience) {
            throw new ServiceError("NOT_FOUND", {
                message: `no installation ${address} of ${audience} is served here`,
            });
        }

        return {
            kind: "installation",
            scope: space.id,
            installationId: found.installation.id,
            audience,
        };
    }
}

/** A space an address resolves to, and whether the cell resolving it serves it. */
export interface SpaceLocation {
    /** The space. */
    readonly id: Identifier<"space">;
    /** Whether this cell serves the space. */
    readonly isServed: boolean;
}
