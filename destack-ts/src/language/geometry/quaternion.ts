import type { PackedCache, Session } from "@destack/language/core";
import { StructType } from "@destack/language/core";
import { Vector4 } from "@destack/language/geometry/vector";
import { registerStructClass } from "@destack/language/registry";
import { hashFloat } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:2400010 ==== */
/**
 * A quaternion.
 */
export class Quaternion extends Vector4 {
  static metatype: StructType = StructType.QUATERNION;
  static __isFrozen__: boolean = true;

  constructor(options: {
    x: number;
    y: number;
    z: number;
    w: number;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(options);

    /* properties */

    /* identity */
    /* ... (already set in parent) */
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.x === other.x || Math.abs(this.x - other.x) < 1e-10)) {
      return false;
    }
    if (!(this.y === other.y || Math.abs(this.y - other.y) < 1e-10)) {
      return false;
    }
    if (!(this.z === other.z || Math.abs(this.z - other.z) < 1e-10)) {
      return false;
    }
    if (!(this.w === other.w || Math.abs(this.w - other.w) < 1e-10)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      propertyReprs.push(`w=${this.w}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Quaternion ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.w)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.QUATERNION, Quaternion);
/* ==== DESTACK_GENERATED_END:STRUCT:2400010 ==== */
