import { principal } from "@destack/access";
import { Scope, Subject } from "@destack/sync";
import { Package } from "@destack/package";

/** The principals the scenarios act as, users with identifiers of their own. */
export const actors = {
    alice: principal.user.reference(Scope.universe.id, "user-019f5530-8000-7000-8000-0000000000a1"),
    bob: principal.user.reference(Scope.universe.id, "user-019f5530-8000-7000-8000-0000000000b2"),
    carol: principal.user.reference(Scope.universe.id, "user-019f5530-8000-7000-8000-0000000000c3"),
    dave: principal.user.reference(Scope.universe.id, "user-019f5530-8000-7000-8000-0000000000d4"),
    erin: principal.user.reference(Scope.universe.id, "user-019f5530-8000-7000-8000-0000000000e5"),
};

/** A principal the scenarios act as. */
export type Actor = keyof typeof actors;

/** Read the name of the actor a subject is. */
export function nameOf(subject: Subject): Actor {
    return (Object.keys(actors) as Actor[]).find((actor) => Subject.same(actors[actor], subject))!;
}

/** The account package release with the person's settings. */
export const accounts = Package.parse({
    id: "package-019f5530-8000-7000-8000-0000000000ac",
    name: "@destack/account",
    version: "2026.9.0",
});
