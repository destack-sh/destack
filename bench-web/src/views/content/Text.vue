<script lang="tsx" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { onMounted, ref, toRef } from "vue";
import { makeViewId } from "@/views";
import { Schema } from "prosemirror-model";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { whenever } from "@vueuse/core";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "title" | "text" | "icon" | "variant" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const trivialSchema = new Schema({
  nodes: {
    doc: { content: "paragraph+" },
    paragraph: {
      content: "text*",
      toDOM() {
        return ["p", 0];
      },
    },
    text: { inline: true },
  },
});
const state = EditorState.create({ schema: trivialSchema });
const textRef = ref<HTMLDivElement | null>(null);

whenever(
  textRef,
  () => {
    const view = new EditorView(textRef.value, { state });
  },
  { once: true },
);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <label v-if="title" class="mb-0.5 block font-medium text-gray-900">{{ title }}</label>
    <div ref="textRef" class="px-2 py-0.5 focus-within:border-primary-400"></div>
  </div>
</template>
<style></style>
