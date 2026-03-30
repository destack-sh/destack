use crate::lex::TagKind::StartTag;
use crate::lex::{
    Attribute, LocalName, Namespace, QualifiedName, Tag, expanded_name, local_name, ns,
};

use super::insert::PushFlag;
use super::parser::{Parser, ProcessResult};
use super::tag::{mathml_text_integration_point, svg_html_integration_point, tag};
use super::token::Token;

// qualified foreign names
macro_rules! qualified_name {
    ("", $local:tt) => {
        QualifiedName {
            prefix: None,
            ns: $crate::lex::ns!(),
            local: $crate::lex::local_name!($local),
        }
    };
    ($prefix: tt $ns:tt $local:tt) => {
        QualifiedName {
            prefix: Some($crate::lex::namespace_prefix!($prefix)),
            ns: $crate::lex::ns!($ns),
            local: $crate::lex::local_name!($local),
        }
    };
}

impl Parser<'_> {
    /// Process one foreign-content token.
    pub(crate) fn step_foreign(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            // text and comments
            Token::NullCharacter => {
                self.unexpected(&token);
                self.append_text("\u{fffd}".into())
            }
            Token::Characters(_, text) => {
                if text
                    .chars()
                    .any(|character: char| !character.is_ascii_whitespace())
                {
                    self.frameset_ok.set(false);
                }

                self.append_text(text)
            }
            Token::Comment(text) => self.append_comment(text),

            // html-in-foreign fallback
            Token::Tag(
                tag @
                tag!(<b> | <big> | <blockquote> | <body> | <br> | <center> | <code> | <dd> | <div> | <dl> |
                    <dt> | <em> | <embed> | <h1> | <h2> | <h3> | <h4> | <h5> | <h6> | <head> | <hr> | <i> |
                    <img> | <li> | <listing> | <menu> | <meta> | <nobr> | <ol> | <p> | <pre> | <ruby> |
                    <s> | <small> | <span> | <strong> | <strike> | <sub> | <sup> | <table> | <tt> |
                    <u> | <ul> | <var> | </br> | </p>),
            ) => self.unexpected_start_tag_in_foreign_content(tag),
            Token::Tag(tag @ tag!(<font>)) => {
                let unexpected = tag.attrs.iter().any(|attribute| {
                    matches!(
                        attribute.name.expanded(),
                        expanded_name!("", "color")
                            | expanded_name!("", "face")
                            | expanded_name!("", "size")
                    )
                });

                if unexpected {
                    self.unexpected_start_tag_in_foreign_content(tag)
                } else {
                    self.foreign_start_tag(tag)
                }
            }
            Token::Tag(tag @ tag!(<>)) => self.foreign_start_tag(tag),

            // foreign end tags
            Token::Tag(tag @ tag!(</>)) => {
                let mut first = true;
                let mut stack_index = self.open_elements.borrow().len() - 1;

                loop {
                    if stack_index == 0 {
                        return ProcessResult::Done;
                    }

                    let html;
                    let equal;

                    {
                        let open_elements = self.open_elements.borrow();
                        let Some(node_name) = self.open_element_name(&open_elements[stack_index])
                        else {
                            return ProcessResult::Done;
                        };
                        html = *node_name.ns() == ns!(html);
                        equal = node_name.local_name().eq_ignore_ascii_case(&tag.name);
                    }

                    if !first && html {
                        return self.step(self.mode.get(), Token::Tag(tag));
                    }

                    if equal {
                        self.open_elements.borrow_mut().truncate(stack_index);
                        return ProcessResult::Done;
                    }

                    if first {
                        self.unexpected(&tag);
                        first = false;
                    }

                    stack_index -= 1;
                }
            }
            Token::Eof => ProcessResult::Done,
        }
    }

    /// Return whether the current token should be processed in foreign content.
    pub(super) fn is_foreign(&self, token: &Token) -> bool {
        // eof and empty stack stay in html mode
        if matches!(token, Token::Eof) || self.open_elements.borrow().is_empty() {
            return false;
        }

        let Some(current) = self.adjusted_current_node() else {
            return false;
        };
        let Some(current_name) = self.open_element_name(&current) else {
            return false;
        };
        let name = current_name.expanded();

        // html namespace stays in html mode
        if let ns!(html) = *name.ns {
            return false;
        }

        // MathML text integration points hand selected tokens back to html mode
        if mathml_text_integration_point(name) {
            match *token {
                Token::Characters(..) | Token::NullCharacter => return false,
                Token::Tag(Tag {
                    kind: StartTag,
                    ref name,
                    ..
                }) if !matches!(*name, local_name!("mglyph") | local_name!("malignmark")) => {
                    return false;
                }
                _ => {}
            }
        }

        // SVG integration points hand text and start tags back to html mode
        if svg_html_integration_point(name) {
            match *token {
                Token::Characters(..)
                | Token::NullCharacter
                | Token::Tag(Tag { kind: StartTag, .. }) => {
                    return false;
                }
                _ => {}
            }
        }

        // annotation-xml has one special svg escape hatch
        if let expanded_name!(mathml "annotation-xml") = name {
            match *token {
                Token::Tag(Tag {
                    kind: StartTag,
                    name: local_name!("svg"),
                    ..
                }) => return false,
                Token::Characters(..)
                | Token::NullCharacter
                | Token::Tag(Tag { kind: StartTag, .. }) => {
                    return !self
                        .builder
                        .is_mathml_annotation_xml_integration_point(&current);
                }
                _ => {}
            }
        }

        true
    }

    /// Enter one foreign namespace from HTML content.
    pub(super) fn enter_foreign(
        &self,
        mut tag: Tag,
        namespace: Namespace,
    ) -> ProcessResult<super::builder::Handle> {
        // namespace-specific rewrites
        match namespace {
            ns!(mathml) => self.rewrite_mathml_attribute_names(&mut tag),
            ns!(svg) => self.rewrite_svg_attribute_names(&mut tag),
            _ => {}
        }

        // cross-namespace attribute rewrites
        self.rewrite_foreign_attribute_names(&mut tag);

        // insertion
        if tag.self_closing {
            self.insert_element(
                PushFlag::NoPush,
                namespace,
                tag.name,
                tag.attrs,
                tag.had_duplicate_attributes,
            );
            ProcessResult::DoneAckSelfClosing
        } else {
            self.insert_element(
                PushFlag::Push,
                namespace,
                tag.name,
                tag.attrs,
                tag.had_duplicate_attributes,
            );
            ProcessResult::Done
        }
    }

    /// Start one foreign-content tag in the current namespace.
    pub(super) fn foreign_start_tag(&self, mut tag: Tag) -> ProcessResult<super::builder::Handle> {
        let Some(current) = self.adjusted_current_node() else {
            return self.unexpected(&tag);
        };
        let Some(current_name) = self.open_element_name(&current) else {
            return self.unexpected(&tag);
        };
        let current_namespace = *current_name.ns();

        // namespace-specific rewrites
        match current_namespace {
            ns!(mathml) => self.rewrite_mathml_attribute_names(&mut tag),
            ns!(svg) => {
                self.rewrite_svg_tag_name(&mut tag);
                self.rewrite_svg_attribute_names(&mut tag);
            }
            _ => {}
        }

        // cross-namespace attribute rewrites
        self.rewrite_foreign_attribute_names(&mut tag);

        // insertion
        if tag.self_closing {
            self.insert_element(
                PushFlag::NoPush,
                current_namespace,
                tag.name,
                tag.attrs,
                tag.had_duplicate_attributes,
            );
            ProcessResult::DoneAckSelfClosing
        } else {
            self.insert_element(
                PushFlag::Push,
                current_namespace,
                tag.name,
                tag.attrs,
                tag.had_duplicate_attributes,
            );
            ProcessResult::Done
        }
    }

    /// Reprocess one unexpected foreign-content start tag in HTML mode.
    pub(super) fn unexpected_start_tag_in_foreign_content(
        &self,
        tag: Tag,
    ) -> ProcessResult<super::builder::Handle> {
        self.unexpected(&tag);

        // pop foreign elements until html mode applies again
        while !self.current_node_in(|name| {
            *name.ns == ns!(html)
                || mathml_text_integration_point(name)
                || svg_html_integration_point(name)
        }) {
            if self.pop().is_none() {
                return ProcessResult::Done;
            }
        }

        self.step(self.mode.get(), Token::Tag(tag))
    }

    fn rewrite_svg_tag_name(&self, tag: &mut Tag) {
        let Tag { ref mut name, .. } = *tag;

        match *name {
            local_name!("altglyph") => *name = local_name!("altGlyph"),
            local_name!("altglyphdef") => *name = local_name!("altGlyphDef"),
            local_name!("altglyphitem") => *name = local_name!("altGlyphItem"),
            local_name!("animatecolor") => *name = local_name!("animateColor"),
            local_name!("animatemotion") => *name = local_name!("animateMotion"),
            local_name!("animatetransform") => *name = local_name!("animateTransform"),
            local_name!("clippath") => *name = local_name!("clipPath"),
            local_name!("feblend") => *name = local_name!("feBlend"),
            local_name!("fecolormatrix") => *name = local_name!("feColorMatrix"),
            local_name!("fecomponenttransfer") => *name = local_name!("feComponentTransfer"),
            local_name!("fecomposite") => *name = local_name!("feComposite"),
            local_name!("feconvolvematrix") => *name = local_name!("feConvolveMatrix"),
            local_name!("fediffuselighting") => *name = local_name!("feDiffuseLighting"),
            local_name!("fedisplacementmap") => *name = local_name!("feDisplacementMap"),
            local_name!("fedistantlight") => *name = local_name!("feDistantLight"),
            local_name!("fedropshadow") => *name = local_name!("feDropShadow"),
            local_name!("feflood") => *name = local_name!("feFlood"),
            local_name!("fefunca") => *name = local_name!("feFuncA"),
            local_name!("fefuncb") => *name = local_name!("feFuncB"),
            local_name!("fefuncg") => *name = local_name!("feFuncG"),
            local_name!("fefuncr") => *name = local_name!("feFuncR"),
            local_name!("fegaussianblur") => *name = local_name!("feGaussianBlur"),
            local_name!("feimage") => *name = local_name!("feImage"),
            local_name!("femerge") => *name = local_name!("feMerge"),
            local_name!("femergenode") => *name = local_name!("feMergeNode"),
            local_name!("femorphology") => *name = local_name!("feMorphology"),
            local_name!("feoffset") => *name = local_name!("feOffset"),
            local_name!("fepointlight") => *name = local_name!("fePointLight"),
            local_name!("fespecularlighting") => *name = local_name!("feSpecularLighting"),
            local_name!("fespotlight") => *name = local_name!("feSpotLight"),
            local_name!("fetile") => *name = local_name!("feTile"),
            local_name!("feturbulence") => *name = local_name!("feTurbulence"),
            local_name!("foreignobject") => *name = local_name!("foreignObject"),
            local_name!("glyphref") => *name = local_name!("glyphRef"),
            local_name!("lineargradient") => *name = local_name!("linearGradient"),
            local_name!("radialgradient") => *name = local_name!("radialGradient"),
            local_name!("textpath") => *name = local_name!("textPath"),
            _ => {}
        }
    }

    fn rewrite_attribute_names<F>(&self, tag: &mut Tag, mut map: F)
    where
        F: FnMut(LocalName) -> Option<QualifiedName>,
    {
        for &mut Attribute { ref mut name, .. } in &mut tag.attrs {
            if let Some(replacement) = map(name.local) {
                *name = replacement;
            }
        }
    }

    fn rewrite_svg_attribute_names(&self, tag: &mut Tag) {
        self.rewrite_attribute_names(tag, |key| match key {
            local_name!("attributename") => Some(qualified_name!("", "attributeName")),
            local_name!("attributetype") => Some(qualified_name!("", "attributeType")),
            local_name!("basefrequency") => Some(qualified_name!("", "baseFrequency")),
            local_name!("baseprofile") => Some(qualified_name!("", "baseProfile")),
            local_name!("calcmode") => Some(qualified_name!("", "calcMode")),
            local_name!("clippathunits") => Some(qualified_name!("", "clipPathUnits")),
            local_name!("diffuseconstant") => Some(qualified_name!("", "diffuseConstant")),
            local_name!("edgemode") => Some(qualified_name!("", "edgeMode")),
            local_name!("filterunits") => Some(qualified_name!("", "filterUnits")),
            local_name!("glyphref") => Some(qualified_name!("", "glyphRef")),
            local_name!("gradienttransform") => Some(qualified_name!("", "gradientTransform")),
            local_name!("gradientunits") => Some(qualified_name!("", "gradientUnits")),
            local_name!("kernelmatrix") => Some(qualified_name!("", "kernelMatrix")),
            local_name!("kernelunitlength") => Some(qualified_name!("", "kernelUnitLength")),
            local_name!("keypoints") => Some(qualified_name!("", "keyPoints")),
            local_name!("keysplines") => Some(qualified_name!("", "keySplines")),
            local_name!("keytimes") => Some(qualified_name!("", "keyTimes")),
            local_name!("lengthadjust") => Some(qualified_name!("", "lengthAdjust")),
            local_name!("limitingconeangle") => Some(qualified_name!("", "limitingConeAngle")),
            local_name!("markerheight") => Some(qualified_name!("", "markerHeight")),
            local_name!("markerunits") => Some(qualified_name!("", "markerUnits")),
            local_name!("markerwidth") => Some(qualified_name!("", "markerWidth")),
            local_name!("maskcontentunits") => Some(qualified_name!("", "maskContentUnits")),
            local_name!("maskunits") => Some(qualified_name!("", "maskUnits")),
            local_name!("numoctaves") => Some(qualified_name!("", "numOctaves")),
            local_name!("pathlength") => Some(qualified_name!("", "pathLength")),
            local_name!("patterncontentunits") => Some(qualified_name!("", "patternContentUnits")),
            local_name!("patterntransform") => Some(qualified_name!("", "patternTransform")),
            local_name!("patternunits") => Some(qualified_name!("", "patternUnits")),
            local_name!("pointsatx") => Some(qualified_name!("", "pointsAtX")),
            local_name!("pointsaty") => Some(qualified_name!("", "pointsAtY")),
            local_name!("pointsatz") => Some(qualified_name!("", "pointsAtZ")),
            local_name!("preservealpha") => Some(qualified_name!("", "preserveAlpha")),
            local_name!("preserveaspectratio") => Some(qualified_name!("", "preserveAspectRatio")),
            local_name!("primitiveunits") => Some(qualified_name!("", "primitiveUnits")),
            local_name!("refx") => Some(qualified_name!("", "refX")),
            local_name!("refy") => Some(qualified_name!("", "refY")),
            local_name!("repeatcount") => Some(qualified_name!("", "repeatCount")),
            local_name!("repeatdur") => Some(qualified_name!("", "repeatDur")),
            local_name!("requiredextensions") => Some(qualified_name!("", "requiredExtensions")),
            local_name!("requiredfeatures") => Some(qualified_name!("", "requiredFeatures")),
            local_name!("specularconstant") => Some(qualified_name!("", "specularConstant")),
            local_name!("specularexponent") => Some(qualified_name!("", "specularExponent")),
            local_name!("spreadmethod") => Some(qualified_name!("", "spreadMethod")),
            local_name!("startoffset") => Some(qualified_name!("", "startOffset")),
            local_name!("stddeviation") => Some(qualified_name!("", "stdDeviation")),
            local_name!("stitchtiles") => Some(qualified_name!("", "stitchTiles")),
            local_name!("surfacescale") => Some(qualified_name!("", "surfaceScale")),
            local_name!("systemlanguage") => Some(qualified_name!("", "systemLanguage")),
            local_name!("tablevalues") => Some(qualified_name!("", "tableValues")),
            local_name!("targetx") => Some(qualified_name!("", "targetX")),
            local_name!("targety") => Some(qualified_name!("", "targetY")),
            local_name!("textlength") => Some(qualified_name!("", "textLength")),
            local_name!("viewbox") => Some(qualified_name!("", "viewBox")),
            local_name!("viewtarget") => Some(qualified_name!("", "viewTarget")),
            local_name!("xchannelselector") => Some(qualified_name!("", "xChannelSelector")),
            local_name!("ychannelselector") => Some(qualified_name!("", "yChannelSelector")),
            local_name!("zoomandpan") => Some(qualified_name!("", "zoomAndPan")),
            _ => None,
        });
    }

    fn rewrite_mathml_attribute_names(&self, tag: &mut Tag) {
        self.rewrite_attribute_names(tag, |key| match key {
            local_name!("definitionurl") => Some(qualified_name!("", "definitionURL")),
            _ => None,
        });
    }

    fn rewrite_foreign_attribute_names(&self, tag: &mut Tag) {
        self.rewrite_attribute_names(tag, |key| match key {
            local_name!("xlink:actuate") => Some(qualified_name!("xlink" xlink "actuate")),
            local_name!("xlink:arcrole") => Some(qualified_name!("xlink" xlink "arcrole")),
            local_name!("xlink:href") => Some(qualified_name!("xlink" xlink "href")),
            local_name!("xlink:role") => Some(qualified_name!("xlink" xlink "role")),
            local_name!("xlink:show") => Some(qualified_name!("xlink" xlink "show")),
            local_name!("xlink:title") => Some(qualified_name!("xlink" xlink "title")),
            local_name!("xlink:type") => Some(qualified_name!("xlink" xlink "type")),
            local_name!("xml:lang") => Some(qualified_name!("xml" xml "lang")),
            local_name!("xml:space") => Some(qualified_name!("xml" xml "space")),
            local_name!("xmlns") => Some(qualified_name!("" xmlns "xmlns")),
            local_name!("xmlns:xlink") => Some(qualified_name!("xmlns" xmlns "xlink")),
            _ => None,
        });
    }
}
