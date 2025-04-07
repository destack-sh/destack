<script lang="ts" setup>
import { IconData } from "@/proto/wire";
import { Command, CommandBuiltinId, getCommand } from "@/ui/command";
import { IconInline, makeIcon } from "@/ui/icon";
import { hasTextMark, setTextMark } from "@/ui/prosemirror/editor";
import { TextMarkType } from "@/ui/prosemirror/schema";
import { type TooltipProps } from "@/ui/prosemirror/view";
import { TooltipInfo } from "@/ui/tooltip";

const props = defineProps<TooltipProps>();

type FormatCommand = {
  id: string | number;
  icon: IconData;
  command: Command;
  isChecked: () => boolean;
  toggle: () => void;
};

function markFormatCommand(mark: TextMarkType, commandId: CommandBuiltinId, icon: string): FormatCommand {
  const command = getCommand(commandId);
  const formatCommand: FormatCommand = {
    id: mark,
    icon: makeIcon(icon),
    command: command,
    isChecked: () => hasTextMark(props.view.state, props.view.state.selection, mark) !== false,
    toggle: () => {
      setTextMark(props.view.state, props.view.state.selection, mark, "toggle", props.view.dispatch);
    },
  };
  return formatCommand;
}

const formatCommands: FormatCommand[] = [
  markFormatCommand("bold", "text.format.bold", "fas fa-bold"),
  markFormatCommand("italic", "text.format.italic", "fas fa-italic"),
  markFormatCommand("underline", "text.format.underline", "fas fa-underline"),
  markFormatCommand("strikethrough", "text.format.strikethrough", "fas fa-strikethrough"),
  markFormatCommand("code", "text.format.code", "fas fa-code"),
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
      <!-- Mark-ish commands -->
      <button
        v-for="command in formatCommands"
        :key="command.id"
        v-tooltip="{
          title: command.command.title,
          shortcuts: command.command.shortcuts,
          group: 'text.format',
        }"
        class="rounded px-1 py-0.5 hover:bg-gray-100"
        :class="[command.isChecked() ? 'text-primary-700' : '']"
        @mousedown.stop.prevent="command.toggle()"
      >
        <IconInline class="w-5 text-center" v-bind="command.icon" />
      </button>
    </div>
  </Transition>
</template>
