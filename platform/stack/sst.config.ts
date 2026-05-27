/// <reference path="./.sst/platform/config.d.ts" />

export default $config({
    async app(input) {
        const { awsRegion, isProductionStage, removalPolicy } = await import("./src/stage");

        return {
            name: "destack-platform",
            home: "aws",
            providers: {
                aws: {
                    region: awsRegion,
                },
                cloudflare: "6.15.0",
            },
            removal: removalPolicy(input.stage),
            protect: isProductionStage(input.stage),
        };
    },
    async run() {
        const { site } = await import("./src/site");

        return site($app.stage);
    },
});
