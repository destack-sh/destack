from bench.language import (
    Action,
    ActionType,
    Agent,
    Flow,
    LinkType,
    Page,
    Trigger,
    icon,
    text,
    vector2,
)

# default Flow
BenchFlow = Flow.new("Flow", icon=icon("fas fa-robot"), text=text("The default agentive Flow."))
Start1 = Action.new(
    ActionType.START,
    "Start",
    triggers=[Trigger.on_message("Message"), Trigger.on_start("Start")],
    position=vector2(0, 0),
)
Tool1 = Action.new(ActionType.TOOL, "Tool", position=vector2(400, 0))
Complete1 = Action.new(ActionType.END, "Complete", position=vector2(800, 0))
BenchFlow.actions.extend(Start1, Tool1, Complete1)
Start1.connect(LinkType.REQUIRE, Tool1)
Tool1.connect(LinkType.DECIDE, Tool1)
Tool1.connect(LinkType.DECIDE, Complete1)

# default Agent
BenchAgent = Agent.new(
    "Bench",
    icon=icon("fas fa-robot"),
    text=text("The default Agent of a Bench."),
    main_flow=BenchFlow,
)
BenchFlow.default_agent = BenchAgent

FlowPage = Page.new("Flow")
FlowPage.extend(BenchFlow, BenchAgent)

# nocheckin: reorganize builtins .. (mirroring language seems wrong)
