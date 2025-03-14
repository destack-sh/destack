import inspect
import re
from typing import Annotated, Any, cast, get_args, get_origin

from bench.language import Action, ActionType, Field, Icon, Kit, NodeMode, TypeIn, code, text
from bench.utils.func import parse_py_annotation


def class_to_kit(
    cls: type[Any], name: str, mode: NodeMode = NodeMode.BUILTIN, template: Kit | None = None
) -> Kit:
    """
    Turn a class into a Kit. Methods become Actions, their signature become Fields.
    Also parses out special metadata like ICON=...
    """

    kit = Kit.new(name, mode=mode, template=template)
    text_value = inspect.getdoc(cls) or ""
    if text_value:
        kit.text = text(text_value)

    for method_name, method in inspect.getmembers(cls, predicate=inspect.isfunction):
        if method_name.startswith("_"):
            continue

        action = Action.new(ActionType.CODE, name=method_name.replace("_", " ").title())

        # template
        if template is not None:
            template_action = template.actions.get(method_name)
            if template_action is not None:
                action.ck = template_action.ck
        else:
            template_action = None

        # icon/text
        doc = inspect.getdoc(method) or ""
        if doc:
            icon_match = re.search(r"ICON:\s*(.*)", doc)
            icon_value = icon_match.group(1).strip() if icon_match else None
            if icon_value:
                action.icon = Icon.new(icon_value)
            clean_doc = re.sub(r"ICON:.*(\n|$)", "", doc).strip()
            if clean_doc:
                action.text = text(clean_doc)
        elif template_action is not None:
            action.icon = template_action.icon
            action.text = template_action.text

        # code
        if not inspect.isabstract(method):
            method_source = inspect.getsource(method)
            lines = method_source.splitlines()
            # find the first line with actual code (after the def line and any docstring)
            start_idx = 0
            for i, line in enumerate(lines):
                if "def " in line.strip():
                    start_idx = i + 1
                    break
            # skip docstring if present
            if (start_idx < len(lines) and '"""' in lines[start_idx]) or "'''" in lines[start_idx]:
                for i in range(start_idx + 1, len(lines)):
                    if '"""' in lines[i] or "'''" in lines[i]:
                        start_idx = i + 1
                        break
            if start_idx < len(lines):
                method_body = lines[start_idx:]
                indent = len(method_body[0]) - len(method_body[0].lstrip())
                method_body = [line[indent:] if line.strip() else line for line in method_body]
                method_source = "\n".join(method_body)
            else:
                method_source = "pass"  # Empty method body
            action.code = code(method_source)

        # inputs
        sig = inspect.signature(method)
        params = list(sig.parameters.items())
        if params and params[0][0] == "self":
            params = params[1:]
        for param_name, param in params:
            param_type = param.annotation if param.annotation != inspect.Parameter.empty else Any
            default_value = None if param.default == inspect.Parameter.empty else param.default
            type_info = parse_py_annotation(cast(Any, param_type), {})
            field = Field.input(
                name=param_name.replace("_", " ").title(),
                typ=cast(TypeIn, type_info.type),
                default=default_value,
                is_list=type_info.is_list,
                is_required=not type_info.is_optional,
            )
            action.fields.append(field)

        # outputs
        return_type = sig.return_annotation
        if (
            return_type != inspect.Signature.empty
            and return_type is not None
            and get_origin(return_type) == Annotated
        ):
            _, metadata = get_args(return_type)
            if isinstance(metadata, dict):
                for output_name, output_type in metadata.items():
                    type_info = parse_py_annotation(output_type, {})
                    field = Field.output(
                        name=output_name.replace("_", " ").title(),
                        typ=type_info.type,
                        is_list=type_info.is_list,
                        is_required=not type_info.is_optional,
                    )
                    action.fields.append(field)

        # 'inherit' base field's ck for instances
        if template_action is not None:
            for template_field in template_action.fields:
                assert template_field.name, f"{template_field!r} has no name"
                field = action.fields.get(template_field.name)
                if field is not None:
                    field.ck = template_field.ck

        kit.actions.append(action)

    return kit
