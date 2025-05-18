import inspect
import re
from typing import Annotated, Any, cast, get_args, get_origin

from bench.language import (
    BENCH_CLASS_BY_NAME,
    Action,
    ActionType,
    Field,
    Icon,
    IconIn,
    NodeMode,
    Service,
    TypeIn,
    code,
    text,
    to_icon,
)
from bench.language.core.property import parse_type_annotation


def class_to_service(
    cls: type[Any],
    name: str,
    mode: NodeMode = NodeMode.BUILTIN,
    icon: IconIn | None = None,
    template: Service | None = None,
) -> Service:
    """
    Turn a class into a Service. Methods become Actions, their signature become Fields.
    Also parses out special metadata like ICON=...
    """

    service = Service.new(name, mode=mode, icon=to_icon(icon) if icon else None, template=template)

    for method_name, method in inspect.getmembers(cls, predicate=inspect.isfunction):
        action = Action.new(ActionType.BUILTIN, name=method_name.replace("_", " "))

        # template
        if template is not None:
            template_action = template.get_child(Action, method_name)
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
                method_source = "..."  # empty method body
            action.code = code(method_source)

        # inputs
        sig = inspect.signature(method)
        params = list(sig.parameters.items())
        if params and params[0][0] == "self":
            params = params[1:]
        for param_name, param in params:
            if param_name == "runner":
                continue  # ignore injected runner
            param_type = param.annotation if param.annotation != inspect.Parameter.empty else Any
            default_value = None if param.default == inspect.Parameter.empty else param.default
            type_info = parse_type_annotation(cast(Any, param_type), BENCH_CLASS_BY_NAME)
            field = Field.input(
                name=param_name.replace("_", " "),
                typ=cast(TypeIn, type_info.type),
                default=default_value,
                is_list=type_info.is_list,
                is_required=not type_info.is_optional,
            )
            action.add_child(field)

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
                    type_info = parse_type_annotation(output_type, BENCH_CLASS_BY_NAME)
                    assert isinstance(type_info.type, type), f"{type_info.type!r} is not a type"
                    field = Field.output(
                        name=output_name.replace("_", " ").title(),
                        typ=type_info.type,
                        is_list=type_info.is_list,
                        is_required=not type_info.is_optional,
                    )
                    action.add_child(field)

        # 'inherit' base field's ck for instances
        if template_action is not None:
            for template_field in template_action.get_children(Field):
                assert template_field.name, f"{template_field!r} has no name"
                field = action.get_child(Field, template_field.name)
                if field is not None:
                    field.ck = template_field.ck

        service.add_child(action)

    return service
