use crate::tests::{DirRows, TestSession};

/// A tree fragment checks against the builder its expected type names.
#[test]
fn test_check_a_tree_fragment_against_the_contextual_builder() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <><span/><span/></>;
    return page;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = (
        <>
            <span />
            <span />
        </>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <><span/><span/></>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source=<><span/><span/></> builder=Panel form=fragment call=fragment children=(Panel, Panel) type=Panel
    /// @generic.instantiation id="fragment<(Panel, Panel)>" template=fragment arguments=((Panel, Panel))
    /// @generic.instance id="fragment<(Panel, Panel)>" template=fragment arguments=((Panel, Panel))
    /// @resolution.tree source=<span/> builder=Panel form=element tag=span call=element children=() type=Panel
    /// @generic.instantiation id="element<\"span\", ()>" template=element arguments=("span", ())
    /// @generic.instance id="element<\"span\", ()>" template=element arguments=("span", ()) dependents=({})
    /// @resolution.tree source=<span/> builder=Panel form=element tag=span call=element children=() type=Panel

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A tree element checks its text children and its expression children.
#[test]
fn test_check_text_and_expression_tree_children() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(title: string): Panel {
    const page: Panel = <div>hello<span/>{title}</div>;
    return page;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(title: string): Panel {
    const page: Panel = (
        <div>
            hello<span />
            {title}
        </div>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(title: string): Panel {
/// @type.symbol symbol=render type=(string) => Panel
/// @type.symbol symbol=render.title source="title: string" type=string
/// @resolution.name source=Panel target=Panel

    const page: Panel = <div>hello<span/>{title}</div>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source=<div>hello<span/>{title}</div> builder=Panel form=element tag=div call=element children=(string, Panel, string) type=Panel
    /// @generic.instantiation id="element<\"div\", (string, Panel, string)>" template=element arguments=("div", (string, Panel, string))
    /// @generic.instance id="element<\"div\", (string, Panel, string)>" template=element arguments=("div", (string, Panel, string)) dependents=({ class?: string })
    /// @resolution.tree source=<span/> builder=Panel form=element tag=span call=element children=() type=Panel
    /// @generic.instantiation id="element<\"span\", ()>" template=element arguments=("span", ())
    /// @generic.instance id="element<\"span\", ()>" template=element arguments=("span", ()) dependents=({})
    /// @resolution.name source=title target=render.title
    /// @resolution.place source=title placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=title root=render.title

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A tag outside the builder's declared tags reports a diagnostic.
#[test]
fn test_reject_an_unknown_tree_tag() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <blink/>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <blink />;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <blink/>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.rejected source=<blink/>

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
        r#"
/// @diagnostic.error id=unknown-tree-tag message="builder 'Panel' declares no 'blink' tag"
/// @diagnostic.label line=29 column=25 span="<blink/>" line_source="const page: Panel = <blink/>;"
"#,
    );
}

/// An attribute outside the tag's declared row reports a diagnostic.
#[test]
fn test_reject_an_unknown_tree_attribute() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <div misspelled="1"/>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <div misspelled="1" />;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <div misspelled="1"/>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.rejected source="<div misspelled=\"1\"/>"

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#, r#"
/// @diagnostic.error id=unknown-tree-attribute message="attribute row '{ div: { class?: string }; img: { src: string }; span: {} }[\"div\"]' declares no 'misspelled' attribute"
/// @diagnostic.label line=29 column=25 span="<div misspelled=\"1\"/>" line_source="const page: Panel = <div misspelled=\"1\"/>;"
"#);
}

/// A tag missing a required attribute reports a diagnostic.
#[test]
fn test_require_a_missing_tree_attribute() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <img/>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <img />;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <img/>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.rejected source=<img/>

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
        r#"
/// @diagnostic.error id=missing-tree-attribute message="required attribute 'src' of row '{ div: { class?: string }; img: { src: string }; span: {} }[\"img\"]' is missing"
/// @diagnostic.label line=29 column=25 span="<img/>" line_source="const page: Panel = <img/>;"
"#,
    );
}

/// An attribute value of another type reports a diagnostic.
#[test]
fn test_reject_a_mismatched_tree_attribute_value() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <img src={1}/>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = <img src={1} />;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <img src={1}/>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source="<img src={1}/>" builder=Panel form=element tag=img call=element attributes=(src: 1) children=() type=Panel
    /// @generic.instantiation id="element<\"img\", ()>" template=element arguments=("img", ())

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#, r#"
/// @diagnostic.error id=not-assignable message="type '1' is not assignable to type 'string'"
/// @diagnostic.label line=29 column=25 span="<img src={1}/>" line_source="const page: Panel = <img src={1}/>;"
/// @diagnostic.related line=29 column=35 span="1" line_source="const page: Panel = <img src={1}/>;" message="expected due to the type of this target"
"#);
}

