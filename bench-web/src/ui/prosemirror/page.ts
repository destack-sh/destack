import { BlockData, PageData } from "@/proto/wire";
import { PreparedNodeConnection } from "@/system/connection";
import Block from "@/views/nodes/Block.vue";
import { inject, provide, Ref } from "vue";

/**
 * NOTE :Architecture: we can't use provide/inject because the Block views are created manually,
 *  which apparently breaks the inject chain (since parent/children are not connected properly).
 */

export const PAGE_CONTEXT_KEY = Symbol("page");

/** Context for Views on a Page */
export type PageContext = {
  page: Ref<PageData | null | undefined>;
  blocks: Ref<BlockData[]>;
  blocksRefById: Ref<Record<string, InstanceType<typeof Block>>>;
  preparedConnection: PreparedNodeConnection;
  gutterWidth: Ref<number | undefined>;
};

/** Provide the PageContext */
export function providePageContext(context: PageContext) {
  provide(PAGE_CONTEXT_KEY, context);
}

/** Inject the PageContext */
export function usePageContext(): PageContext {
  const context = inject<PageContext>(PAGE_CONTEXT_KEY);
  if (context == null) throw new Error("no page context");
  return context;
}
