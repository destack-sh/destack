interface Producer<out T> {
    get(): T;
}

interface Consumer<in T> {
    put(value: T): void;
}

type Mapper<in T, out U> = (value: T) => U;
