/** A ContextVar is a wrapper for a runtime context-specific value. */
export class ContextVar<T> {
  private _defaultValue: T;
  private _stack: T[] = [];

  constructor(defaultValue: T) {
    this._defaultValue = defaultValue;
  }

  /**
   * Get the current value from the context stack.
   */
  get(): T {
    return this._stack.length > 0 ? this._stack[this._stack.length - 1] : this._defaultValue;
  }

  /**
   * Set a new value on the context stack.
   * Returns a token that can be used to reset to the previous value.
   */
  set(value: T): string {
    this._stack.push(value);
    return (this._stack.length - 1).toString();
  }

  /**
   * Reset the context to a previous state.
   * If token is provided, reset to that specific stack position.
   * If no token is provided, clear the entire stack.
   */
  reset(token?: string | null): void {
    if (token != null) {
      const index = parseInt(token, 10);
      if (index >= 0 && index < this._stack.length) {
        this._stack.splice(index + 1);
      }
    } else {
      this._stack = [];
    }
  }
}
