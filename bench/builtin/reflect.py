import inspect
import re
from typing import Annotated, Any, cast, get_args, get_origin

from bench.language import Action, ActionType, Field, Icon, Kit, NodeMode, TypeIn, code, text


def class_to_kit(cls: type[Any], name: str, mode: NodeMode = NodeMode.BUILTIN) -> Kit:
    """
    Turn a class into a Kit. Methods become Actions, their signature become Fields.
    Also parses out special metadata like ICON=...
    """

    kit = Kit.new(name, mode=mode)

    for method_name, method in inspect.getmembers(cls, predicate=inspect.isfunction):
        if method_name.startswith("_"):
            continue

        # action
        doc = inspect.getdoc(method) or ""
        icon_match = re.search(r"ICON:\s*(.*)", doc)
        icon_value = icon_match.group(1).strip() if icon_match else None
        action = Action.new(ActionType.TOOL, name=method_name)
        if icon_value:
            action.icon = Icon.new(icon_value)

        # text
        clean_doc = re.sub(r"ICON:.*(\n|$)", "", doc).strip()
        if clean_doc:
            action.text = text(clean_doc)

        # code
        if not inspect.isabstract(method):
            method_source = inspect.getsource(method)
            action.code = code(method_source)

        kit.actions.append(action)

        # inputs
        sig = inspect.signature(method)
        params = list(sig.parameters.items())
        if params and params[0][0] == "self":
            params = params[1:]
        for param_name, param in params:
            param_type = param.annotation if param.annotation != inspect.Parameter.empty else Any
            default_value = None if param.default == inspect.Parameter.empty else param.default

            field = Field.input(
                name=param_name, typ=cast(TypeIn, param_type), default=default_value
            )
            action.fields.append(field)

        # outputs
        return_type = sig.return_annotation
        if return_type != inspect.Signature.empty and return_type is not None:
            # Handle Annotated return types with metadata
            if get_origin(return_type) == Annotated:
                _, metadata = get_args(return_type)
                if isinstance(metadata, dict):
                    for output_name, output_type in metadata.items():
                        field = Field.output(name=output_name, typ=output_type)
                        action.fields.append(field)
            else:
                field = Field.output(name="result", typ=return_type)
                action.fields.append(field)

    return kit
