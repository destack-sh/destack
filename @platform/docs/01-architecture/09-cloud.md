---
title: Cloud
description: Cloud.
---

# Cloud

- global control plane
- manages accounts, space locations, routing, and resource limits.
- Execution hosts run spaces independently of their persistent storage.
- gateways route authenticated requests to the selected host
- outbound tunnels provide access to local hosts

- sleep by default, basically like workers / lambdas
- each space selects EU or US residency for all the "data plane" stuff
- execution placement must satisfy the space's processing requirements
- (could be workers if possible, otherwise containers or something heavier)
