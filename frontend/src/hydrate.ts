import { useAppStore, useFlowsStore, useMetaStore, useNotificationsStore } from "./stores";
import { useArtifactsStore } from "./stores/artifacts";

type BaseStore = {
  $id: string;
  hydrate?: () => Promise<void>;
  dehydrate?: () => Promise<void>;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
};

export function useStores(
  stores = [useAppStore, useArtifactsStore, useFlowsStore, useMetaStore, useNotificationsStore]
): BaseStore[] {
  return stores.map((useStore) => useStore()) as BaseStore[];
}

export async function hydrate() {
  const stores = useStores();
  const appStore = useAppStore();

  appStore.hydrating = true;

  try {
    const hydratedStores = [] as string[];
    await Promise.all(stores.filter(({ $id }) => !hydratedStores.includes($id)).map((store) => store.hydrate?.()));
  } catch (e) {
    console.log(e);
    appStore.error = e as Error;
  } finally {
    appStore.hydrating = false;
  }

  appStore.hydrated = true;
}

export async function dehydrate(stores = useStores()) {
  const appStore = useAppStore();
  if (!appStore.hydrated) {
    return;
  }

  for (const store of stores) {
    await store.dehydrate?.();
  }

  appStore.hydrated = false;
}
