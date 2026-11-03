# @destack/registry

The registry's handle declares repositories, references, packages, releases, tags and a `builds` adapter; a package such as forge implements it, and `Registry` resolves, pushes and opens releases through whichever installation does.

```ts
import registryPackage from "@destack/registry/package";
import { Registry } from "@destack/registry/client";

implementPackage(forgePackage, { protocols: [registryPackage], objects: [...], adapters: { builds } });
const registry = Registry.of(directory, home, sign, environment, "forge", releases);
const manifest = await registry.find(packageId, "latest"); // the digest the tag points at
const build = await registry.open(packageId, "2026.9.0"); // a release's build by version or tag
await registry.push(store, manifest); // upload a stored build to its package's forge
```
