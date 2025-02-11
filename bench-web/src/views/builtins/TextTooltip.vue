<script lang="ts" setup>
import { IconData } from "@/proto/wire";
import { IconInline, makeIcon } from "@/ui/icon";
import { TextMarkType } from "@/ui/prosemirror/schema";
import { type TooltipProps } from "@/ui/prosemirror/view";
import * as commands from "prosemirror-commands";

const props = defineProps<TooltipProps>();

type FormatAction = {
  title: string;
  icon: IconData;
  isChecked: () => boolean;
  toggle: () => void;
};

const ICON_BY_MARK: Record<TextMarkType, IconData> = {
  bold: makeIcon("fas fa-bold"),
  italic: makeIcon("fas fa-italic"),
  underline: makeIcon("fas fa-underline"),
  strikethrough: makeIcon("fas fa-strikethrough"),
};

function markFormatAction(mark: TextMarkType): FormatAction {
  const action: FormatAction = {
    title: mark,
    icon: ICON_BY_MARK[mark],
    isChecked: () => {
      const { from, to } = props.view.state.selection;
      let hasMark = false;
      props.view.state.doc.nodesBetween(from, to, (node) => {
        if (node.marks.some((markType) => markType.type.name === mark)) {
          hasMark = true;
        }
      });
      return hasMark;
    },
    toggle: () => {
      const view = props.view;
      const state = view.state;
      commands.toggleMark(state.schema.marks[mark])(state, view.dispatch);
    },
  };
  return action;
}

const formatActions: FormatAction[] = [
  markFormatAction("bold"),
  markFormatAction("italic"),
  markFormatAction("underline"),
  markFormatAction("strikethrough"),
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
    >
      <!-- Mark-ish actions -->
      <button
        v-for="action in formatActions"
        :key="action.title"
        class="rounded px-1 py-0.5 hover:bg-gray-100"
        :class="[action.isChecked() ? 'text-primary-700' : '']"
        @click.stop.prevent="action.toggle()"
      >
        <IconInline class="w-5 text-center" v-bind="action.icon" />
      </button>
    </div>
  </Transition>
</template>
