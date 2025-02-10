import { BlockData, PageData } from "@/proto/wire";
import { PreparedGetConnection } from "@/system/connection";
import Block from "@/views/nodes/Block.vue";
import { inject, Ref } from "vue";

/**
 * NOTE :Architecture: we can't use provide/inject because the Block views are created manually,
 *  which apparently breaks the inject chain (since parent/children are not connected properly).
 */

/** Context for Views on a Page */
export type PageContext = {
  page: Ref<PageData | null | undefined>;
  blocks: Ref<BlockData[]>;
  blocksRefById: Ref<Record<string, InstanceType<typeof Block>>>;
  preparedConnection: PreparedGetConnection;
  gutterWidth: Ref<number | undefined>;
};

const activePageContextsByKey: Record<string, PageContext> = {};

export function providePageContext(key: string, context: PageContext) {
  activePageContextsByKey[key] = context;
}

/** Inject the PageContext */
export function usePageContext(key: string): PageContext {
  const page = activePageContextsByKey[key];
  if (page == null) throw new Error("no page context");
  return page;
}
