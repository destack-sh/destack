<script lang="ts" setup>
import { canvas } from "@/globals";
import { makeType } from "@/language/core/type";
import { newChangeId } from "@/language/runtime/transaction";
import { createInlineNode } from "@/language/source/block";
import {
  BenchType,
  BlockType,
  ColorShade,
  NodeReferenceData,
  NodeType,
  Orientation,
  TextLineType,
  TextSpanType,
  TypeKind,
} from "@/proto/wire";
import { isNode, isNodeRef, toNodeRef } from "@/proto/wiring";
import { IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { autoWrap, findAncestor } from "@/ui/prosemirror/editor";
import { PageContext } from "@/ui/prosemirror/page";
import { getPmLineType, PM_SCHEMA, SpanSpecialInputType } from "@/ui/prosemirror/schema";
import { SearchItem, useValueSearch } from "@/ui/search";
import { getColorHex } from "@/ui/style";
import { useFloating } from "@/utils/floating";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import { FocusAnchor, ViewEmits, ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { Node as PmNode } from "prosemirror-model";
import { TextSelection } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { computed, nextTick, Ref, ref, watch } from "vue";

const MAX_HEIGHT = 500;
const WIDTH = 320;

const PLACEHOLDER_BY_TYPE: Record<SpanSpecialInputType, string> = {
  ["/"]: "Filter...",
  ["@"]: "Mention...",
};

const props = defineProps<{
  type: SpanSpecialInputType;
  node: PmNode;
  view: EditorView;
  getPos: () => number;
  pageContext?: PageContext;
}>();
const emit = defineEmits<ViewEmits>();

const query = ref("");
const inputRef = ref<HTMLSpanElement>();
const inputContainerRef = ref<HTMLDivElement>();
const popoverRef = ref<HTMLDivElement>();
let isOpen = true;

/** Convert (type+) query into a regular span. */
function closeSelf() {
  if (!isOpen) return;
  const pos = props.getPos();
  const { state, dispatch } = props.view;
  const { schema } = state;
  const newText = props.type + query.value;
  const textNode = schema.text(newText);
  let tr = state.tr.replaceWith(pos, pos + props.node.nodeSize, textNode);
  tr = tr.setSelection(TextSelection.create(tr.doc, pos + newText.length));
  dispatch(tr);
  props.view.focus();
  isOpen = false;
}

/** Delete this temporary span. */
function deleteSelf() {
  if (!isOpen) return;
  const pos = props.getPos();
  const { state, dispatch } = props.view;
  let tr = state.tr.delete(pos, pos + props.node.nodeSize);
  tr = tr.setSelection(TextSelection.create(tr.doc, pos));
  dispatch(tr);
  props.view.focus();
  isOpen = false;
}

//
// Search
//

const valueType = computed(() => {
  if (props.type == "/") {
    if (props.pageContext == null) {
      return makeType({ kind: TypeKind.ENUM, benchType: BenchType.TEXT_LINE_TYPE });
    } else {
      return makeType({ kind: TypeKind.ENUM, benchType: BenchType.BLOCK_TYPE });
    }
  } else if (props.type == "@") {
    return makeType({ kind: TypeKind.ENUM, benchType: BenchType.FLOW });
  } else {
    return null;
  }
});
const { index, candidates, results, resultsTotal, isLoading, update, getItemFromValue, getValueFromItem } =
  useValueSearch({ query, valueType });
const resultsRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const activeResultId: Ref<string | null> = ref(null);

function updateQuery(newQuery: string) {
  query.value = newQuery;

  // autoclose if it looks like the search is no longer needed
  if (
    newQuery == " " ||
    newQuery.includes("/") ||
    newQuery.includes("@") ||
    (newQuery.endsWith(" ") && results.value.length == 0)
  ) {
    closeSelf();
  }
}

// auto-select first result if there is no valid active result
watch(
  [activeResultId, results],
  () => {
    if (activeResultId.value == null || !results.value.find((item) => item.id == activeResultId.value)) {
      activeResultId.value = results.value[0]?.id;
    }
  },
  { immediate: true },
);

/** Move the selection up or down. */
function select(direction: "up" | "down") {
  const currentIndex = results.value.findIndex((item) => item.id == activeResultId.value);
  if (direction == "up" && currentIndex > 0) {
    activeResultId.value = results.value[currentIndex - 1]?.id;
  } else if (direction == "down" && currentIndex < results.value.length - 1) {
    activeResultId.value = results.value[currentIndex + 1]?.id;
  }
  if (activeResultId.value == null) {
    activeResultId.value = results.value[0]?.id;
  }
}

/** Apply the selected item. */
function apply(item: SearchItem) {
  const view = props.view;
  const { state, dispatch } = view;
  const pos = props.getPos();
  const { node: lineNode, pos: linePos } = findAncestor(state, state.doc.resolve(pos), (node) =>
    node.type.isInGroup("line"),
  );
  if (lineNode == null || linePos == null) {
    throw new Error("no containing line node found");
  }

  if (props.type == "/") {
    const type = getValueFromItem(item) as number;
    const textLineType = props.pageContext == null ? (type as TextLineType) : ((type - 10_000) as TextLineType);
    const blockType = props.pageContext == null ? ((type + 10_000) as BlockType) : (type as BlockType);

    if (props.pageContext == null || blockType >= BlockType.PARAGRAPH) {
      // morph text line directly
      const tr = view.state.tr;
      if (blockType == BlockType.DIVIDER) {
        tr.insert(linePos - 1, PM_SCHEMA.node("lineDivider"));
        tr.delete(linePos + 1, linePos + lineNode.nodeSize - 1);
        tr.setSelection(TextSelection.near(tr.doc.resolve(linePos)));
      } else {
        tr.setBlockType(linePos, linePos + lineNode.nodeSize - 2, getPmLineType(textLineType), {
          ...lineNode.attrs,
          type: textLineType,
        });
        tr.delete(linePos, linePos + lineNode.nodeSize - 2);
        tr.setSelection(TextSelection.near(tr.doc.resolve(linePos)));
        autoWrap(tr);
      }
      dispatch(tr);
    } else {
      // morph block with node
      const page = props.pageContext?.page.value;
      if (page == null) throw new Error("no page context");
      const { connection, graph } = props.pageContext.preparedConnection;
      const tx = connection.tx.with({ change: { title: "Morph", key: newChangeId() } });
      const block = graph.get(lineNode.attrs.blockPtr);
      if (!isNode(block, NodeType.BLOCK)) throw new Error("block not found");
      // create inline node
      const node = createInlineNode(tx, graph, {
        node: {
          metatype: blockType as unknown as NodeType,
          parentPtr: toNodeRef(page),
          packagePtr: page.packagePtr,
          blockPtr: toNodeRef(block),
        },
      });
      tx.update(block, { type: blockType, nodePtr: toNodeRef(node) });
      deleteSelf();
      if (blockType == BlockType.PAGE) {
        nextTick(() => canvas.goToNode(node)); // go to the new page
      } else {
        // NOTE :Cleanup: focus new blocks in Text properly, using the component ref is a bit hacky
        //  (ideally we would do this through the prosemirror transaction system, but we're also updating our own graph,
        //   and I'm not quite sure how to synchronize/connect the two properly)
        nextTick(() => {
          const blockRef = props.pageContext?.blocksRefById.value?.[block.id];
          blockRef?.focus?.("top");
        });
      }
    }
  } else if (props.type == "@") {
    // create mention
    const nodePtr = getValueFromItem(item) as NodeReferenceData;
    if (!isNodeRef(nodePtr)) throw new Error("nodePtr is not a node ref");
    const tr = view.state.tr;
    tr.delete(pos, pos + props.node.nodeSize);
    tr.insert(pos, PM_SCHEMA.node("spanMention", { type: TextSpanType.MENTION, nodePtr }));
    tr.insertText(" ", pos + 1, pos + 1);
    view.dispatch(tr);
  }

  view.focus();
  isOpen = false;
}

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  inputRef.value?.focus?.();
}

useFloating({
  floating: popoverRef,
  reference: inputContainerRef,
  options: {
    placement: "bottom-left",
    offset: "referenceWidth",
    referenceMargin: 4,
    containerMargin: 20,
  },
  isEnabled: computed(() => popoverRef.value != null),
  keepPlacement: true,
  watchElements: true,
});

defineExpose<Partial<ViewExpose>>({ focus });
</script>
<template>
  <div class="relative inline rounded bg-gray-100 px-0.5 py-1.5" @focusout="$nextTick(closeSelf)">
    <!-- Input container -->
    <div ref="inputContainerRef" class="inline">
      <!-- Type -->
      <span>{{ type }}</span>
      <!-- Input -->
      <span
        ref="inputRef"
        class="inline-block outline-none"
        contenteditable="true"
        @keydown.escape.stop.prevent="closeSelf"
        @keydown.up.stop.prevent="select('up')"
        @keydown.down.stop.prevent="select('down')"
        @keydown.enter.stop.prevent="
          () => {
            const activeItem = results.find((item) => item.id == activeResultId);
            if (activeItem != null) {
              apply(activeItem);
            } else {
              closeSelf();
            }
          }
        "
        @keydown.backspace="
          (e) => {
            if (query.length == 0) {
              deleteSelf();
              e.preventDefault();
              e.stopPropagation();
            }
          }
        "
        @input="
          () => {
            updateQuery(inputRef?.innerText ?? '');
          }
        "
      />
      <!-- Placeholder -->
      <span v-if="query.length == 0" class="text-gray-400">{{ PLACEHOLDER_BY_TYPE[type] }}</span>
    </div>

    <!-- Popover -->
    <Transition
      enter-active-class="transition-all ease-in duration-75"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-all ease-out duration-75"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
      appear
    >
      <div
        v-if="results.length > 0"
        ref="popoverRef"
        class="absolute z-70 rounded border border-gray-200 bg-white text-sm shadow-sm shadow-gray-300"
        :style="{
          width: `${WIDTH}px`,
        }"
      >
        <Scroll
          id="body"
          size-is-dynamic
          :size="{ width: WIDTH, height: MAX_HEIGHT }"
          :orientation="Orientation.VERTICAL"
          :track-width="ScrollbarWidth.sm"
          track-is-overlay
        >
          <!-- Results -->
          <ul class="mx-0.5 flex max-w-full flex-col py-0.5">
            <template v-for="item in results" :key="item.id">
              <!-- Results -->
              <li
                :ref="(ref?: any) => (ref != null ? (resultsRefs[item.id] = ref) : delete resultsRefs[item.id])"
                role="menuitem"
                class="flex max-w-full cursor-pointer flex-row items-center rounded px-2 py-1 hover:bg-gray-100"
                :class="[activeResultId == item.id ? 'bg-gray-100' : '']"
                :data-active="activeResultId == item.id"
                @click.prevent="apply(item)"
              >
                <!-- Main content -->
                <IconInline
                  v-if="(item as any).icon"
                  v-bind="(item as any).icon"
                  class="mr-1.5 w-6 flex-shrink-0 rounded-md py-1 text-gray-700"
                  :style="{
                    backgroundColor: item.color != null ? getColorHex(item.color, ColorShade.S300) : undefined,
                  }"
                />
                <span v-else class="mr-1.5 w-6 flex-shrink-0 text-gray-700" />
                <span class="max-w-full select-none truncate" v-html="item.titleMarked ?? item.title" />
                <!-- Metadata -->
                <NodeMetadata v-if="item.metatype == 'node'" size="regular" :node="item.node!" class="ml-1.5" />
                <!-- Secondary -->
                <span class="ml-auto truncate pl-2 text-right">
                  <!-- Path -->
                  <span
                    v-if="valueType?.kind != TypeKind.BASED_NODE && 'path' in item"
                    class="truncate pl-2 text-gray-400"
                  >
                    <span v-html="item.pathMarked ?? item.path" />
                  </span>
                  <span
                    v-else-if="item.alias"
                    class="ml-auto truncate pl-2 text-right text-gray-400"
                    v-html="item.aliasMarked ?? item.alias"
                  />
                  <span v-else-if="item.text" class="truncate pl-2 text-right text-gray-400">
                    {{ item.text }}
                  </span>
                </span>
              </li>
            </template>
          </ul>
        </Scroll>
      </div>
    </Transition>
  </div>
</template>
