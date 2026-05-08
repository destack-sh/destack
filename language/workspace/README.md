# workspace

The workspace crate owns repository state and project configuration for the language toolchain.
It is the boundary between source truth and derived toolchain work.

## Repository

`Repository` owns immutable source revisions, file contents, workspace discovery, package discovery, module discovery, host inputs, and published artifact versions.
It does not own compiler, linter, query, or provider execution state.

A `Revision` is a stable handle for one immutable snapshot of repository inputs.
Consumers use a revision to ask repository questions or to request artifacts through the session layer.

## Configuration

The configuration model describes toolchain inputs loaded from `destack.json`.
The effective model is expressed as typed options for compiler, runtime, formatter, linter, packages, workspaces, and targets.

`destack.json` is the native manifest format.
External project metadata should be translated before it reaches this crate.

## Boundaries

Workspace types should be stable facts about a repository or project configuration.
Reusable compiler, linter, query, and editor products should be artifacts or provider-owned caches outside this crate.
