use std::fmt::Write;
use std::sync::Arc;

use destack_core::StringPool;
use destack_dir as dir;

use super::{TestSource, fixture_text, test_file};
use crate::{
    Binding, Matcher, Node, NodeId, NodeList, NthChild, Pattern, PatternMatch, Relation,
    RelationStop,
};

/// One additional relation required of a test pattern.
enum TestRelation {
    /// Search ancestors for the node type.
    Inside(dir::NodeType),
    /// Search descendants for the node type.
    Has(dir::NodeType),
    /// Search following siblings for the node type.
    Precedes(dir::NodeType),
    /// Search preceding siblings for the node type.
    Follows(dir::NodeType),
    /// Select one exact sibling position.
    NthChild(i32),
}

impl TestRelation {
    /// Add this relation to a compiled pattern.
    fn add_to(self, pattern: &mut Pattern) {
        let fragment = pattern.tree.root;
        let relation = match self {
            Self::Inside(node_type) => {
                let node_type = Self::add_node_type(pattern, node_type);

                Node::Inside(Relation {
                    pattern: node_type,
                    stop: RelationStop::End,
                })
            }
            Self::Has(node_type) => {
                let node_type = Self::add_node_type(pattern, node_type);

                Node::Has(Relation {
                    pattern: node_type,
                    stop: RelationStop::End,
                })
            }
            Self::Precedes(node_type) => {
                let node_type = Self::add_node_type(pattern, node_type);

                Node::Precedes(Relation {
                    pattern: node_type,
                    stop: RelationStop::End,
                })
            }
            Self::Follows(node_type) => {
                let node_type = Self::add_node_type(pattern, node_type);

                Node::Follows(Relation {
                    pattern: node_type,
                    stop: RelationStop::End,
                })
            }
            Self::NthChild(position) => Node::NthChild(NthChild {
                step: 0,
                offset: position,
                pattern: None,
            }),
        };
        let relation = NodeId(pattern.tree.nodes.allocate(relation));
        let start = pattern.tree.node_ids.len() as u32;
        pattern.tree.node_ids.extend([fragment, relation]);
        let all = Node::All(NodeList { start, length: 2 });
        pattern.tree.root = NodeId(pattern.tree.nodes.allocate(all));
    }

    /// Add one node-type operation to a compiled pattern.
    fn add_node_type(pattern: &mut Pattern, node_type: dir::NodeType) -> NodeId {
        NodeId(pattern.tree.nodes.allocate(Node::NodeType(node_type)))
    }
}

/// One source-aligned match annotation.
struct MatchAnnotation {
    /// The source offset selecting the preceding source line.
    anchor: u32,
    /// The byte column of the annotation marker.
    column: u32,
    /// The number of carets in the marker.
    width: u32,
    /// The match label and bindings.
    text: String,
}

impl MatchAnnotation {
    /// Render the source-aligned annotation.
    fn render(&self) -> String {
        let indent = " ".repeat(self.column as usize);
        let marker = "^".repeat(self.width as usize);

        format!("{indent}{marker} {}", self.text)
    }
}

/// Authored inputs for one matcher exercise.
pub(crate) struct TestMatcher {
    /// The structural pattern source.
    pattern: String,
    /// The candidate source.
    source: String,
    /// The contextual node selector.
    selector: Option<dir::NodeType>,
    /// The predicate sources in authored order.
    predicates: Vec<String>,
    /// Additional checked modules.
    files: Vec<(String, String)>,
    /// Additional pattern relations.
    relations: Vec<TestRelation>,
}

