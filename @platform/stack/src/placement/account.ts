import type { Authenticator } from "@destack/account/better-auth";
import type { AccountConfiguration } from "@destack/account/workload";
import { forgeService } from "@destack/forge/service";
import type { Policy } from "@destack/access";
import type { ObjectType } from "@destack/object";
import type { Service, ServiceRouter } from "@destack/service";
import { ServiceError } from "@destack/service/error";
import { setting } from "@destack/setting/object";
import { PLATFORM_HANDLES } from "../universe/handle.ts";
import { RESIDENCIES } from "../universe/residency.ts";

/** The name of the platform's own organisation. */
const PLATFORM_NAME = "Destack";

/** The jurisdiction the platform's own organisation keeps its data in, Destack's home in the European Union. */
const PLATFORM_RESIDENCY = "eu";

/** The account service as every universe configures it, in the dev process and on workerd. */
export const PlatformAccount = {
    /** Name the sign-in pages below the universe's origin. */
    pages(origin: string) {
        return {
            signInUri: `${origin}/sign-in`,
            consentUri: `${origin}/consent`,
            verificationUri: `${origin}/device`,
            secondFactorUri: `${origin}/sign-in/two-factor`,
            handleUri: `${origin}/sign-in/handle`,
        };
    },

    /** Configure the account service with its sign-in, settings, placed package permissions and operator invitation. */
    configuration(signIn: Authenticator, operator: string): AccountConfiguration {
        return {
            authentication: signIn,
            connections: {
                providers: [],
                vault: {
                    // TODO #Incomplete: reach each space's vault through the cell serving it with RemoteVault
                    write: async () => {
                        throw new ServiceError("NOT_IMPLEMENTED", {
                            message: "the universe runs no vault service",
                        });
                    },
                    read: async () => {
                        throw new ServiceError("NOT_IMPLEMENTED", {
                            message: "the universe runs no vault service",
                        });
                    },
                    destroy: async () => {
                        throw new ServiceError("NOT_IMPLEMENTED", {
                            message: "the universe runs no vault service",
                        });
                    },
                },
            },
            inherited: [setting],
            policies: policiesOf(forgeService),
            residencies: RESIDENCIES,
            platform: {
                name: PLATFORM_NAME,
                residencyId: PLATFORM_RESIDENCY,
                operator,
                handles: PLATFORM_HANDLES,
            },
        };
    },
};

/** Read the policies of the object types a service serves, which accounts' roles grant permissions on. */
function policiesOf(
    service: Service<ServiceRouter, Readonly<Record<string, ObjectType>>>,
): Policy[] {
    return Object.values(service.objects).map((object) => object.policy);
}
