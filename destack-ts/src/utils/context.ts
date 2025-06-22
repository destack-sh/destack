/** A ContextVar is a wrapper for a runtime context-specific value. */
export class ContextVar<T> {
  private _value: T;

  // nocheckin: implement Typescript ContextVar somehow
  constructor(value: T) {
    this._value = value;
  }

  get(): T {
    return this._value;
  }

  set(value: T) {
    this._value = value;
  }
}
