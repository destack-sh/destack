use crate::tests::{DirRows, TestSession};

/// An extension implements a generic static interface member.
#[test]
fn test_implement_generic_static_interface_members() {
    let session = TestSession::single(
        r#"
newtype interface Maker {
    static make<T>(value: T): this;
}

class Panel {}

extension of Panel implements Maker {
    static make<T>(value: T): Panel {
        return new Panel();
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Maker {
    static make<T>(value: T): this;
}

class Panel {}

extension of Panel implements Maker {
    static make<T>(value: T): Panel {
        return new Panel();
    }
}

=== dir ===
newtype interface Maker {
/// @generic.template symbol=Maker parameters=(this: Maker)
/// @type.symbol symbol=Maker type=Maker
/// @definition.interface symbol=Maker template=(this: Maker) nominal=true
/// @definition.where symbol=Maker relation=satisfies left=this right=Maker
/// @definition.method symbol=Maker.make source="static make<T>(value: T): this" slot=make static=true type=<T#1>(T#1) => this

    static make<T>(value: T): this;
    /// @generic.template symbol=Maker.make parent=template#0 parameters=(T#1)
    /// @type.symbol symbol=Maker.make source="static make<T>(value: T): this" type=<T#1>(T#1) => this
    /// @type.symbol symbol=Maker.make.T source=T type=T#1
    /// @type.symbol symbol=Maker.make.value source="value: T" type=T#1
    /// @resolution.name source=T target=Maker.make.T

}

class Panel {}
/// @type.symbol symbol=Panel source="class Panel {}" type=typeof Panel
/// @definition.class symbol=Panel source="class Panel {}"

extension of Panel implements Maker {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=Maker target=Maker
/// @definition.method symbol=make slot=make static=true type=<T#2>(T#2) => Panel
/// @definition.conformance symbol=<module>#2 member=make requirement=Maker.make
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=Maker target=Maker

    static make<T>(value: T): Panel {
    /// @generic.template symbol=make parent=template#1 parameters=(T#2)
    /// @type.symbol symbol=make type=<T#2>(T#2) => Panel
    /// @type.symbol symbol=make.T source=T type=T#2
    /// @type.symbol symbol=make.value source="value: T" type=T#2
    /// @resolution.name source=T target=make.T
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}
"#,
    );
}

/// An extension implements a static interface member with a const parameter.
#[test]
fn test_implement_const_static_interface_members() {
    let session = TestSession::single(
        r#"
newtype interface Tagger {
    static tag<const Name: string>(name: Name): this;
}

class Panel {}

extension of Panel implements Tagger {
    static tag<const Name: string>(name: Name): Panel {
        return new Panel();
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Tagger {
    static tag<const Name: string>(name: Name): this;
}

class Panel {}

extension of Panel implements Tagger {
    static tag<const Name: string>(name: Name): Panel {
        return new Panel();
    }
}

=== dir ===
newtype interface Tagger {
/// @generic.template symbol=Tagger parameters=(this: Tagger)
/// @type.symbol symbol=Tagger type=Tagger
/// @definition.interface symbol=Tagger template=(this: Tagger) nominal=true
/// @definition.where symbol=Tagger relation=satisfies left=this right=Tagger
/// @definition.method symbol=Tagger.tag source="static tag<const Name: string>(name: Name): this" slot=tag static=true type=<const Name#1: string>(Name#1) => this

    static tag<const Name: string>(name: Name): this;
    /// @generic.template symbol=Tagger.tag parent=template#0 parameters=(const Name#1: string)
    /// @type.symbol symbol=Tagger.tag source="static tag<const Name: string>(name: Name): this" type=<const Name#1: string>(Name#1) => this
    /// @type.symbol symbol=Tagger.tag.Name source="const Name: string" type=Name#1
    /// @type.symbol symbol=Tagger.tag.name source="name: Name" type=Name#1
    /// @resolution.name source=Name target=Tagger.tag.Name

}

class Panel {}
/// @type.symbol symbol=Panel source="class Panel {}" type=typeof Panel
/// @definition.class symbol=Panel source="class Panel {}"

extension of Panel implements Tagger {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=Tagger target=Tagger
/// @definition.method symbol=tag slot=tag static=true type=<const Name#2: string>(Name#2) => Panel
/// @definition.conformance symbol=<module>#2 member=tag requirement=Tagger.tag
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=Tagger target=Tagger

    static tag<const Name: string>(name: Name): Panel {
    /// @generic.template symbol=tag parent=template#1 parameters=(const Name#2: string)
    /// @type.symbol symbol=tag type=<const Name#2: string>(Name#2) => Panel
    /// @type.symbol symbol=tag.Name source="const Name: string" type=Name#2
    /// @type.symbol symbol=tag.name source="name: Name" type=Name#2
    /// @resolution.name source=Name target=tag.Name
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}
"#,
    );
}

/// An extension implements a static member bounded by keyof an associated type.
#[test]
fn test_implement_keyof_bounded_static_interface_members() {
    let session = TestSession::single(
        r#"
newtype interface Rowed {
    type Rows = {};

    static row<const Key: keyof this.Rows>(key: Key): this;
}

class Panel {}

extension of Panel implements Rowed {
    type Rows = { header: string };

    static row<const Key: keyof this.Rows>(key: Key): Panel {
        return new Panel();
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Rowed {
    type Rows = {};

    static row<const Key: keyof this.Rows>(key: Key): this;
}

class Panel {}

extension of Panel implements Rowed {
    type Rows = { header: string };

    static row<const Key: keyof this.Rows>(key: Key): Panel {
        return new Panel();
    }
}

=== dir ===
newtype interface Rowed {
/// @generic.template symbol=Rowed parameters=(this: Rowed)
/// @type.symbol symbol=Rowed type=Rowed
/// @definition.interface symbol=Rowed template=(this: Rowed) nominal=true
/// @definition.where symbol=Rowed relation=satisfies left=this right=Rowed
/// @definition.associated.type symbol=Rowed.Rows source="type Rows = {}" key=Rows value={}
/// @definition.method symbol=Rowed.row source="static row<const Key: keyof this.Rows>(key: Key): this" slot=row static=true type=<const Key#1: keyof this.Rows>(Key#1) => this

    type Rows = {};
    /// @type.symbol symbol=Rowed.Rows source="type Rows = {}" type={}

    static row<const Key: keyof this.Rows>(key: Key): this;
    /// @generic.template symbol=Rowed.row parent=template#0 parameters=(const Key#1: keyof this.Rows)
    /// @type.symbol symbol=Rowed.row source="static row<const Key: keyof this.Rows>(key: Key): this" type=<const Key#1: keyof this.Rows>(Key#1) => this
    /// @type.symbol symbol=Rowed.row.Key source="const Key: keyof this.Rows" type=Key#1
    /// @resolution.name source=this.Rows target=Rowed.Rows
    /// @type.symbol symbol=Rowed.row.key source="key: Key" type=Key#1
    /// @resolution.name source=Key target=Rowed.row.Key

}

class Panel {}
/// @type.symbol symbol=Panel source="class Panel {}" type=typeof Panel
/// @definition.class symbol=Panel source="class Panel {}"

extension of Panel implements Rowed {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=Rowed target=Rowed
/// @definition.associated.type symbol=Rows source="type Rows = { header: string }" key=Rows value={ header: string }
/// @definition.method symbol=row slot=row static=true type=<const Key#2: keyof Panel.Rows>(Key#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Rows requirement=Rowed.Rows
/// @definition.conformance symbol=<module>#2 member=row requirement=Rowed.row
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=Rowed target=Rowed

    type Rows = { header: string };
    /// @type.symbol symbol=Rows source="type Rows = { header: string }" type={ header: string }
    /// @type.symbol symbol=Rows.header source="header: string" type=string

    static row<const Key: keyof this.Rows>(key: Key): Panel {
    /// @generic.template symbol=row parent=template#1 parameters=(const Key#2: keyof Panel.Rows)
    /// @type.symbol symbol=row type=<const Key#2: keyof Panel.Rows>(Key#2) => Panel
    /// @type.symbol symbol=row.Key source="const Key: keyof this.Rows" type=Key#2
    /// @resolution.name source=this.Rows target=Rows
    /// @type.symbol symbol=row.key source="key: Key" type=Key#2
    /// @resolution.name source=Key target=row.Key
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}
"#,
    );
}

/// An extension implements a static member projecting the row its key selects.
#[test]
fn test_implement_row_projected_static_interface_members() {
    let session = TestSession::single(
        r#"
newtype interface Rowed {
    type Rows = {};

    static row<const Key: keyof this.Rows>(key: Key, value: this.Rows[Key]): this;
}

class Panel {}

extension of Panel implements Rowed {
    type Rows = { header: string };

    static row<const Key: keyof this.Rows>(key: Key, value: this.Rows[Key]): Panel {
        return new Panel();
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Rowed {
    type Rows = {};

    static row<const Key: keyof this.Rows>(key: Key, value: this.Rows[Key]): this;
}

class Panel {}

extension of Panel implements Rowed {
    type Rows = { header: string };

    static row<const Key: keyof this.Rows>(key: Key, value: { header: string }[Key]): Panel {
        return new Panel();
    }
}

=== dir ===
newtype interface Rowed {
/// @generic.template symbol=Rowed parameters=(this: Rowed)
/// @type.symbol symbol=Rowed type=Rowed
/// @definition.interface symbol=Rowed template=(this: Rowed) nominal=true
/// @definition.where symbol=Rowed relation=satisfies left=this right=Rowed
/// @definition.associated.type symbol=Rowed.Rows source="type Rows = {}" key=Rows value={}
/// @definition.method symbol=Rowed.row source="static row<const Key: keyof this.Rows>(key: Key, value: this.Rows[Key]): this" slot=row static=true type=<const Key#1: keyof this.Rows>(Key#1, this.Rows[Key#1]) => this

    type Rows = {};
    /// @type.symbol symbol=Rowed.Rows source="type Rows = {}" type={}

    static row<const Key: keyof this.Rows>(key: Key, value: this.Rows[Key]): this;
    /// @generic.template symbol=Rowed.row parent=template#0 parameters=(const Key#1: keyof this.Rows)
    /// @type.symbol symbol=Rowed.row source="static row<const Key: keyof this.Rows>(key: Key, value: this.Rows[Key]): this" type=<const Key#1: keyof this.Rows>(Key#1, this.Rows[Key#1]) => this
    /// @type.symbol symbol=Rowed.row.Key source="const Key: keyof this.Rows" type=Key#1
    /// @resolution.name source=this.Rows target=Rowed.Rows
    /// @type.symbol symbol=Rowed.row.key source="key: Key" type=Key#1
    /// @resolution.name source=Key target=Rowed.row.Key
    /// @type.symbol symbol=Rowed.row.value source="value: this.Rows[Key]" type=this.Rows[Key#1]
    /// @resolution.name source=this.Rows target=Rowed.Rows
    /// @resolution.name source=Key target=Rowed.row.Key

}

class Panel {}
/// @type.symbol symbol=Panel source="class Panel {}" type=typeof Panel
/// @definition.class symbol=Panel source="class Panel {}"

extension of Panel implements Rowed {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=Rowed target=Rowed
/// @definition.associated.type symbol=Rows source="type Rows = { header: string }" key=Rows value={ header: string }
/// @definition.method symbol=row slot=row static=true type=<const Key#2: keyof Panel.Rows>(Key#2, { header: string }[Key#2]) => Panel
/// @definition.conformance symbol=<module>#2 member=Rows requirement=Rowed.Rows
/// @definition.conformance symbol=<module>#2 member=row requirement=Rowed.row
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=Rowed target=Rowed

    type Rows = { header: string };
    /// @type.symbol symbol=Rows source="type Rows = { header: string }" type={ header: string }
    /// @type.symbol symbol=Rows.header source="header: string" type=string

    static row<const Key: keyof this.Rows>(key: Key, value: this.Rows[Key]): Panel {
    /// @generic.template symbol=row parent=template#1 parameters=(const Key#2: keyof Panel.Rows)
    /// @type.symbol symbol=row type=<const Key#2: keyof Panel.Rows>(Key#2, { header: string }[Key#2]) => Panel
    /// @type.symbol symbol=row.Key source="const Key: keyof this.Rows" type=Key#2
    /// @resolution.name source=this.Rows target=Rows
    /// @type.symbol symbol=row.key source="key: Key" type=Key#2
    /// @resolution.name source=Key target=row.Key
    /// @type.symbol symbol=row.value source="value: this.Rows[Key]" type={ header: string }[Key#2]
    /// @resolution.name source=this.Rows target=Rows
    /// @resolution.name source=Key target=row.Key
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}
"#,
    );
}

/// An extension implements a static member bounded by a variadic tuple.
#[test]
fn test_implement_tuple_bounded_static_interface_members() {
    let session = TestSession::single(
        r#"
newtype interface Grouper {
    static group<Children: (...unknown[],)>(children: Children): this;
}

class Panel {}

extension of Panel implements Grouper {
    static group<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Grouper {
    static group<Children: (...unknown[],)>(children: Children): this;
}

class Panel {}

extension of Panel implements Grouper {
    static group<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

=== dir ===
newtype interface Grouper {
/// @generic.template symbol=Grouper parameters=(this: Grouper)
/// @type.symbol symbol=Grouper type=Grouper
/// @definition.interface symbol=Grouper template=(this: Grouper) nominal=true
/// @definition.where symbol=Grouper relation=satisfies left=this right=Grouper
/// @definition.method symbol=Grouper.group source="static group<Children: (...unknown[],)>(children: Children): this" slot=group static=true type=<Children#1: (...unknown[],)>(Children#1) => this

    static group<Children: (...unknown[],)>(children: Children): this;
    /// @generic.template symbol=Grouper.group parent=template#0 parameters=(Children#1: (...unknown[],))
    /// @type.symbol symbol=Grouper.group source="static group<Children: (...unknown[],)>(children: Children): this" type=<Children#1: (...unknown[],)>(Children#1) => this
    /// @type.symbol symbol=Grouper.group.Children source="Children: (...unknown[],)" type=Children#1
    /// @generic.instance id=Array<unknown> template=Array arguments=(unknown)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<unknown>> template=sliceAssumeInit arguments=(MaybeUninit<unknown>)
    /// @generic.instance id=sliceUninit<MaybeUninit<unknown>> template=sliceUninit arguments=(MaybeUninit<unknown>)
    /// @type.symbol symbol=Grouper.group.children source="children: Children" type=Children#1
    /// @resolution.name source=Children target=Grouper.group.Children

}

class Panel {}
/// @type.symbol symbol=Panel source="class Panel {}" type=typeof Panel
/// @definition.class symbol=Panel source="class Panel {}"

extension of Panel implements Grouper {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.implements symbol=<module>#2 source=Grouper target=Grouper
/// @definition.method symbol=group slot=group static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=group requirement=Grouper.group
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=Grouper target=Grouper

    static group<Children: (...unknown[],)>(children: Children): Panel {
    /// @generic.template symbol=group parent=template#1 parameters=(Children#2: (...unknown[],))
    /// @type.symbol symbol=group type=<Children#2: (...unknown[],)>(Children#2) => Panel
    /// @type.symbol symbol=group.Children source="Children: (...unknown[],)" type=Children#2
    /// @type.symbol symbol=group.children source="children: Children" type=Children#2
    /// @resolution.name source=Children target=group.Children
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}
"#,
    );
}

/// An extension implements the tree builder interface with its declared statics.
#[test]
fn test_implement_tree_builder_with_declared_statics() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
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
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
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
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; span: {} }[Tag], Children#1) => Panel
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
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; span: {} }[Tag]
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
"#,
    );
}

/// A tree element checks against the builder its expected type names.
#[test]
fn test_check_tree_element_against_the_contextual_builder() {
    let session = TestSession::single(
        r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    label: string = "";
}

extension of Panel implements TreeBuilder {
    type Tags = {
        div: { class?: string };
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
    const page: Panel = <div class="intro"><span/></div>;
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
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; span: {} }[Tag],
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
        <div class="intro">
            <span />
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
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; span: {} }[Tag], Children#1) => Panel
/// @definition.method symbol=fragment slot=fragment static=true type=<Children#2: (...unknown[],)>(Children#2) => Panel
/// @definition.conformance symbol=<module>#2 member=Tags requirement=TreeBuilder.Tags
/// @definition.conformance symbol=<module>#2 member=element requirement=TreeBuilder.element
/// @definition.conformance symbol=<module>#2 member=fragment requirement=TreeBuilder.fragment
/// @resolution.name source=Panel target=Panel
/// @resolution.name source=TreeBuilder target=TreeBuilder

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parent=template#0 parameters=(const Tag: keyof Panel.Tags, Children#1: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children#1: (...unknown[],)>(Tag, { div: { class?: string }; span: {} }[Tag], Children#1) => Panel
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
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; span: {} }[Tag]
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

    const page: Panel = <div class="intro"><span/></div>;
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.tree source="<div class=\"intro\"><span/></div>" builder=Panel form=element tag=div call=element attributes=(class: "intro") children=(Panel) type=Panel
    /// @generic.instantiation id="element<\"div\", (Panel,)>" template=element arguments=("div", (Panel,))
    /// @generic.instance id="element<\"div\", (Panel,)>" template=element arguments=("div", (Panel,)) dependents=({ class?: string })
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

/// An object literal checks against the row a const key projects.
#[test]
fn test_check_object_literal_against_a_const_projected_parameter() {
    let session = TestSession::single(
        r#"
class Panel {
    label: string = "";
}

extension of Panel {
    type Tags = {
        div: { class?: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = Panel.element("div", { class: "intro" }, (new Panel(),));
    return page;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Panel {
    label: string = "";
}

extension of Panel {
    type Tags = {
        div: { class?: string };
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: { div: { class?: string }; span: {} }[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }
}

function render(): Panel {
    const page: Panel = Panel.element<"div", (Panel,)>(
        "div",
        { class: "intro" as string | undefined },
        (new Panel(),),
    );
    return page;
}

=== dir ===
class Panel {
/// @type.symbol symbol=Panel type=typeof Panel
/// @definition.class symbol=Panel
/// @definition.field symbol=Panel.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Panel.label source="label: string = \"\"" type=string

}

extension of Panel {
/// @definition.extension symbol=<module>#2 form=local target=Panel
/// @definition.associated.type symbol=Tags key=Tags value={ div: { class?: string }; span: {} }
/// @definition.method symbol=element slot=element static=true type=<const Tag: keyof Panel.Tags, Children: (...unknown[],)>(Tag, { div: { class?: string }; span: {} }[Tag], Children) => Panel
/// @resolution.name source=Panel target=Panel

    type Tags = {
    /// @type.symbol symbol=Tags type={ div: { class?: string }; span: {} }

        div: { class?: string };
        /// @type.symbol symbol=Tags.div source="div: { class?: string }" type={ class?: string }
        /// @type.symbol symbol=Tags.class source="class?: string" type=string

        span: {};
        /// @type.symbol symbol=Tags.span source="span: {}" type={}

    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
    /// @generic.template symbol=element parameters=(const Tag: keyof Panel.Tags, Children: (...unknown[],))
    /// @type.symbol symbol=element type=<const Tag: keyof Panel.Tags, Children: (...unknown[],)>(Tag, { div: { class?: string }; span: {} }[Tag], Children) => Panel
    /// @type.symbol symbol=element.Tag source="const Tag: keyof this.Tags" type=Tag
    /// @resolution.name source=this.Tags target=Tags
    /// @type.symbol symbol=element.Children source="Children: (...unknown[],)" type=Children

        tag: Tag,
        /// @type.symbol symbol=element.tag source="tag: Tag" type=Tag
        /// @resolution.name source=Tag target=element.Tag

        attributes: this.Tags[Tag],
        /// @type.symbol symbol=element.attributes source="attributes: this.Tags[Tag]" type={ div: { class?: string }; span: {} }[Tag]
        /// @resolution.name source=this.Tags target=Tags
        /// @resolution.name source=Tag target=element.Tag

        children: Children,
        /// @type.symbol symbol=element.children source="children: Children" type=Children
        /// @resolution.name source=Children target=element.Children

    ): Panel {
    /// @resolution.name source=Panel target=Panel

        return new Panel();
        /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
        /// @resolution.name source=Panel target=Panel

    }
}

function render(): Panel {
/// @type.symbol symbol=render type=() => Panel
/// @resolution.name source=Panel target=Panel

    const page: Panel = Panel.element("div", { class: "intro" }, (new Panel(),));
    /// @type.symbol symbol=render.page source=page type=Panel
    /// @resolution.pattern source=page kind=binding target=render.page
    /// @resolution.name source=Panel target=Panel
    /// @resolution.name source=Panel target=Panel
    /// @resolution.member source=Panel.element receiver=typeof Panel type=<const Tag: keyof Panel.Tags, Children: (...unknown[],)>(Tag, { div: { class?: string }; span: {} }[Tag], Children) => Panel kind=symbol target_receiver=typeof Panel target=element
    /// @resolution.call source="Panel.element(\"div\", { class: \"intro\" }, (new Panel(),))" parameters=("div", { div: { class?: string }; span: {} }["div"], (Panel,)) arguments=(provided("div") as "div", provided({ class: "intro" }) as { div: { class?: string }; span: {} }["div"], provided((new Panel(),)) as (Panel,)) return=Panel kind=symbol target=element instance="Panel.<extension#1>.element<\"div\", (Panel,)>"
    /// @generic.instantiation id="element<\"div\", (Panel,)>" template=element arguments=("div", (Panel,))
    /// @resolution.construct source="new Panel()" parameters=() return=Panel kind=class target=Panel constructor=default
    /// @resolution.name source=Panel target=Panel

    return page;
    /// @resolution.name source=page target=render.page
    /// @resolution.place source=page placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=page root=render.page

}
"#, r#"
"#);
}
