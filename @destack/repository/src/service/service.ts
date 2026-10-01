import { defineService } from "@destack/service";
import { reference, repository } from "../object/index.ts";

/** Repositories and their references, served as objects; Git itself travels over Git's own protocols. */
export const repositoryService = defineService("repository", {
    objects: { repository, reference },
});
