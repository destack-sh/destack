import type * as image from "../_generated/runtime/world/lineage/image.js";
import type { Snapshot } from "../_generated/runtime/world/world/snapshot.js";

import type { World } from "./world.js";

/** One retained Image in a World's lineage. */
export class Image {
    /** World containing this Image. */
    readonly world: World;
    /** Complete retained Image record. */
    readonly record: image.Image;

    /** Bind one retained Image record to its World. */
    constructor(world: World, record: image.Image) {
        this.world = world;
        this.record = record;
    }

    /** Retained Image identity. */
    get id(): image.ImageId {
        return this.record.id;
    }

    /** Committed Moment captured by this Image. */
    get moment() {
        return this.record.moment;
    }

    /** Explicit Image name, when present. */
    get name() {
        return this.record.name;
    }

    /** Image labels. */
    get labels() {
        return this.record.labels;
    }

    /** Rewind this World to the captured Moment. */
    rewind(): Promise<void> {
        return this.world.rewind(this.moment);
    }

    /** Fork one new World from the captured Moment. */
    fork(name: string): Promise<World> {
        return this.world.fork(this.moment, name);
    }

    /** Store this Image as a Blob-backed Snapshot. */
    async snapshot(): Promise<Snapshot> {
        const response = await this.world.client.snapshot({
            worldId: this.world.id,
            imageId: this.id,
        });

        return response.value;
    }
}

export type { ImageId } from "../_generated/runtime/world/lineage/image.js";
export type { Snapshot };
