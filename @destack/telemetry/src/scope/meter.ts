import {
    type Attributes,
    type Counter,
    type Gauge,
    type Histogram,
    type Meter,
    type MeterProvider,
    type MetricOptions,
    metrics,
} from "@opentelemetry/api";

/** The instruments a package creates on its meter: the synchronous ones. */
export type PackageMeter = Pick<Meter, "createCounter" | "createHistogram" | "createGauge">;

/** A meter whose instruments record on the global meter provider current at each measurement, as OpenTelemetry's proxy tracer does for spans. */
export class ProxyMeter implements PackageMeter {
    /** The instrumenting package's name. */
    readonly #name: string;
    /** The instrumenting package's version. */
    readonly #version: string | undefined;

    /** Create a meter for a package before or after a meter provider registers. */
    constructor(name: string, version?: string) {
        this.#name = name;
        this.#version = version;
    }

    /** Create a counter of increments. */
    createCounter<Labels extends Attributes = Attributes>(
        name: string,
        options?: MetricOptions,
    ): Counter<Labels> {
        const counter = this.#resolver((meter) => meter.createCounter<Labels>(name, options));

        return { add: (...measurement) => counter().add(...measurement) };
    }

    /** Create a histogram of measurements. */
    createHistogram<Labels extends Attributes = Attributes>(
        name: string,
        options?: MetricOptions,
    ): Histogram<Labels> {
        const histogram = this.#resolver((meter) => meter.createHistogram<Labels>(name, options));

        return { record: (...measurement) => histogram().record(...measurement) };
    }

    /** Create a gauge of current values. */
    createGauge<Labels extends Attributes = Attributes>(
        name: string,
        options?: MetricOptions,
    ): Gauge<Labels> {
        const gauge = this.#resolver((meter) => meter.createGauge<Labels>(name, options));

        return { record: (...measurement) => gauge().record(...measurement) };
    }

    /** Resolve an instrument on the current provider, creating it again once another provider registers. */
    #resolver<Instrument>(create: (meter: Meter) => Instrument): () => Instrument {
        let provider: MeterProvider | undefined;
        let instrument: Instrument | undefined;

        return () => {
            // create the instrument on a newly registered provider
            const current = metrics.getMeterProvider();
            if (instrument === undefined || current !== provider) {
                provider = current;
                instrument = create(current.getMeter(this.#name, this.#version));
            }

            return instrument;
        };
    }
}
