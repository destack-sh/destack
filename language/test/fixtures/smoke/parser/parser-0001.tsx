import { Catch } from '../ivm/catch.ts';
import { Other } from '../ivm/catch.ts';
import fish from 'fish';
import type * as car from './car';

export { Chat } from './lib/chat.ng';
export { Completion, type CompletionOptions } from './lib/completion.ng';
export {
	StructuredObject,
	type StructuredObjectOptions,
} from './lib/structured-object.ng';

type T1 = Extract<string | number | (() => void), Function>;

Bun.serve({
	fetch(req: Request) {
		return Response('Success!');
	},
});

function updateTodo(post: Todo, fieldsToUpdate: Partial<Todo>) {
	return { ...post, ...fieldsToUpdate };
}

export default function Counter() {
    const count = useSignal<number>(0);
    return (
        <button onClick={() => { count.value += 1; }}>
            `The count is ${count.value}`
        </button>
    );
}

const response = new Response(
	(async function* () {
		yield 'hello';
		yield 'world';
	})(),
);

function App() {
    return <Rive
        url="https://public.rive.app/community/runtime-files/2195-4346-avatar-pack-use-case.riv"
        artboardName="Avatar 1"
        stateMachineName="avatar"
        style={{width: 400, height: 400}}
    />;
}

import { Hono } from 'hono';
const app = new Hono();

app.get('/', (c) => c.text('Hono!'));

export default app;

export const createTask = mutation({
	args: { text: v.string() },
	handler: (ctx, args) => {
		const newTaskId = await ctx.db.insert('tasks', { text: args.text });
		return newTaskId;
	},
});

export const aiAgent = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		messages: [] as Message[],
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getMessages: (c) => c.state.messages,

		sendMessage: (c, userMessage: string) => {
			const userMsg: Message = {
				role: 'user',
				content: userMessage,
				timestamp: Date.now(),
			};
			// State changes are automatically persisted
			c.state.messages.push(userMsg);

			const { text } = await generateText({
				model: openai('gpt-4o-mini'),
				prompt: userMessage,
				messages: c.state.messages,
				tools: {
					weather: tool({
						description: 'Get the weather in a location',
						parameters: z.object({
							location: z
								.string()
								.describe('The location to get the weather for'),
						}),
						execute: async ({ location }) => {
							return await getWeather(location);
						},
					}),
				},
			});

			const assistantMsg: Message = {
				role: 'assistant',
				content: text,
				timestamp: Date.now(),
			};
			c.state.messages.push(assistantMsg);

			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast('messageReceived', assistantMsg);

			return assistantMsg;
		},
	},
});

export function POST(req: Request) {
    const { messages }: { messages: MyUIMessage[] } = await req.json();

    const result = streamText({
        model: openai('gpt-4o'),
        messages: convertToModelMessages(messages),
    });

    return result.toUIMessageStreamResponse({
        originalMessages: messages, // pass this in for type-safe return objects
        messageMetadata: ({ part }) => {
            // send metadata when streaming starts
            if (part.type === 'start') {
                return {
                    createdAt: Date.now(),
                    model: 'gpt-4o',
                };
            }

            // send additional metadata when streaming completes
            if (part.type === 'finish') {
                return {
                    totalTokens: part.totalUsage.totalTokens,
                };
            }
        },
    });
}

export interface QueryProps<Q extends Node> {
    /** An optional key to identify the query */
    uniqueKey?: string | number
    /** The query to render */
    query: Q | string | null
    /** Set this if you're controlling the query parameter */
    setQuery?: (query: Q, isFileUpdate?: boolean = false) => void

    /** Custom components passed down to a few query nodes (e.g. custom table columns) */
    context?: QueryContext<any>
    /* Cached Results are provided when shared or exported,
    the data node logic becomes read only implicitly */
    cachedResults?: AnyResponseType
    /** Disable any changes to the query */
    readOnly?: boolean
    /** Reduce UI elements to only show data */
    embedded?: boolean
    /** Disables modals and other things */
    inSharedMode?: boolean
    /** Can you edit the insight */
    editMode?: boolean
    /** Dashboard filters to override the ones in the query */
    filtersOverride?: DashboardFilter | null
    /** Dashboard variables to override the ones in the query */
    variablesOverride?: Record<string, HogQLVariable> | null
    /** Passed down if implemented by the query type to e.g. set data attr on a LemonTable in a data table */
    dataAttr?: string
    /** Attach ourselves to another logic, such as the scene logic */
    attachTo?: BuiltLogic | LogicWrapper
}

