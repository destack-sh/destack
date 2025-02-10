<script lang="ts" setup>
import { isTextEmpty } from "@/language/core/text";
import { NodeType, TextData, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { useTextEditor, useTextModelValueInterface } from "@/ui/prosemirror";
import { NavigationDirection, type ViewEmits, type ViewExposed } from "@/views/common";
import { ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: TextData;
    placeholder?: string;
    suppressEnter?: boolean;
    suppressDrop?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInput" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

const textRef = ref<HTMLElement | null>(null);
const textInterface = useTextModelValueInterface({
  modelValue: toRef(props, "modelValue"),
  update: (text) => emit("update:modelValue", text),
});
const { focus, actions, isInDropZone } = useTextEditor({
  textRef,
  text: textInterface,
  isInput: toRef(props, "isInput"),
  suppressEnter: toRef(props, "suppressEnter"),
  suppressDrop: toRef(props, "suppressDrop"),
  navigate: (direction: NavigationDirection) => emit("navigate", direction),
  deleteSelf: () => emit("deleteSelf"),
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div
    ref="textRef"
    class="pm-text relative rounded hover:cursor-text"
    :class="[
      !isMinimal
        ? 'border border-gray-200 px-2 py-0.5 focus-within:border-gray-400 not-focus-within:hover:border-gray-200'
        : 'stealth',
      isInDropZone ? 'outline-dotted outline-2 outline-gray-400' : '',
    ]"
    data-suppress-actions="space.move.left,space.move.right"
    data-suppress-drag="both"
  >
    <!-- Placeholder -->
    <template v-if="modelValue == null || isTextEmpty(modelValue)">
      <div
        v-if="placeholder"
        class="pointer-events-none absolute"
        :class="!isMinimal ? 'left-2 top-1' : 'left-0.5 top-0.5'"
      >
        <div class="text-base text-gray-400">{{ placeholder }}</div>
      </div>
    </template>
  </div>
</template>
