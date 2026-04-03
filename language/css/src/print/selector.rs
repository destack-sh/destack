use super::TokenRenderer;
use super::printer::Printer;
use crate::{
    AnySelector, AttributeSelector, Combinator, KeyframeSelector, KeyframeSelectorList,
    LocalNodeId, NthOfSelector, NthSelector, NthSelectorKind, PageMarginBox, PagePseudoClass,
    PageSelector, PageSelectorList, PseudoArgument, PseudoClass, PseudoElement, Selector,
    SelectorComponent, SelectorList, SimpleSelector, TimelineRangeName, VendorPrefix,
};

impl<'a> Printer<'a> {
    /// Render one selector list as canonical CSS source.
    pub(crate) fn render_selector_list(&self, selectors: LocalNodeId<SelectorList>) -> String {
        let selectors = self.tree.get(selectors);

        Self::join_sources(", ", &selectors.selectors, |selector| {
            self.render_selector(*selector)
        })
    }

    /// Render one selector as canonical CSS source.
    pub(crate) fn render_selector(&self, selector: LocalNodeId<Selector>) -> String {
        let selector = self.tree.get(selector);
        let mut source = String::new();

        self.write_selector(&mut source, selector);

        source
    }

    /// Render one keyframe selector list as canonical CSS source.
    pub(crate) fn render_keyframe_selector_list(selectors: &KeyframeSelectorList) -> String {
        Self::join_sources(", ", &selectors.selectors, Self::render_keyframe_selector)
    }

    /// Render one keyframe selector as canonical CSS source.
    pub(crate) fn render_keyframe_selector(selector: &KeyframeSelector) -> String {
        match selector {
            KeyframeSelector::Percentage(value) => TokenRenderer::render_percentage_source(*value),
            KeyframeSelector::From => "from".to_string(),
            KeyframeSelector::To => "to".to_string(),
            KeyframeSelector::TimelineRangePercentage(range) => format!(
                "{} {}",
                Self::render_timeline_range_name(range.name),
                Self::render_keyframe_selector(&KeyframeSelector::Percentage(range.percentage)),
            ),
        }
    }

    /// Write one selector into CSS source.
    pub(crate) fn write_selector(&self, source: &mut String, selector: &Selector) {
        for component_id in &selector.components {
            let component = self.tree.get(*component_id);
            self.write_selector_component(source, component);
        }
    }

    /// Write one selector component into CSS source.
    pub(crate) fn write_selector_component(
        &self,
        source: &mut String,
        component: &SelectorComponent,
    ) {
        match component {
            SelectorComponent::Combinator(combinator) => {
                source.push_str(match combinator {
                    Combinator::Child => " > ",
                    Combinator::Descendant => " ",
                    Combinator::NextSibling => " + ",
                    Combinator::LaterSibling => " ~ ",
                    Combinator::PseudoElement => "::",
                    Combinator::SlotAssignment => " /deep/ ",
                    Combinator::Part => "::part",
                    Combinator::DeepDescendant => " >>> ",
                    Combinator::Deep => " >> ",
                });
            }
            SelectorComponent::Simple(simple) => {
                let simple = self.tree.get(*simple);
                self.write_simple_selector(source, simple);
            }
        }
    }