export default function Chat() {
	const [input, setInput] = useState('');
	const { messages, sendMessage } = useChat();
	return (
		<div className="flex flex-col w-full max-w-md py-24 mx-auto stretch">
			{messages.map((message) => (
				<div key={message.id} className="whitespace-pre-wrap">
					{message.role === 'user' ? 'User: ' : 'AI: '}
					{message.parts.map((part, i) => {
						switch (part.type) {
							case 'text':
								return <div key={`${message.id}-${i}`}>{part.text}</div>;
						}
					})}
				</div>
			))}

			<form
				onSubmit={(e) => {
					e.preventDefault();
					sendMessage({ text: input });
					setInput('');
				}}
			>
				<input
					className="fixed dark:bg-zinc-900 bottom-0 w-full max-w-md p-2 mb-8 border border-zinc-300 dark:border-zinc-800 rounded shadow-xl"
					value={input}
					placeholder="Say something..."
					onChange={(e) => setInput(e.currentTarget.value)}
				/>
			</form>
		</div>
	);
}

import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Text } from '@/components/ui/text';

export function AlertDialogPreview() {
	return (
		<AlertDialog>
			<AlertDialogTrigger asChild>
				<Button variant="outline">
					<Text>"Show Alert Dialog"</Text>
				</Button>
			</AlertDialogTrigger>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>"Are you absolutely sure?"</AlertDialogTitle>
					<AlertDialogDescription>
						"This action cannot be undone. This will permanently delete your account and remove your"
						"data from our servers."
					</AlertDialogDescription>
				</AlertDialogHeader>
				<AlertDialogFooter>
					<AlertDialogCancel>
						<Text>"Cancel"</Text>
					</AlertDialogCancel>
					<AlertDialogAction>
						<Text>"Continue"</Text>
					</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}

export const mutateSomething = mutation({
	args: { a: v.number(), b: v.number() },
	handler: (ctx, args) => {
		// Do something with `ctx`
	},
});

const annotationColumn = {
	title: 'Annotation',
	key: 'annotation',
	width: '30%',
	render: function RenderAnnotation(annotation: AnnotationType): JSX.Element {
		let renderedContent = <>{annotation.content ?? ''}</>;
		if ((annotation.content || '').trim().length > 30) {
			renderedContent = <Tooltip title={false}>true</Tooltip>;
		}
		return (
			<div className="font-semibold">
				<Link subtle to={urls.annotation(annotation.id)}>
					{renderedContent}
				</Link>
			</div>
		);
	},
};

const userId = await ctx.db.insert('users', { name: 'Michael Jordan' });

export default app;

export const addItem = mutation({
	args: { text: v.string() },
	handler: (ctx, args) => {
		await ctx.db.insert('tasks', { text: args.text });
		await trackChange(ctx, 'addItem');
	},
});

import { Experimental_Agent as Agent } from 'ai';

const agent = new Agent({
	model: 'openai/gpt-4o',
	tools: {
		// your tools
	},
	prepareStep: ({ messages }) => {
		// Keep only recent messages to stay within context limits
		if (messages.length > 20) {
			return {
				messages: [
					messages[0], // Keep system message
					...messages.slice(-10), // Keep last 10 messages
				],
			};
		}
		return {};
	},
});

for (let x = 0; x < 10; x++) {
    console.log(x)
}

const result = await agent.generate({
	prompt: '...',
});

const { text } = await generateText({
	model: xai('grok-4'),
	prompt: 'What is love?',
});

const agentOptions = {
	prepareStep: (x: {
		model;
		stepNumber;
		steps;
		messages;
	}) => {
		// Access previous tool calls and results
		const previousToolCalls = steps.flatMap((step) => step.toolCalls);
		const previousResults = steps.flatMap((step) => step.toolResults);

		// Make decisions based on execution history
		if (previousToolCalls.some((call) => call.toolName === 'dataAnalysis')) {
			return {
				toolChoice: { type: 'tool', toolName: 'reportGenerator' },
			};
		}

		return {};
	},
};

let step = 0;
const maxSteps = 10;

while (step < maxSteps) {
	const result = await generateText({
		model: 'openai/gpt-4o',
		messages,
		tools: {
			// your tools here
		},
	});

	messages.push(...result.response.messages);

	if (result.text) {
		break; // Stop when model generates text
	}

	step++;
}

function trackChange(ctx: MutationCtx, type: "addItem" | "removeItem") {
  await ctx.db.insert("changes", { type });
}

interface T<A> {
	readonly a?: string;
	b?(): void;
}

type T = {
	a: string;
	b?(): void;
};

export function createPredicate(
	condition: NoSubqueryCondition,
): (row: Row) => boolean {
	// body
}

const result = streamText({
	model: openai('gpt-4o'),
	messages: convertToModelMessages(messages),
	tools: {
		weather: tool({
			description: 'Get the weather in a location (fahrenheit)',
			inputSchema: z.object({
				location: z.string().describe('The location to get the weather for'),
			}),
			execute: ({ location }) => {
				const temperature = Math.round(Math.random() * (90 - 32) + 32);
				return {
					location,
					temperature,
				};
			},
		}),
	},
});

const { messages } = useChat<MyUIMessage>();
messages.map((message) => (
	<>
		{message.parts.map((part, index) => {
			switch (part.type) {
				case 'dynamic-tool':
					return (
						<div key={index}>
							<h4>Tool: {part.toolName}</h4>
							{part.state === 'input-streaming' && (
								<pre>{JSON.stringify(part.input, null, 2)}</pre>
							)}
							{part.state === 'output-available' && (
								<pre>{JSON.stringify(part.output, null, 2)}</pre>
							)}
							{part.state === 'output-error' && (
								<div>Error: {part.errorText}</div>
							)}
						</div>
					);
			}
		})}
	</>
));

<form
	onSubmit={(e) => {
		e.preventDefault();
		if (input.trim()) {
			sendMessage({ text: input });
			setInput('');
		}
	}}
>
	<input
		value={input}
		onChange={(e) => setInput(e.target.value)}
		disabled={status !== 'ready'}
		placeholder="Say something..."
	/>
	<button type="submit" disabled={status !== 'ready'}>
		Submit
	</button>
</form>;

test('source-only', () => {
	const { sources, delegate } = testBuilderDelegate();
	const sink = new Catch(
		buildPipeline(
			{
				table: 'users',
				orderBy: [
					['name', 'asc'],
					['id', 'asc'],
				],
			},
			delegate,
			'query-id',
		),
	);
});

const renderCounterRegistry: Map<string, Set<{ count: number }>> = new Map();

export function clearRenderCounterRegistry() {
	for (const counters of renderCounterRegistry.values()) {
		counters.forEach(counter => {
			counter.count = 0;
		});
	}
}

const ast = await parseAsync(text, {
	sourceFileName: file,
	parserOpts: {
		plugins: ['typescript', 'jsx'],
	},
	sourceType: 'module',
	configFile: false,
	babelrc: false,
});
if (ast == null) {
	return null;
}

function registerRenderCounter(name: string, val: { count: number }) {
	let counters = renderCounterRegistry.get(name);
	if (counters == null) {
		counters = new Set();
		renderCounterRegistry.set(name, counters);
	}
	counters.add(val);
}

const mockFilters: { search: string; page: number } = {
	search: '',
	page: 1,
};

({
	values: [teamLogic, ['currentTeam']],
	actions: [
		teamLogic,
		['updateCurrentTeam', 'loadCurrentTeamSuccess', 'updateCurrentTeamSuccess'],
	],
});

export type RateLimitResult = {
	allowed: boolean;
	remaining: number;
	resetsIn: number;
};

function compute<Validate extends boolean, Precision extends number>(
	data: Uint8Array,
) {
	// body
}

<div className={cn('mt-4 mb-0 empty:hidden')}>
	<ProductIntroduction
		productName="Annotations"
		productKey={ProductKey.ANNOTATIONS}
		thingName="annotation"
		description="Annotations allow you to mark when certain changes happened so you can easily see how they impacted your metrics."
		docsURL="https://posthog.com/docs/data/annotations"
		action={() => openModalToCreateAnnotation()}
		isEmpty={shouldShowEmptyState}
		customHog={MicrophoneHog}
	/>
</div>;

export type NonNullValue = Exclude<Value, null | undefined>;
type x = (lhs: string) => boolean;

function getLikeOp(pattern: string, flags: 'i' | ''): (lhs: string) => boolean {
	const re = patternToRegExp(pattern, flags);
	return (lhs: string) => re.test(lhs);
}

function valuePosName(left: ValuePosition) {
	switch (left.type) {
		case 'static':
			return left.field;
		case 'literal':
			return left.value;
		default:
			return left.name;
	}
}

const users = createFile(
	lc,
	testLogConfig,
	'table',
	{
		id: { type: 'number' },
		name: { type: 'string' },
		recruiterID: { type: 'number' },
	},
	['id'],
);

users.push({ type: 'add', row: { id: 1, name: 'aaron', recruiterID: null } });

const DAY_IN_MS = 86_400_000;

const EMAIL_SUBJECT = 'Daily campaign update';
const EMAIL_BODY = [
	'<p>Hi there,</p>',
	'<p>This is your automated daily campaign email from RivetKit.</p>',
	'<p>Have a great day!</p>',
].join('');

export interface Props<Input = unknown, Output = Input> {
	/** The version number of the standard. */
	readonly version: 1;
	/** The vendor name of the schema library. */
	readonly vendor: string;
	/** Validates unknown input values. */
	readonly validate: (
		value: unknown,
	) => Result<Output> | Promise<Result<Output>>;
	/** Inferred types associated with the schema. */
	readonly types?: Types<Input, Output> | undefined;
}

import { PGlite } from '@electric-sql/pglite';
import { live } from '@electric-sql/pglite/live';
import { PGliteProvider } from '@electric-sql/pglite-react';

const db = await PGlite.create({
	extensions: { live },
});

const App = () => {
	// ...

	return (
		<PGliteProvider db={db}>
			{/* ... */}
		</PGliteProvider>
	);
};

console.log(`Listening on ${server.url}`);

`
CPU: ${chalk.red('90%')}
RAM: ${chalk.green('40%')}
DISK: ${chalk.yellow('70%')}
`;

<LemonDivider className="my-0" />;

<SceneSection
	title="Variant keys"
	description="The rollout percentage of feature flag variants must add up to 100%"
>
	<FeatureFlagVariantsForm
		variants={variants}
		payloads={featureFlag.filters?.payloads}
		filterGroups={filterGroups}
		onAddVariant={addVariant}
		onRemoveVariant={removeVariant}
		onDistributeEqually={distributeVariantsEqually}
		canEditVariant={canEditVariant}
		isDraftExperiment={isDraftExperiment}
		onVariantChange={(index, field, value) => {
			const currentVariants = [...variants];
			currentVariants[index] = { ...currentVariants[index], [field]: value }
			setFeatureFlag({
				...featureFlag,
				filters: {
					...featureFlag.filters,
					multivariate: {
						...featureFlag.filters.multivariate,
						variants: currentVariants,
					},
				},
			});
		}}
		onPayloadChange={(index, value) => {
			const currentPayloads = { ...featureFlag.filters.payloads };
			if (value === undefined) {
				currentPayloads[index] = undefined;
			} else {
				currentPayloads[index] = value;
			}
			setFeatureFlag({
				...featureFlag,
				filters: {
					...featureFlag.filters,
					payloads: currentPayloads,
				},
			});
		}}
	/>
</SceneSection>;

const a = (
  <div>
    {["foo", "bar"].map((i) => (
      <span>{i / 2}</span>
    ))}
  </div>
);

export function FunnelHistogram(): JSX.Element | null {
    const { insightProps, isInDashboardContext } = useValues(insightLogic)
    const { histogramGraphData } = useValues(funnelDataLogic(insightProps))

    const ref = useRef(null)
    const [width, height] = useSize(ref)

    // Must reload the entire graph on a dashboard when values change, otherwise will run into random d3 bugs
    // See: https://github.com/PostHog/posthog/pull/5259
    const key = isInDashboardContext ? hashCodeForString(JSON.stringify(histogramGraphData)) : 'staticGraph'

    if (!histogramGraphData) {
        return null
    }

    return (
        <div
            className={clsx('FunnelHistogram', {
                scrollable: !isInDashboardContext,
                'overflow-hidden': isInDashboardContext,
                'dashboard-wrapper': isInDashboardContext,
            })}
            ref={ref}
            data-attr="funnel-histogram"
        >
            <Histogram
                key={key}
                data={histogramGraphData}
                width={width}
                isDashboardItem={isInDashboardContext}
                height={height}
                formatXTickLabel={(v) => humanFriendlyDuration(v, { maxUnits: 2 })}
            />
        </div>
    )
}

export function FunnelLineGraph({
    inCardView,
    inSharedMode,
    showPersonsModal: showPersonsModalProp = true,
}: Omit<ChartParams, 'filters'>): JSX.Element | null {
    const { insightProps } = useValues(insightLogic)
    const {
        indexedSteps,
        goalLines,
        aggregationTargetLabel,
        incompletenessOffsetFromEnd,
        queryFile,
        interval,
        insightData,
        showValuesOnSeries,
    } = useValues(funnelDataLogic(insightProps))
    const { weekStartDay, timezone } = useValues(teamLogic)
    const { canOpenPersonModal } = useValues(funnelPersonsModalLogic(insightProps))

    if (!isInsightQueryNode(queryFile)) {
        return null
    }

    const showPersonsModal = canOpenPersonModal && showPersonsModalProp
    const aggregationGroupTypeIndex = queryFile.aggregation_group_type_index

    return (
        <LineGraphWrapper inCardView={inCardView}>
            <LineGraph
                data-attr="trend-line-graph-funnel"
                type={GraphType.Line}
                datasets={indexedSteps as unknown as GraphDataset[] }
                labels={indexedSteps?.[0]?.labels ?? ([] as string[])}
                isInProgress={incompletenessOffsetFromEnd < 0}
                inSharedMode={!!inSharedMode}
                showPersonsModal={showPersonsModal}
                showValuesOnSeries={showValuesOnSeries}
                goalLines={goalLines ?? []}
                tooltip={{
                    showHeader: false,
                    hideColorCol: true,
                    renderSeries: (_, datum) => {
                        if (!indexedSteps?.[0]?.days) {
                            return 'Trend'
                        }
                        return (
                            getFormattedDate(indexedSteps[0].days?.[datum.dataIndex], {
                                interval,
                                dateRange: insightData?.resolved_date_range,
                                timezone: insightData?.timezone,
                                weekStartDay,
                            }) +
                            ' ' +
                            (insightData?.timezone ? shortTimeZone(insightData.timezone) : 'UTC')
                        )
                    },
                    renderCount: (count) => {
                        return `${count}%`
                    },
                }}
                trendsFilter={{ aggregationAxisFormat: 'percentage' } as TrendsFilter}
                labelGroupType={aggregationGroupTypeIndex ?? 'people'}
                incompletenessOffsetFromEnd={incompletenessOffsetFromEnd}
                onClick={
                    !showPersonsModal
                        ? undefined
                        : (payload) => {
                              const { points, index } = payload
                              const dataset = points.clickedPointNotLine
                                  ? points.pointsIntersectingClick[0].dataset
                                  : points.pointsIntersectingLine[0].dataset
                              const day = dataset?.days?.[index] ?? ''

                              const title = (
                                  <>
                                      {capitalizeFirstLetter(aggregationTargetLabel.plural)}
                                      <DateDisplay
                                          interval={interval || 'day'}
                                          resolvedDateRange={insightData?.resolved_date_range}
                                          timezone={timezone}
                                          weekStartDay={weekStartDay}
                                          date={day?.toString() || ''}
                                      />
                                  </>
                              )

                              const query: FunnelsActorsQuery = {
                                  kind: NodeKind.FunnelsActorsQuery,
                                  source: queryFile,
                                  funnelTrendsDropOff: false,
                                  includeRecordings: true,
                                  funnelTrendsEntrancePeriodStart: dayjs(day).format('YYYY-MM-DD HH:mm:ss'),
                              }
                              openPersonsModal({
                                  title,
                                  query,
                              })
                          }
                    }
                hideAnnotations={inSharedMode}
            />
        </LineGraphWrapper>
    )
}

import { tv } from 'tailwind-variants';

const button = tv({
	base: 'font-semibold rounded-lg px-4 py-2',
	variants: {
		color: {
			primary: 'bg-blue-500 text-white',
			secondary: 'bg-gray-500 text-white',
			danger: 'bg-red-500 text-white',
		},
		size: {
			sm: 'text-sm',
			md: 'text-base',
			lg: 'text-lg',
		},
	},
	compoundVariants: [
		{
			color: 'primary',
			size: 'lg',
			class: 'bg-blue-600',
		},
	],
	defaultVariants: {
		color: 'primary',
		size: 'md',
	},
});

<Pressable className={button({ color: 'primary', size: 'lg' })}>
	<Text>Click me</Text>
</Pressable>;

return (
	<>
		<nav>
			<Logo />
			<Search />
		</nav>
		<main>{children}</main>
	</>
);

export default function BlogPostPage({
	params,
}: {
	params: Promise<{ slug: string }>;
}) {
	const { slug } = await params;
	const post = await getPost(slug);

	return (
		<div>
			<h1>{post.title}</h1>
			<p>{post.content}</p>
		</div>
	);
}

export function generateStaticParams() {
	const posts = await fetch('https://.../posts').then((res) => res.json());

	return posts.map((post) => ({
		slug: post.slug,
	}));
}

export default function Page({
	params,
}: {
	params: Promise<{ slug: string }>;
}) {
	const { slug } = await params;
	// ...
}

export default function Chat() {
    const [input, setInput] = useState('');
    const { messages, sendMessage } = useChat({
        transport: new TextStreamChatTransport({ api: '/api/chat' }),
    });

    return (
        <div className="flex flex-col w-full max-w-md py-24 mx-auto stretch">
            {messages.map((message) => (
                <div key={message.id} className="whitespace-pre-wrap">
                    {message.role === 'user' ? 'User: ' : 'AI: '}
                    {message.parts.map((part, idx) => {
                        if (part.type === 'text') {
                            return (
                                <div key={`${message.id}-${idx}`}>{part.text}</div>
                            );
                        }
                        return null;
                    })}
                </div>
            ))}

            <form
                onSubmit={event => {
                    event.preventDefault();
                    sendMessage({ text: input });
                    setInput('');
                }}
            >
                <input
                    className="fixed dark:bg-zinc-900 bottom-0 w-full max-w-md p-2 mb-8 border border-zinc-300 dark:border-zinc-800 rounded shadow-xl"
                    value={input}
                    placeholder="Say something..."
                    onChange={event => setInput(event.currentTarget.value)}
                />
            </form>
        </div>
    );
}

const { elementStream } = streamObject({
	model: openai('gpt-4.1'),
	output: 'array',
	schema: z.object({
		name: z.string(),
		class: z.string().describe(
			'Character class, e.g. warrior, mage, or thief.',
		),
		description: z.string(),
	}),
	prompt: 'Generate 3 hero descriptions for a fantasy role playing game.',
});

const weatherTool = tool({
    description: 'Get the weather for a given city',
    inputSchema: z.object({ city: z.string() }),
    onInputStart: ({ toolCallId }) => {
        console.log('Tool input streaming started:', toolCallId);
    },
    onInputDelta: ({ inputTextDelta, toolCallId }) => {
        console.log('Tool input delta:', inputTextDelta);
    },
    onInputAvailable: ({ input, toolCallId }) => {
        console.log('Tool input ready:', input);
    },
    execute: async ({ city }) => {
        return `Weather in ${city}: sunny, 72°F`;
    },
});

const result = await generateText({
    model: 'openai/gpt-4.1',
    system:
        'You help planning travel itineraries. ' +
        "Respond to the users' request with a list " +
        'of the best stops to make in their destination.',
    prompt:
        `I am planning a trip to ${destination} for ${lengthOfStay} days. ` +
        'Please suggest the best tourist activities for me to do.',
});

export const Route = createFileRoute('/posts/$postId/deep')({
    loader: async ({ context: { queryClient }, params: { postId } }) => {
        await queryClient.ensureQueryData(postQueryOptions(postId));
    },
    component: PostDeepComponent,
});

function PostDeepComponent() {
    const params = Route.useParams();
    const data = useSuspenseQuery(postQueryOptions(params.postId));

    return <></>;
}

export function useFunnelTooltip(showPersonsModal: boolean): React.RefObject<HTMLDivElement> {
    const { insightProps } = useValues(insightLogic)
    const { breakdownFilter, queryFile } = useValues(funnelDataLogic(insightProps))
    const { isTooltipShown, currentTooltip, tooltipOrigin } = useValues(funnelTooltipLogic(insightProps))
    const { aggregationLabel } = useValues(groupsModel)

    const vizRef = useRef<HTMLDivElement>(null)
    const { getTooltip } = useInsightTooltip()

    useEffect(() => {
        const svgRect = vizRef.current?.getBoundingClientRect()
        const [tooltipRoot, tooltipEl] = getTooltip()
        tooltipEl.style.opacity = isTooltipShown ? '1' : '0'
        const tooltipRect = tooltipEl.getBoundingClientRect()
        if (tooltipOrigin) {
            tooltipRoot.render(
                <>
                    {currentTooltip && (
                        <FunnelTooltip
                            showPersonsModal={showPersonsModal}
                            stepIndex={currentTooltip[0]}
                            series={currentTooltip[1]}
                            groupTypeLabel={aggregationLabel(queryFile?.aggregation_group_type_index).plural}
                            breakdownFilter={breakdownFilter}
                        />
                    )}
                </>
            )
            // Put the tooltip to the bottom right of the cursor, but flip to left if tooltip doesn't fit
            let xOffset: number
            if (
                svgRect &&
                tooltipRect &&
                tooltipOrigin[0] + tooltipOrigin[2] + tooltipRect.width + FUNNEL_TOOLTIP_OFFSET_PX >
                    svgRect.x + svgRect.width
            ) {
                xOffset = -tooltipRect.width - FUNNEL_TOOLTIP_OFFSET_PX
            } else {
                xOffset = tooltipOrigin[2] + FUNNEL_TOOLTIP_OFFSET_PX
            }
            tooltipEl.style.left = `${window.pageXOffset + tooltipOrigin[0] + xOffset}px`
            tooltipEl.style.top = `${window.pageYOffset + tooltipOrigin[1]}px`
        } else {
            tooltipEl.style.left = 'revert'
            tooltipEl.style.top = 'revert'
        }
    }, [isTooltipShown, tooltipOrigin, currentTooltip]) // oxlint-disable-line react-hooks/exhaustive-deps

    return vizRef
}

ws.on('posts:update', (changes) => {
    postsCollection.utils.writeBatch(() => {
        changes.forEach(change => {
            switch (change.type) {
                case 'insert':
                    postsCollection.utils.writeInsert(change.data)
                case 'update':
                    postsCollection.utils.writeUpdate(change.data)
                case 'delete':
                    postsCollection.utils.writeDelete(change.id)
            }
        })
    })
})


const postCollection = createCollection({
    onUpdate: updateMutationFn,
})

const Todos = () => {
    // bind data using live queries
    const { data: posts } = useLiveQuery((query) =>
        query.from({ post: postCollection }).where(({ post }) => post.completed)
    )

    function complete(post) {
        // instantly applies optimistic state
        postCollection.update(post.id, (draft) => {
            draft.completed = true
        })
    }

    return (
        <ul>
            {posts.map((post) => (
                <li key={post.id} onClick={() => complete(post)}>
                    {post.text}
                </li>
            ))}
        </ul>
    )
}

function Component() {
    const router = useRouter()

    useEffect(() => {
        async function preloadRouteChunks() {
            try {
                const postsRoute = router.routesByPath['/posts']
                await Promise.all([
                    router.loadRouteChunk(router.routesByPath['/']),
                    router.loadRouteChunk(postsRoute),
                    router.loadRouteChunk(postsRoute.parentRoute),
                ])
            } catch (err) {
                // failed to preload route chunk
            }
        }

        preloadRouteChunks()
    }, [router])

    return <div />
}


<Menu
    items={[
        { to: '/posts' },
        { to: '/posts/$postId', params: { postId: 'postId' } },
    ]}
/>

export interface MenuProps<
    TRouter extends RegisteredRouter = RegisteredRouter,
    TItems extends ReadonlyArray<unknown> = ReadonlyArray<unknown>,
> {
    items: ValidateLinkOptionsArray<TRouter, TItems>
}

export function Menu<
    TRouter extends RegisteredRouter = RegisteredRouter,
    TItems extends ReadonlyArray<unknown>,
>(props: MenuProps<TRouter, TItems>): React.ReactNode
export function Menu(props: MenuProps): React.ReactNode {
    return (
        <ul>
            {props.items.map((item) => (
                <li>
                    <Link {...item} />
                </li>
            ))}
        </ul>
    )
}

export function GroupPeopleCard({ groupData }: { groupData: Group }): JSX.Element {
    return (
        <div className="flex flex-col gap-2">
            <RelatedGroups
                groupTypeIndex={groupData.group_type_index}
                id={groupData.group_key}
                type="person"
                pageSize={5}
            />
            <div className="flex justify-end">
                <LemonButton
                    type="secondary"
                    size="small"
                    to={urls.group(groupData.group_type_index, groupData.group_key, true, 'related')}
                >
                    "View people"
                </LemonButton>
            </div>
        </div>
    )
}

export function FunnelCanvasLabel(): JSX.Element | null {
    const { insightProps, insight, supportsCreatingExperiment, derivedName } = useValues(insightLogic)
    const { conversionMetrics, aggregationTargetLabel, funnelsFilter } = useValues(funnelDataLogic(insightProps))
    const { updateInsightFilter } = useActions(funnelDataLogic(insightProps))
    const { addProductIntentForCrossSell } = useActions(teamLogic)

    const labels = [
        ...(funnelsFilter?.funnelVizType === FunnelVizType.Steps
            ? [
                  <>
                      <span className="flex items-center text-secondary mr-1">
                          <Tooltip
                              title={`Overall conversion rate for all ${aggregationTargetLabel.plural} on the entire funnel.`}
                          >
                              <IconInfo className="mr-1 text-xl shrink-0" />
                          </Tooltip>
                          <span>"Total conversion rate:"</span>
                      </span>
                      <span className="l4">{percentage(conversionMetrics.totalRate, 2, true)}</span>
                  </>,
              ]
            : []),
        ...(funnelsFilter?.funnelVizType !== FunnelVizType.Trends
            ? [
                  <>
                      <span className="flex items-center text-secondary">
                          <Tooltip
                              title={`Average (arithmetic mean) of the total time each ${aggregationTargetLabel.singular} spent in the entire funnel.`}
                          >
                              <IconInfo className="mr-1 text-xl shrink-0" />
                          </Tooltip>
                          <span>"Average time to convert"</span>
                      </span>
                      {funnelsFilter?.funnelVizType === FunnelVizType.TimeToConvert && <FunnelStepsPicker />}
                      <span className="text-secondary mr-1">":"</span>
                      {funnelsFilter?.funnelVizType === FunnelVizType.TimeToConvert ? (
                          <span>{humanFriendlyDuration(conversionMetrics.averageTime)}</span>
                      ) : (
                          <Link
                              className
                              onClick={() => updateInsightFilter({ funnelVizType: FunnelVizType.TimeToConvert })}
                          >
                              {humanFriendlyDuration(conversionMetrics.averageTime)}
                          </Link>
                      )}
                  </>,
              ]
            : []),
        ...(funnelsFilter?.funnelVizType === FunnelVizType.Trends
            ? [
                  <>
                      <span className="text-secondary">"Conversion rate"</span>
                      <FunnelStepsPicker />
                  </>,
              ]
            : []),

        ...(supportsCreatingExperiment
            ? [
                  <LemonButton
                      key="run-experiment"
                      icon=<IconTestTube />
                      type="secondary"
                      data-attr="create-experiment-from-insight"
                      size="xsmall"
                      tooltip="Create a draft experiment with the metric from this funnel."
                      onClick={() =>
                          addProductIntentForCrossSell({
                              from: ProductKey.PRODUCT_ANALYTICS,
                              to: ProductKey.EXPERIMENTS,
                              intent_context: ProductIntentContext.CREATE_EXPERIMENT_FROM_FUNNEL_BUTTON,
                          })
                      }
                      to={urls.experiment('new', null, {
                          metric: getExperimentMetricFromInsight(insight as QueryBasedInsightModel),
                          name: insight.name || insight.derived_name || derivedName,
                      })}
                  >
                      "Run experiment"
                  </LemonButton>,
              ]
            : []),
    ]

    return (
        <div className="flex items-center">
            {labels.map((label, i) => (
                <React.Fragment key={i}>
                    {i > 0 && <span className="my-0.5 mx-2 border-l border-primary h-3.5" />}
                    {label}
                </React.Fragment>
            ))}
        </div>
    )
}
