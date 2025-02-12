<script lang="ts" setup>
import { NodeReferenceData } from "@/proto/wire";
import { SpanSpecialInputType } from "@/ui/prosemirror/schema";
import { useFloating } from "@/utils/floating";
import { FocusAnchor, ViewEmits, ViewExpose } from "@/views/common";
import { Node as PmNode } from "prosemirror-model";
import { TextSelection } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { ref } from "vue";

const PLACEHOLDER_BY_TYPE: Record<SpanSpecialInputType, string> = {
  ["/"]: "Filter...",
  ["@"]: "Mention...",
};

const props = defineProps<{
  type: SpanSpecialInputType;
  node: PmNode;
  view: EditorView;
  getPos: () => number;
}>();
const emit = defineEmits<ViewEmits>();

const query = ref("");
const inputRef = ref<HTMLSpanElement>();
const inputContainerRef = ref<HTMLDivElement>();
const popoverRef = ref<HTMLDivElement>();

function updateQuery(newQuery: string) {
  query.value = newQuery;
  if (newQuery == " " || newQuery.includes("/") || newQuery.includes("@")) {
    closeSelf();
  }
}

/** Convert (type+) query into a regular span. */
function closeSelf() {
  const pos = props.getPos();
  const { state, dispatch } = props.view;
  const { schema } = state;
  const newText = props.type + query.value;
  const textNode = schema.text(newText);
  let tr = state.tr.replaceWith(pos, pos + props.node.nodeSize, textNode);
  tr = tr.setSelection(TextSelection.create(tr.doc, pos + newText.length));
  dispatch(tr);
  props.view.focus();
}

/** Delete this temporary span. */
function deleteSelf() {
  const pos = props.getPos();
  const { state, dispatch } = props.view;
  let tr = state.tr.delete(pos, pos + props.node.nodeSize);
  tr = tr.setSelection(TextSelection.create(tr.doc, pos));
  dispatch(tr);
  props.view.focus();
}

//
// Interaction
//

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  inputRef.value?.focus?.();
}

useFloating({
  floating: popoverRef,
  reference: inputContainerRef,
  options: {
    placement: "bottom-left",
    referenceMargin: 4,
  },
});

defineExpose<Partial<ViewExpose>>({ focus });
</script>
<template>
  <div class="relative inline rounded bg-gray-100 px-0.5 py-0.5" @blur="closeSelf">
    <!-- Input container -->
    <div ref="inputContainerRef" class="inline">
      <!-- Type -->
      <span>{{ type }}</span>
      <!-- Input -->
      <span
        ref="inputRef"
        class="ml-0.5 inline-block outline-none"
        contenteditable="true"
        @keydown.escape.prevent="closeSelf"
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
    <div ref="popoverRef" class="absolute z-50 rounded border border-gray-200 bg-white">
      <!-- nocheckin: Special input popover -->
      popover
    </div>
  </div>
</template>
