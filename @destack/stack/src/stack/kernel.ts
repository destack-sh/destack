import { bucket } from "../object/bucket.ts";
import { database } from "../object/database.ts";
import { dependency } from "../object/dependency.ts";
import { gitStorage } from "../object/git.ts";
import { https } from "../object/https.ts";
import { mail } from "../object/mail.ts";
import registryPackage from "@destack/registry/package";

/** The resource types the stack provisions in a space in addition to its package's objects. */
export const PROVISIONED = { database, bucket, dependency, https, mail, git: gitStorage };

/** The types the kernel declares beside its packages', which packages implement or provision. */
export const KERNEL_TYPES = [
    ...Object.values(PROVISIONED),
    ...Object.values(registryPackage.objects),
];
