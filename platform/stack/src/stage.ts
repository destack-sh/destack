export const awsRegion = process.env.AWS_REGION ?? "eu-central-2";

const productionStage = "production";

export function isProductionStage(stage: string) {
    return stage === productionStage;
}

export function removalPolicy(stage: string) {
    if (isProductionStage(stage)) {
        return "retain";
    }

    return "remove";
}
