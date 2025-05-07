<script lang="ts" setup>
import { NodeType, TextData, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { CommandMapKit } from "@/ui/command";
import { useTextEditor } from "@/ui/prosemirror/editor";
import { usePlaceholderPlugin, useTooltipPlugin } from "@/ui/prosemirror/view";
import { useTextModelValueInterface } from "@/ui/prosemirror/wiring";
import TextTooltip from "@/views/builtin/TextTooltip.vue";
import { NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import { Plugin } from "prosemirror-state";
import { getCurrentInstance, ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: TextData;
    placeholder?: string;
    suppressEnter?: boolean;
    suppressDrop?: boolean;
    isSmall?: boolean;
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
const vueInstance = getCurrentInstance();
if (vueInstance == null) throw new Error("no vue instance in Text");

const plugins: Plugin[] = [
  usePlaceholderPlugin({
    defaultPlaceholder: props.placeholder,
    showIfUnfocused: true,
    showAtBeginningOnly: true,
  }),
];
if (props.isInput) {
  plugins.push(useTooltipPlugin({ component: TextTooltip, parentComponent: vueInstance, container: textRef }));
}

const { focus, commands: textCommands } = useTextEditor({
  mode: "block",
  textRef,
  text: textInterface,
  isInput: toRef(props, "isInput"),
  suppressEnter: toRef(props, "suppressEnter"),
  suppressDrop: toRef(props, "suppressDrop"),
  navigate: (direction: NavigationDirection) => emit("navigate", direction),
  deleteSelf: () => emit("deleteSelf"),
  enter: () => emit("enter"),
  parentComponent: vueInstance,
  plugins,
  history: true,
});
// NOTE :Performance: maybe render simple Text into static DOM node (if readonly)?
//  (see https://discuss.prosemirror.net/t/render-doc-content-to-html/4193)

const commands: Partial<CommandMapKit<"space">> = {
  ...textCommands,
};

canvas.registerView(self, id);
defineExpose<ViewExpose>({ self, id, commands, focus });
</script>
<template>
  <div
    ref="textRef"
    class="pm-text pm-compact relative rounded-sm"
    :class="[
      !isMinimal
        ? 'border border-gray-200 px-2 py-0.5 focus-within:border-gray-400 not-focus-within:hover:border-gray-200'
        : '',
      isSmall ? 'pm-sm' : 'pm-base',
    ]"
    data-suppress-actions="space.move.left,space.move.right,space.history.undo,space.history.redo"
    data-suppress-drag="both"
  >
    <!-- ... -->
  </div>
</template>
