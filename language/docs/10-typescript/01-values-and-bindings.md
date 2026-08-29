---
title: Values and Bindings
description: Constants, variables, values, and scope.
---

# Values and Bindings

`const` fixes a binding. `let` permits assignment. Both are block scoped.

```ds:src/bindings.ds
const service = "relay";
let deliveries = 0;

deliveries += 1;

if (deliveries > 0) {
    const service = "worker";
    console.log(`${service}: ${deliveries}`);
}

console.log(service);
```
