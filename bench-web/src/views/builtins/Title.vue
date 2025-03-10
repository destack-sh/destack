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
    forceLineType?: TextLineType;
    placeholder?: string;
    truncate?: boolean;
    isSmall?: boolean;
  } & Partial<Pick<ViewData, "isInput" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();

const textRef = ref<HTMLElement | null>(null);
const textInterface = useTextLineModelValueInterface({
  modelValue: toRef(props, "modelValue"),
  forceLineType: props.forceLineType,
  update: (text) => emit("update:modelValue", text),
});
const vueInstance = getCurrentInstance();
if (vueInstance == null) throw new Error("no vue instance in Text");

const plugins: Plugin[] = [usePlaceholderPlugin({ defaultPlaceholder: props.placeholder, showIfUnfocused: true })];
if (props.isInput && props.forceLineType == TextLineType.PARAGRAPH) {
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
  history: true,
});
// NOTE :Performance: maybe render simple Text into static DOM node (if readonly)?
//  (see https://discuss.prosemirror.net/t/render-doc-content-to-html/4193)

const actions: Partial<ActionMapKit<"space">> = {
  ...textActions,
};

defineExpose({ actions, focus });
</script>
<template>
  <div
    ref="textRef"
    class="pm-text pm-compact relative rounded hover:cursor-text"
    :class="[isSmall ? 'pm-small' : '', truncate ? 'pm-truncate truncate' : '']"
    data-suppress-actions="space.move.left,space.move.right,space.history.undo,space.history.redo"
    data-suppress-drag="both"
  >
    <!-- ... -->
  </div>
</template>
