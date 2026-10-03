import { defineService } from "@destack/service";
import { reference, repository } from "../object/index.ts";

/** Repositories and their references, served as objects, with Git over its smart HTTP protocol. */
export const repositoryService = defineService("repository", {
    objects: { repository, reference },
});
