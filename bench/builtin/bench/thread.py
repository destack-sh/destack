from bench.language import Membership, NodeMode, Page, Thread

from .agent import BenchAgent

BlankThread = Thread.new(
    "Blank Thread", memberships=[Membership.new(BenchAgent)], mode=NodeMode.TEMPLATE
)

ThreadPage = Page.new("Thread")
# TODO :Incomplete: use Threads as templates
# (can't right now beacuse we can't have Threads in our loaded node types for packages)
# ThreadPage.extend(BlankThread)
