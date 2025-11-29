import { Elysia } from "elysia";

export type DemoMetrics = {
    totalRequests: number;
    guardHits: number;
    wsConnections: number;
    lastProfileId: number;
};

export const metricsState: DemoMetrics = {
    totalRequests: 0,
    guardHits: 0,
    wsConnections: 0,
    lastProfileId: 0
};

export const metricsFeature = new Elysia({ name: "metrics" })
    .state("metrics", metricsState)
    .derive(({ store }) => ({
        metricsSnapshot() {
            return { ...store.metrics };
        },
        noteGuardHit() {
            store.metrics.guardHits += 1;
        }
    }))
    .onTransform(({ store }) => {
        store.metrics.totalRequests += 1;
    });