/// A spread attribute checks against the tag's declared row.
#[test]
fn test_spread_tree_attributes_into_the_declared_row() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const shared = { src: "logo.png" };
    const page: Panel = <img {...shared}/>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const shared: { src: string } = { src: "logo.png" };
    const page: Panel = <img {...shared} />;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const shared = { src: "logo.png" };
    /// @type.symbol symbol=render.shared source=shared type={ src: string }
    /// @resolution.pattern source=shared kind=binding target=render.shared

    const page: Panel = <img {...shared}/>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source="<img {...shared}/>" builder=Panel form=element tag=img call=element attributes=(...: { src: string }) children=() type=Panel
    /// @generic.instantiation id="element<\"img\", ()>" template=element arguments=("img", ())
    /// @resolution.name source=shared target=render.shared
    /// @resolution.place source=shared placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=shared root=render.shared

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
        r#"
"#,
    );
}

/// A component tag checks its attributes against the component's props.
#[test]
fn test_check_a_component_tree_tag_with_props() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function Header(props: { title: string }): Panel {
    return new Panel();
}

function render(): Panel {
    const page: Panel = <Header title="hello"/>;
    return page;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function Header(props: { title: string }): Panel {
    return new Panel();
}

function render(): Panel {
    const page: Panel = <Header title="hello" />;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function Header(props: { title: string }): Panel {
/// @type.symbol symbol=Header type=({ title: string }) => Panel
/// @type.symbol symbol=Header.props source="props: { title: string }" type={ title: string }
/// @type.symbol symbol=Header.title source="title: string" type=string
/// @resolution.name source=Panel target=Panel

    return new Panel();
    /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
    /// @resolution.name source=Panel target=Panel

}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <Header title="hello"/>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source="<Header title=\"hello\"/>" builder=Panel form=component callee=Header call=Header attributes=(title: "hello") children=() type=Panel
    /// @resolution.name source=Header target=Header
    /// @resolution.function source=Header type=Function<({ title: string },), Panel, "readonly"> target=Header

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A spread child from a tuple splats into the children the builder takes.
#[test]
fn test_splat_spread_tree_children_from_a_tuple() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const pair: (Panel, Panel) = (<span/>, <span/>,);
    const page: Panel = <div>{...pair}</div>;
    return page;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const pair: (Panel, Panel) = (<span />, <span />);
    const page: Panel = <div>{...pair}</div>;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const pair: (Panel, Panel) = (<span/>, <span/>,);
    /// @type.symbol symbol=render.pair source=pair type=(Panel, Panel)
    /// @resolution.pattern source=pair kind=binding target=render.pair
    /// @resolution.name source=Panel target=Panel
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source=<span/> builder=Panel form=element tag=span call=element children=() type=Panel
    /// @generic.instantiation id="element<\"span\", ()>" template=element arguments=("span", ())
    /// @generic.instance id="element<\"span\", ()>" template=element arguments=("span", ()) dependents=({})
    /// @resolution.tree source=<span/> builder=Panel form=element tag=span call=element children=() type=Panel

    const page: Panel = <div>{...pair}</div>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source=<div>{...pair}</div> builder=Panel form=element tag=div call=element children=((Panel, Panel)) type=Panel
    /// @generic.instantiation id="element<\"div\", (Panel, Panel)>" template=element arguments=("div", (Panel, Panel))
    /// @generic.instance id="element<\"div\", (Panel, Panel)>" template=element arguments=("div", (Panel, Panel)) dependents=({ class?: string })
    /// @resolution.name source=pair target=render.pair
    /// @resolution.place source=pair placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=pair root=render.pair

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A spread child from a dynamic array reports a diagnostic.
#[test]
fn test_reject_spread_tree_children_from_a_dynamic_array() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(items: Panel[]): Panel {
    const page: Panel = <div>{...items}</div>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(items: Panel[]): Panel {
    const page: Panel = <div>{...items}</div>;
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(items: Panel[]): Panel {
/// @type.symbol symbol=render type=(Panel[]) => Panel
/// @type.symbol symbol=render.items source="items: Panel[]" type=Panel[]
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <div>{...items}</div>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.rejected source=<div>{...items}</div>
    /// @resolution.name source=items target=render.items
    /// @resolution.place source=items placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=items root=render.items

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#, r#"
/// @diagnostic.error id=tree-spread-not-tuple message="spread children splat tuples, found 'Panel[]'"
/// @diagnostic.label line=29 column=25 span="<div>{...items}</div>" line_source="const page: Panel = <div>{...items}</div>;"
"#);
}

/// A component checks its children against its children prop.
#[test]
fn test_check_component_children_through_the_children_prop() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function Stack(props: { title: string; children: (Panel,) }): Panel {
    return new Panel();
}

function render(): Panel {
    const page: Panel = <Stack title="hello"><span/></Stack>;
    return page;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function Stack(props: { title: string; children: (Panel,) }): Panel {
    return new Panel();
}

function render(): Panel {
    const page: Panel = (
        <Stack title="hello">
            <span />
        </Stack>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function Stack(props: { title: string; children: (Panel,) }): Panel {
/// @type.symbol symbol=Stack type=({ title: string; children: (Panel,) }) => Panel
/// @type.symbol symbol=Stack.props source="props: { title: string; children: (Panel,) }" type={ title: string; children: (Panel,) }
/// @type.symbol symbol=Stack.title source="title: string" type=string
/// @type.symbol symbol=Stack.children source="children: (Panel,)" type=(Panel,)
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=Panel target=Panel

    return new Panel();
    /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
    /// @resolution.name source=Panel target=Panel

}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <Stack title="hello"><span/></Stack>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source="<Stack title=\"hello\"><span/></Stack>" builder=Panel form=component callee=Stack call=Stack attributes=(title: "hello") children=(Panel) type=Panel
    /// @resolution.name source=Stack target=Stack
    /// @resolution.function source=Stack type=Function<({ title: string; children: (Panel,) },), Panel, "readonly"> target=Stack
    /// @resolution.tree source=<span/> builder=Panel form=element tag=span call=element children=() type=Panel
    /// @generic.instantiation id="element<\"span\", ()>" template=element arguments=("span", ())
    /// @generic.instance id="element<\"span\", ()>" template=element arguments=("span", ()) dependents=({})

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A component without a children prop reports a diagnostic for its children.
#[test]
fn test_reject_component_children_without_a_children_prop() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function Header(props: { title: string }): Panel {
    return new Panel();
}

function render(): Panel {
    const page: Panel = <Header title="hello"><span/></Header>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function Header(props: { title: string }): Panel {
    return new Panel();
}

function render(): Panel {
    const page: Panel = (
        <Header title="hello">
            <span />
        </Header>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function Header(props: { title: string }): Panel {
/// @type.symbol symbol=Header type=({ title: string }) => Panel
/// @type.symbol symbol=Header.props source="props: { title: string }" type={ title: string }
/// @type.symbol symbol=Header.title source="title: string" type=string
/// @resolution.name source=Panel target=Panel

    return new Panel();
    /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
    /// @resolution.name source=Panel target=Panel

}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <Header title="hello"><span/></Header>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.rejected source="<Header title=\"hello\"><span/></Header>"
    /// @resolution.name source=Header target=Header
    /// @resolution.function source=Header type=Function<({ title: string },), Panel, "readonly"> target=Header
    /// @resolution.tree source=<span/> builder=Panel form=element tag=span call=element children=() type=Panel
    /// @generic.instantiation id="element<\"span\", ()>" template=element arguments=("span", ())

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#, r#"
/// @diagnostic.error id=unknown-tree-attribute message="attribute row '{ title: string }' declares no 'children' attribute"
/// @diagnostic.label line=33 column=25 span="<Header title=\"hello\"><span/></Header>" line_source="const page: Panel = <Header title=\"hello\"><span/></Header>;"
"#);
}

/// A class component constructs through its constructor.
#[test]
fn test_construct_a_class_component_through_its_constructor() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

class Card {
    heading: string;

    constructor(props: { heading: string }) {
        this.heading = props.heading;
    }
}

function render(): Panel {
    const page: Panel = <><Card heading="hi"/></>;
    return page;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

class Card {
    heading: string;

    constructor(props: { heading: string }) {
        this.heading = props.heading;
    }
}

function render(): Panel {
    const page: Panel = (
        <>
            <Card heading="hi" />
        </>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

class Card {
/// @type.symbol symbol=Card type=typeof Card
/// @definition.class symbol=Card
/// @definition.field symbol=Card.heading source="heading: string" key=heading type=string
/// @definition.method symbol=Card.constructor slot=constructor role=constructor type=(this: &'managed Card, { heading: string }) => Card

    heading: string;
    /// @type.symbol symbol=Card.heading source="heading: string" type=string

    constructor(props: { heading: string }) {
    /// @type.symbol symbol=Card.constructor type=(this: &'managed Card, { heading: string }) => Card
    /// @type.symbol symbol=Card.constructor.this type=&'managed Card
    /// @type.symbol symbol=Card.constructor.props source="props: { heading: string }" type={ heading: string }
    /// @type.symbol symbol=Card.constructor.heading source="heading: string" type=string

        this.heading = props.heading;
        /// @resolution.receiver source=this kind=this declaration=Card type=&'managed Card
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.heading kind=place
        /// @resolution.place source=this.heading placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.heading root=this keys=[heading]
        /// @resolution.assignment source=this.heading write="receiver=&'managed Card, target=field(receiver=&'managed Card, target=Card.heading, type=string), type=string" type=string
        /// @resolution.name source=props target=Card.constructor.props
        /// @resolution.member source=props.heading receiver={ heading: string } type=string kind=field target_receiver={ heading: string } key=heading target_type=string
        /// @resolution.place source=props placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=props root=Card.constructor.props
        /// @resolution.place source=props.heading placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=props.heading root=Card.constructor.props keys=[heading]

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <><Card heading="hi"/></>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source="<><Card heading=\"hi\"/></>" builder=Panel form=fragment call=fragment children=(Card) type=Panel
    /// @generic.instantiation id=fragment<(Card,)> template=fragment arguments=((Card,))
    /// @generic.instance id=fragment<(Card,)> template=fragment arguments=((Card,))
    /// @resolution.tree source="<Card heading=\"hi\"/>" builder=Panel form=component callee=Card construct=Card attributes=(heading: "hi") children=() type=Card
    /// @resolution.name source=Card target=Card

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A struct component constructs through its field literal.
#[test]
fn test_construct_a_struct_component_through_its_field_form() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

struct Badge {
    label: string;
}

function render(): Panel {
    const page: Panel = <><Badge label="new"/></>;
    return page;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

struct Badge {
    label: string;
}

function render(): Panel {
    const page: Panel = (
        <>
            <Badge label="new" />
        </>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

struct Badge {
/// @type.symbol symbol=Badge type=Badge
/// @definition.struct symbol=Badge
/// @definition.field symbol=Badge.label source="label: string" key=label type=string

    label: string;
    /// @type.symbol symbol=Badge.label source="label: string" type=string

}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <><Badge label="new"/></>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source="<><Badge label=\"new\"/></>" builder=Panel form=fragment call=fragment children=(Badge) type=Panel
    /// @generic.instantiation id=fragment<(Badge,)> template=fragment arguments=((Badge,))
    /// @generic.instance id=fragment<(Badge,)> template=fragment arguments=((Badge,))
    /// @resolution.tree source="<Badge label=\"new\"/>" builder=Panel form=component callee=Badge struct=Badge attributes=(label: "new") children=() type=Badge
    /// @resolution.name source=Badge target=Badge

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A struct component missing a required field reports a diagnostic.
#[test]
fn test_require_a_missing_struct_component_field() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

struct Badge {
    label: string;
}

function render(): Panel {
    const page: Panel = <><Badge/></>;
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
        img: { src: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; img: { src: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

struct Badge {
    label: string;
}

function render(): Panel {
    const page: Panel = (
        <>
            <Badge />
        </>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; img: { src: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; img: { src: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        img: { src: string };
        /// @type.symbol symbol=Tags.img source="img: { src: string }" type={ src: string }
        /// @type.symbol symbol=Tags.src source="src: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; img: { src: string }; span: {} }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; img: { src: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

struct Badge {
/// @type.symbol symbol=Badge type=Badge
/// @definition.struct symbol=Badge
/// @definition.field symbol=Badge.label source="label: string" key=label type=string

    label: string;
    /// @type.symbol symbol=Badge.label source="label: string" type=string

}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = <><Badge/></>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source=<><Badge/></> builder=Panel form=fragment call=fragment children=(<error>) type=Panel
    /// @resolution.rejected source=<Badge/>
    /// @resolution.name source=Badge target=Badge

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#, r#"
/// @diagnostic.error id=missing-tree-attribute message="required attribute 'label' of row 'Badge' is missing"
/// @diagnostic.label line=33 column=27 span="<Badge/>" line_source="const page: Panel = <><Badge/></>;"
"#);
}

/// A tree literal without an expected type checks through the default builder.
#[test]
fn test_check_a_tree_literal_through_the_default_builder() {
    let session = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "tree": "panel.tspp#Panel"
    }
}
"#,
        )
        .module(
            "panel.tspp",
            r#"
import { TreeBuilder } from "tspp:tree";

export class Panel {
    label: string = "";
}

export extension of Panel implements TreeBuilder {
    type Tags = {
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Panel } from "./panel.tspp";

function render(): Panel {
    const page = <span/>;
    return page;
}
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Panel } from "./panel.tspp";

function render(): Panel {
    const page: Panel = <span />;
    return page;
}

=== dir ===
import { Panel } from "./panel.tspp";

function render(): Panel {
/// @type.symbol symbol=render type=() => panel.Panel
/// @resolution.name source=Panel target=panel.Panel

    const page = <span/>;
    /// @type.symbol symbol=render.page source=page type=panel.Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.tree source=<span/> builder=panel.Panel form=element tag=span call=panel.element children=() type=panel.Panel
    /// @generic.instantiation id="panel.element<\"span\", ()>" template=panel.element arguments=("span", ())
    /// @generic.instance id="panel.element<\"span\", ()>" template=panel.element arguments=("span", ()) dependents=({})

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
    );
}

/// A tree element checks the attributes its tag requires.
#[test]
fn test_check_tree_elements_with_required_attributes() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        title: {};
        meta: { content: string; name: string };
        link: { rel: string; href: string };
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = (
        <>
            <title></title>
            <meta content="width=device-width" name="viewport" />
            <link rel="icon" href="/favicon.ico" />
        </>
    );
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        crate::tests::DirRows::checked(),
        r#"
=== annotated ===
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        title: {};
        meta: { content: string; name: string };
        link: { rel: string; href: string };
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: {
            title: {};
            meta: { content: string; name: string };
            link: { rel: string; href: string };
        }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = (
        <>
            <title></title>
            <meta content="width=device-width" name="viewport" />
            <link rel="icon" href="/favicon.ico" />
        </>
    );
    return page;
}

=== dir ===
import { TreeBuilder } from "tspp:tree";

class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel implements TreeBuilder {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=TreeBuilder target=TreeBuilder
/// @definition.associated.type symbol=Tags key=Tags value={ title: {}; meta: { content: string; name: string }; link: { rel: string; href: string } }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { title: {}; meta: { content: string; name: string }; link: { rel: string; href: string } }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ title: {}; meta: { content: string; name: string }; link: { rel: string; href: string } }

        title: {};
        /// @type.symbol symbol=Tags.title source="title: {}" type={}

        meta: { content: string; name: string };
        /// @type.symbol symbol=Tags.meta source="meta: { content: string; name: string }" type={ content: string; name: string }
        /// @type.symbol symbol=Tags.content source="content: string" type=string
        /// @type.symbol symbol=Tags.name source="name: string" type=string

        link: { rel: string; href: string };
        /// @type.symbol symbol=Tags.link source="link: { rel: string; href: string }" type={ rel: string; href: string }
        /// @type.symbol symbol=Tags.rel source="rel: string" type=string
        /// @type.symbol symbol=Tags.href source="href: string" type=string

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { title: {}; meta: { content: string; name: string }; link: { rel: string; href: string } }[Tag], Children#1) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children#1

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ title: {}; meta: { content: string; name: string }; link: { rel: string; href: string } }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children#1
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=fragment parent=template#0 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=fragment type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=fragment.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=fragment.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=fragment.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = (
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel

        <>
        /// @resolution.tree builder=Panel form=fragment call=fragment children=(Panel, Panel, Panel) type=Panel
        /// @generic.instantiation id="fragment<(Panel, Panel, Panel)>" template=fragment arguments=((Panel, Panel, Panel))

            <title></title>
            /// @resolution.tree source=<title></title> builder=Panel form=element tag=title call=element children=() type=Panel
            /// @generic.instantiation id="element<\"title\", ()>" template=element arguments=("title", ())

            <meta content="width=device-width" name="viewport" />
            /// @resolution.tree source="<meta content=\"width=device-width\" name=\"viewport\" />" builder=Panel form=element tag=meta call=element attributes=(content: "width=device-width", name: "viewport") children=() type=Panel
            /// @generic.instantiation id="element<\"meta\", ()>" template=element arguments=("meta", ())

            <link rel="icon" href="/favicon.ico" />
            /// @resolution.tree source="<link rel=\"icon\" href=\"/favicon.ico\" />" builder=Panel form=element tag=link call=element attributes=(rel: "icon", href: "/favicon.ico") children=() type=Panel
            /// @generic.instantiation id="element<\"link\", ()>" template=element arguments=("link", ())

        </>
    );
    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#,
        r#"
"#,
    );
}