impl TestMatcher {
    /// Create one expression pattern exercise.
    pub(crate) fn new(pattern: &str, source: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            source: fixture_text(source).to_string(),
            selector: None,
            predicates: Vec::new(),
            files: Vec::new(),
            relations: Vec::new(),
        }
    }

    /// Create one contextual pattern exercise.
    pub(crate) fn context(pattern: &str, selector: dir::NodeType, source: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            source: fixture_text(source).to_string(),
            selector: Some(selector),
            predicates: Vec::new(),
            files: Vec::new(),
            relations: Vec::new(),
        }
    }

    /// Add one module to the checked fixture.
    pub(crate) fn file(mut self, path: &str, source: &str) -> Self {
        self.files
            .push((path.to_string(), fixture_text(source).to_string()));

        self
    }

    /// Add one semantic predicate.
    pub(crate) fn guard(mut self, predicate: &str) -> Self {
        self.predicates.push(predicate.to_string());

        self
    }

    /// Require the structural fragment to occur inside one node type.
    pub(crate) fn inside(mut self, node_type: dir::NodeType) -> Self {
        self.relations.push(TestRelation::Inside(node_type));

        self
    }

    /// Require the structural fragment to contain one node type.
    pub(crate) fn has(mut self, node_type: dir::NodeType) -> Self {
        self.relations.push(TestRelation::Has(node_type));

        self
    }

    /// Require the structural fragment to precede one node type.
    pub(crate) fn precedes(mut self, node_type: dir::NodeType) -> Self {
        self.relations.push(TestRelation::Precedes(node_type));

        self
    }

    /// Require the structural fragment to follow one node type.
    pub(crate) fn follows(mut self, node_type: dir::NodeType) -> Self {
        self.relations.push(TestRelation::Follows(node_type));

        self
    }

    /// Require the structural fragment to occupy one exact sibling position.
    pub(crate) fn nth_child(mut self, position: i32) -> Self {
        self.relations.push(TestRelation::NthChild(position));

        self
    }

    /// Assert every annotated match and binding.
    #[track_caller]
    pub(crate) fn assert(self, expected: &str) {
        let matches = self.compile();
        let actual = matches.render();
        let expected = fixture_text(expected).trim_end_matches('\n');

        assert_eq!(actual, expected);
    }

    /// Compile and run the authored matcher exercise.
    #[track_caller]
    fn compile(self) -> TestMatches {
        let (source, strings) = if self.predicates.is_empty() {
            let strings = Arc::new(StringPool::new());
            let source = TestSource::parse(&self.source, strings.clone());

            (source, strings)
        } else {
            TestSource::checked(&self.source, &self.files)
        };
        let pattern_file = test_file("<pattern>", &self.pattern);
        let mut pattern = match self.selector {
            Some(selector) => Pattern::parse_context(pattern_file, selector, strings.clone())
                .expect("compile test context"),
            None => Pattern::parse(pattern_file, strings.clone()).expect("compile test pattern"),
        };

        // compile predicates against the candidate program strings
        for (index, predicate) in self.predicates.iter().enumerate() {
            let name = format!("<predicate-{}>", index + 1);
            let predicate = test_file(&name, predicate);
            pattern
                .add_predicate(predicate)
                .expect("compile test predicate");
        }

        // add programmatic relation operations
        for relation in self.relations {
            relation.add_to(&mut pattern);
        }

        // match every candidate DIR node in parse allocation order
        let matcher = match source.program() {
            Some((program, module)) => {
                Matcher::in_module(&pattern, module, program).expect("build checked test matcher")
            }
            None => Matcher::new(&pattern, source.view()),
        };
        let matches = matcher
            .find(source.tree().iter_node_ids())
            .expect("match test pattern");

        TestMatches {
            pattern,
            source,
            matches,
        }
    }
}

/// Complete results of one matcher exercise.
struct TestMatches {
    /// The compiled pattern.
    pattern: Pattern,
    /// The candidate source.
    source: TestSource,
    /// Successful matches in candidate order.
    matches: Vec<PatternMatch>,
}

impl TestMatches {
    /// Overlay every match and binding on the candidate source.
    fn render(&self) -> String {
        let mut annotations = self
            .matches
            .iter()
            .flat_map(|pattern_match| self.annotations(pattern_match))
            .collect::<Vec<_>>();
        annotations.sort_by_key(|annotation| annotation.anchor);

        let source = self.source.text().trim_end_matches('\n');
        let mut output = String::new();
        let mut annotation_index = 0;
        let mut line_start = 0;

        // copy each source line and append its source-aligned annotations
        for line in source.split('\n') {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(line);

            let line_end = line_start + line.len() as u32;
            while let Some(annotation) = annotations.get(annotation_index) {
                if annotation.anchor > line_end {
                    break;
                }

                write!(output, "\n{}", annotation.render()).expect("write test match");
                annotation_index += 1;
            }

            line_start = line_end + 1;
        }

        output.trim_end().to_string()
    }

    /// Build source-aligned annotations for one exact match span.
    fn annotations(&self, pattern_match: &PatternMatch) -> Vec<MatchAnnotation> {
        let span = self.source.node_span(pattern_match.root);
        let start_line = self.source.line_start(span.start);
        let end_offset = span.end.saturating_sub(1);
        let end_line = self.source.line_start(end_offset);
        let bindings = self.format_bindings(pattern_match);

        // underline one-line roots directly
        if start_line == end_line {
            let annotation = MatchAnnotation {
                anchor: span.start,
                column: span.start - start_line,
                width: span.end.saturating_sub(span.start).max(1),
                text: format!("match{bindings}"),
            };

            return vec![annotation];
        }

        // mark both exact edges of multi-line roots
        let start = MatchAnnotation {
            anchor: span.start,
            column: span.start - start_line,
            width: 1,
            text: format!("match:start{bindings}"),
        };
        let end = MatchAnnotation {
            anchor: end_offset,
            column: end_offset - end_line,
            width: 1,
            text: "match:end".to_string(),
        };

        vec![start, end]
    }

    /// Render one match's bindings without unstable node ids.
    fn format_bindings(&self, pattern_match: &PatternMatch) -> String {
        let mut output = String::new();
        for (variable_id, variable) in self.pattern.metavariables().iter() {
            let name = self.pattern.strings().get(variable.name());
            let binding = pattern_match.bindings.values[variable_id.0 as usize]
                .as_ref()
                .expect("named metavariable binding");
            write!(output, " {name}.{}", self.format_binding(binding)).expect("write test binding");
        }

        output
    }

    /// Render one bound structural value.
    fn format_binding(&self, binding: &Binding) -> String {
        match binding {
            Binding::Node(node) => format!("node={:?}", self.source.node_text(*node)),
            Binding::Name { name, .. } => {
                format!("name={:?}", self.pattern.strings().get(*name))
            }
            Binding::Nodes(nodes) => {
                let nodes = nodes
                    .iter()
                    .map(|node| self.source.node_text(*node))
                    .collect::<Vec<_>>();

                format!("nodes={nodes:?}")
            }
        }
    }
}
