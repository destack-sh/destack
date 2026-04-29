# Declaration Integration

Declaration integration fixtures cover documentation, decorators, heritage clauses, and member bodies together.

## Classes

### jsdoc decorated exported class with heritage

JSDoc, decorators, exports, heritage clauses, and decorated members keep their relative order.

```ts:main.ts jsdoc=true line-width=80
/**
 * Stores values.
 * @typeParam T value type
 */
@entity
export class Store<T> extends Base<T> implements Reader<T> {
  /**
   * Current value.
   * @returns stored value
   */
  @tracked
  get value(): T { return this.current }
}
```

```ts expected
/**
 * Stores values.
 *
 * @typeParam T Value type
 */
@entity
export class Store<T> extends Base<T> implements Reader<T> {
    /**
     * Current value.
     *
     * @returns Stored value
     */
    @tracked
    get value(): T {
        return this.current;
    }
}
```
