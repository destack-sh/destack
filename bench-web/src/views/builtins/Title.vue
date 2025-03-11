<script lang="ts" setup>
import { TextLineData, TextLineType, ViewData } from "@/proto/wire";
import { ActionMapKit } from "@/ui/action";
import { useTextEditor } from "@/ui/prosemirror/editor";
import { usePlaceholderPlugin, useTooltipPlugin } from "@/ui/prosemirror/view";
import { useTextLineModelValueInterface } from "@/ui/prosemirror/wiring";
import TextTooltip from "@/views/builtins/TextTooltip.vue";
import { NavigationDirection, type ViewEmits } from "@/views/common";
import { Plugin } from "prosemirror-state";
import { getCurrentInstance, ref, toRef } from "vue";

const props = defineProps<
  {
    modelValue?: TextLineData;
    forceLineType?: TextLineType | "inherit";
    placeholder?: string;
    truncate?: boolean;
    isSmall?: boolean;
    hideMentions?: boolean;
  } & Partial<Pick<ViewData, "isInput" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();

const textRef = ref<HTMLElement | null>(null);
const forceLineType = props.forceLineType == "inherit" ? TextLineType.PARAGRAPH : props.forceLineType;
const textInterface = useTextLineModelValueInterface({
  modelValue: toRef(props, "modelValue"),
  forceLineType,
  hideMentions: props.hideMentions,
  update: (text) => emit("update:modelValue", text),
});
const vueInstance = getCurrentInstance();
if (vueInstance == null) throw new Error("no vue instance in Text");

const plugins: Plugin[] = [
  usePlaceholderPlugin({ defaultPlaceholder: props.placeholder, placeholderByNodeType: {}, showIfUnfocused: true }),
];
if (props.isInput && forceLineType) {
  plugins.push(useTooltipPlugin({ component: TextTooltip, parentComponent: vueInstance, container: textRef }));
}

const { focus, actions: textActions } = useTextEditor({
  mode: "line",
  textRef,
  text: textInterface,
  isInput: toRef(props, "isInput"),
  suppressEnter: true,
  suppressDrop: true,
  navigate: (direction: NavigationDirection) => emit("navigate", direction),
  deleteSelf: () => emit("deleteSelf"),
  parentComponent: vueInstance,
  plugins,
});
// NOTE :Performance: having each Title be its own full Text editor is a bit heavy

const actions: Partial<ActionMapKit<"space">> = {
  ...textActions,
};

defineExpose({ actions, focus });
</script>
<template>
  <div
    ref="textRef"
    class="pm-text pm-compact pm-paddingless relative inline-block rounded hover:cursor-text"
    :class="[
      truncate ? 'pm-truncate truncate' : '',
      props.forceLineType == 'inherit' ? 'pm-inherit' : isSmall ? 'pm-sm' : 'pm-base',
    ]"
    data-suppress-actions="space.move.left,space.move.right"
    data-suppress-drag="both"
  >
    <!-- ... -->
  </div>
</template>
