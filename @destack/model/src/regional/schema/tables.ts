import { repository } from "../package/repository.ts";
import { repositoryRef } from "../package/ref.ts";
import { release } from "../package/release.ts";
import { packageTable } from "../package/package.ts";

/** Repository and registry records retained in their assigned region. */
export const tables = { repository, repositoryRef, release, package: packageTable };
