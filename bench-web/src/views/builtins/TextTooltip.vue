<script lang="ts" setup>
import { IconData, TextSpanType } from "@/proto/wire";
import { IconInline, makeIcon } from "@/ui/icon";
import { hasTextMark, hasTextSpanType, setTextMark, setTextSpanType } from "@/ui/prosemirror/editor";
import { TextMarkType } from "@/ui/prosemirror/schema";
import { type TooltipProps } from "@/ui/prosemirror/view";

const props = defineProps<TooltipProps>();

type FormatAction = {
  id: string | number;
  icon: IconData;
  isChecked: () => boolean;
  toggle: () => void;
};

function markFormatAction(mark: TextMarkType, icon: string): FormatAction {
  const action: FormatAction = {
    id: mark,
    icon: makeIcon(icon),
    isChecked: () => hasTextMark(props.view.state, props.view.state.selection, mark) !== false,
    toggle: () => {
      setTextMark(props.view.state, props.view.state.selection, mark, "toggle", props.view.dispatch);
    },
  };
  return action;
}

function spanTypeFormatAction(type: TextSpanType, icon: string): FormatAction {
  const action: FormatAction = {
    id: type,
    icon: makeIcon(icon),
    isChecked: () => hasTextSpanType(props.view.state, props.view.state.selection, type) !== false,
    toggle: () => {
      setTextSpanType(props.view.state, props.view.state.selection, type, props.view.dispatch);
    },
  };
  return action;
}

const formatActions: FormatAction[] = [
  markFormatAction("bold", "fas fa-bold"),
  markFormatAction("italic", "fas fa-italic"),
  markFormatAction("underline", "fas fa-underline"),
  markFormatAction("strikethrough", "fas fa-strikethrough"),
  markFormatAction("code", "fas fa-code"),
];
</script>
<template>
  <Transition
    enter-active-class="transition-all ease-in duration-200"
    enter-from-class="opacity-0 scale-95"
    enter-to-class="opacity-100 scale-100"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 scale-100"
    leave-to-class="opacity-0 scale-95"
    appear
  >
    <div
      v-if="visible && tick >= 0 /* react to tick */"
      class="flex w-fit flex-row items-center rounded border border-gray-200 bg-white px-1 py-1"
      :style="{ height: `${height}px` }"
      @click.stop.prevent
      @mousedown.stop.prevent
    >
      <!-- Mark-ish actions -->
      <button
        v-for="action in formatActions"
        :key="action.id"
        class="rounded px-1 py-0.5 hover:bg-gray-100"
        :class="[action.isChecked() ? 'text-primary-700' : '']"
        @mousedown.stop.prevent="action.toggle()"
      >
        <IconInline class="w-5 text-center" v-bind="action.icon" />
      </button>
    </div>
  </Transition>
</template>
