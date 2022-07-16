import { ref, watchEffect, type Ref } from "vue";

export type AsyncResult<T> = {
  result: Ref<T | null>;
  error: Ref<any>;
  loading: Ref<boolean>;
};

export function computedAsync<T>(getter: () => Promise<T>): AsyncResult<T> {
  /**
   * Wraps a computed promise to get its result along with error/loading info.
   */
  const result: Ref<T | null> = ref(null);
  const error: Ref<Error | null> = ref(null);
  const loading: Ref<boolean> = ref(false);

  watchEffect(async () => {
    try {
      loading.value = true;
      result.value = await getter();
    } catch (error: any) {
      error.value = error;
    } finally {
      loading.value = false;
    }
  });

  return { result, error, loading };
}