    /// Write one simple selector into CSS source.
    pub(crate) fn write_simple_selector(&self, source: &mut String, selector: &SimpleSelector) {
        match selector {
            SimpleSelector::ExplicitAnyNamespace => source.push_str("*|"),
            SimpleSelector::ExplicitNoNamespace => source.push('|'),
            SimpleSelector::DefaultNamespace => {}
            SimpleSelector::Namespace(prefix) => {
                source.push_str(&self.render_identifier(prefix));
                source.push('|');
            }
            SimpleSelector::ExplicitUniversalType => source.push('*'),
            SimpleSelector::LocalName(name) => {
                source.push_str(&self.render_identifier(&name.name));
            }
            SimpleSelector::Id(value) => {
                source.push('#');
                source.push_str(&self.render_identifier(value));
            }
            SimpleSelector::Class(value) => {
                source.push('.');
                source.push_str(&self.render_identifier(value));
            }
            SimpleSelector::Attribute(selector) => {
                let selector = self.tree.get(*selector);
                self.write_attribute_selector(source, selector);
            }
            SimpleSelector::Negation(selectors) => {
                source.push_str(":not(");
                source.push_str(&self.render_selector_list(*selectors));
                source.push(')');
            }
            SimpleSelector::Root => source.push_str(":root"),
            SimpleSelector::Empty => source.push_str(":empty"),
            SimpleSelector::Scope => source.push_str(":scope"),
            SimpleSelector::Nth(selector) => {
                let selector = self.tree.get(*selector);
                Self::write_nth_selector(source, selector);
            }
            SimpleSelector::NthOf(selector) => {
                let selector = self.tree.get(*selector);
                self.write_nth_of_selector(source, selector);
            }
            SimpleSelector::PseudoClass(selector) => {
                let selector = self.tree.get(*selector);
                self.write_pseudo_class(source, selector);
            }
            SimpleSelector::Slotted(selector) => {
                source.push_str("::slotted(");
                let selector = self.tree.get(*selector);
                self.write_selector(source, selector);
                source.push(')');
            }
            SimpleSelector::Part(parts) => {
                source.push_str("::part(");
                for (index, part) in parts.iter().enumerate() {
                    if index > 0 {
                        source.push(' ');
                    }

                    source.push_str(&self.render_identifier(part));
                }
                source.push(')');
            }
            SimpleSelector::Host(selector) => {
                source.push_str(":host");
                if let Some(selector) = selector {
                    source.push('(');
                    let selector = self.tree.get(*selector);
                    self.write_selector(source, selector);
                    source.push(')');
                }
            }
            SimpleSelector::Where(selectors) => {
                source.push_str(":where(");
                source.push_str(&self.render_selector_list(*selectors));
                source.push(')');
            }
            SimpleSelector::Is(selectors) => {
                source.push_str(":is(");
                source.push_str(&self.render_selector_list(*selectors));
                source.push(')');
            }
            SimpleSelector::Any(selector) => {
                let selector = self.tree.get(*selector);
                self.write_any_selector(source, selector);
            }
            SimpleSelector::Has(selectors) => {
                source.push_str(":has(");
                source.push_str(&self.render_selector_list(*selectors));
                source.push(')');
            }
            SimpleSelector::PseudoElement(selector) => {
                let selector = self.tree.get(*selector);
                self.write_pseudo_element(source, selector);
            }
            SimpleSelector::Nesting => source.push('&'),
        }
    }
    /// Write one attribute selector into CSS source.
    pub(crate) fn write_attribute_selector(
        &self,
        source: &mut String,
        selector: &AttributeSelector,
    ) {
        source.push('[');
        self.write_component_value_list(source, &selector.components);
        source.push(']');
    }

    /// Write one nth selector into CSS source.
    pub(crate) fn write_nth_selector(source: &mut String, selector: &NthSelector) {
        source.push(':');
        source.push_str(match selector.kind {
            NthSelectorKind::Child if selector.is_function => "nth-child(",
            NthSelectorKind::Child => "first-child",
            NthSelectorKind::LastChild if selector.is_function => "nth-last-child(",
            NthSelectorKind::LastChild => "last-child",
            NthSelectorKind::OfType if selector.is_function => "nth-of-type(",
            NthSelectorKind::OfType => "first-of-type",
            NthSelectorKind::LastOfType if selector.is_function => "nth-last-of-type(",
            NthSelectorKind::LastOfType => "last-of-type",
            NthSelectorKind::OnlyChild => "only-child",
            NthSelectorKind::OnlyOfType => "only-of-type",
            NthSelectorKind::Column => "nth-col(",
            NthSelectorKind::LastColumn => "nth-last-col(",
        });

        if selector.is_function {
            source.push_str(&Self::render_affine(selector.a, selector.b));
            source.push(')');
        }
    }

    /// Write one nth-of selector into CSS source.
    pub(crate) fn write_nth_of_selector(&self, source: &mut String, selector: &NthOfSelector) {
        let nth = self.tree.get(selector.nth);

        source.push(':');
        source.push_str(match nth.kind {
            NthSelectorKind::Child => "nth-child(",
            NthSelectorKind::LastChild => "nth-last-child(",
            NthSelectorKind::OfType => "nth-of-type(",
            NthSelectorKind::LastOfType => "nth-last-of-type(",
            NthSelectorKind::OnlyChild => "only-child",
            NthSelectorKind::OnlyOfType => "only-of-type",
            NthSelectorKind::Column => "nth-col(",
            NthSelectorKind::LastColumn => "nth-last-col(",
        });
        source.push_str(&Self::render_affine(nth.a, nth.b));
        source.push_str(" of ");
        source.push_str(&self.render_selector_list(selector.selectors));
        source.push(')');
    }

    /// Write one pseudo class into CSS source.
    pub(crate) fn write_pseudo_class(&self, source: &mut String, selector: &PseudoClass) {
        source.push(':');
        source.push_str(&selector.name);

        if let Some(arguments) = &selector.arguments {
            source.push('(');
            self.write_pseudo_argument(source, arguments);
            source.push(')');
        }
    }

