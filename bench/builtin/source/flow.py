from bench.language import (
    Action,
    ActionType,
    Flow,
    Identity,
    LinkType,
    Page,
    Trigger,
    icon,
    text,
    vector2,
)

# default Flow
DefaultFlow = Flow.new("Flow", icon=icon("fas fa-robot"), text=text("The default agentive Flow."))
Start1 = Action.new(
    ActionType.START,
    "Start",
    triggers=[Trigger.on_message("Message"), Trigger.on_start("Start")],
    position=vector2(0, 0),
)
Tool1 = Action.new(ActionType.TOOL, "Tool", position=vector2(400, 0))
Complete1 = Action.new(ActionType.END, "Complete", position=vector2(800, 0))
DefaultFlow.actions.extend(Start1, Tool1, Complete1)
Start1.connect(LinkType.REQUIRE, Tool1)
Tool1.connect(LinkType.DECIDE, Tool1)
Tool1.connect(LinkType.DECIDE, Complete1)

# default Agent
AgentIdentity = Identity.new(
    "Agent",
    icon=icon("fas fa-robot"),
    text=text("The default Agent Identity."),
    flow=DefaultFlow,
)
DefaultFlow.identity = AgentIdentity

FlowPage = Page.new("Flow")
FlowPage.extend(DefaultFlow, AgentIdentity)