    /// Write one vendor `:any(...)` selector into CSS source.
    pub(crate) fn write_any_selector(&self, source: &mut String, selector: &AnySelector) {
        source.push(':');
        source.push_str(match selector.vendor_prefix {
            VendorPrefix::None => "",
            VendorPrefix::Webkit => "-webkit-",
            VendorPrefix::Moz => "-moz-",
            VendorPrefix::Ms => "-ms-",
            VendorPrefix::O => "-o-",
            VendorPrefix::Other(ref prefix) => prefix,
        });
        source.push_str("any(");
        source.push_str(&self.render_selector_list(selector.selectors));
        source.push(')');
    }

    /// Write one pseudo element into CSS source.
    pub(crate) fn write_pseudo_element(&self, source: &mut String, selector: &PseudoElement) {
        source.push_str(&selector.name);

        if let Some(arguments) = &selector.arguments {
            source.push('(');
            self.write_pseudo_argument(source, arguments);
            source.push(')');
        }
    }

    /// Write one pseudo argument payload into CSS source.
    pub(crate) fn write_pseudo_argument(&self, source: &mut String, argument: &PseudoArgument) {
        match argument {
            PseudoArgument::Components(arguments) => {
                self.write_component_value_list(source, arguments);
            }
            PseudoArgument::Selector(selector) => {
                let selector = self.tree.get(*selector);
                self.write_selector(source, selector);
            }
            PseudoArgument::ViewTransitionPart(argument) => {
                if let Some(name) = &argument.name {
                    source.push_str(name);
                }

                for class in &argument.classes {
                    source.push('.');
                    source.push_str(&self.render_identifier(class));
                }
            }
        }
    }

    /// Render one timeline range name as canonical CSS source.
    pub(crate) fn render_timeline_range_name(name: TimelineRangeName) -> &'static str {
        match name {
            TimelineRangeName::Cover => "cover",
            TimelineRangeName::Contain => "contain",
            TimelineRangeName::Entry => "entry",
            TimelineRangeName::Exit => "exit",
            TimelineRangeName::EntryCrossing => "entry-crossing",
            TimelineRangeName::ExitCrossing => "exit-crossing",
        }
    }

    /// Render one affine nth expression as canonical CSS source.
    pub(crate) fn render_affine(a: i32, b: i32) -> String {
        match (a, b) {
            (0, 0) => "0".to_string(),
            (1, 0) => "n".to_string(),
            (-1, 0) => "-n".to_string(),
            (_, 0) => format!("{a}n"),
            (2, 1) => "odd".to_string(),
            (0, _) => b.to_string(),
            (1, _) => format!("n{b:+}"),
            (-1, _) => format!("-n{b:+}"),
            (_, _) => format!("{a}n{b:+}"),
        }
    }

    /// Render one identifier as canonical CSS source.
    pub(crate) fn render_identifier(&self, value: &str) -> String {
        self.token_renderer().render_identifier(value)
    }

    /// Render one page selector list as canonical CSS source.
    pub(crate) fn render_page_selector_list(selectors: &PageSelectorList) -> String {
        Self::join_sources(", ", &selectors.selectors, Self::render_page_selector)
    }

    /// Render one page selector as canonical CSS source.
    pub(crate) fn render_page_selector(selector: &PageSelector) -> String {
        let mut source = String::new();

        if let Some(name) = &selector.name {
            source.push_str(name);
        }

        for pseudo_class in &selector.pseudo_classes {
            source.push(':');
            source.push_str(Self::render_page_pseudo_class(*pseudo_class));
        }

        source
    }

    /// Render one page pseudo class as canonical CSS source.
    pub(crate) fn render_page_pseudo_class(pseudo_class: PagePseudoClass) -> &'static str {
        match pseudo_class {
            PagePseudoClass::Left => "left",
            PagePseudoClass::Right => "right",
            PagePseudoClass::First => "first",
            PagePseudoClass::Last => "last",
            PagePseudoClass::Blank => "blank",
        }
    }

    /// Render one page margin box as canonical CSS source.
    pub(crate) fn render_page_margin_box(margin_box: PageMarginBox) -> &'static str {
        match margin_box {
            PageMarginBox::TopLeftCorner => "top-left-corner",
            PageMarginBox::TopLeft => "top-left",
            PageMarginBox::TopCenter => "top-center",
            PageMarginBox::TopRight => "top-right",
            PageMarginBox::TopRightCorner => "top-right-corner",
            PageMarginBox::LeftTop => "left-top",
            PageMarginBox::LeftMiddle => "left-middle",
            PageMarginBox::LeftBottom => "left-bottom",
            PageMarginBox::RightTop => "right-top",
            PageMarginBox::RightMiddle => "right-middle",
            PageMarginBox::RightBottom => "right-bottom",
            PageMarginBox::BottomLeftCorner => "bottom-left-corner",
            PageMarginBox::BottomLeft => "bottom-left",
            PageMarginBox::BottomCenter => "bottom-center",
            PageMarginBox::BottomRight => "bottom-right",
            PageMarginBox::BottomRightCorner => "bottom-right-corner",
        }
    }
}
