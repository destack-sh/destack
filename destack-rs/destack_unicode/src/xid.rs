use crate::table_gen::derived_property::{XID_CONTINUE, XID_START};
use crate::table_gen::util::bsearch_range_table;

/// Methods for determining if a character is a valid identifier character.
pub trait UnicodeXID {
    /// Check whether the specified character satisfies the 'XID_Start' Unicode property.
    ///
    /// 'XID_Start' is a Unicode Derived Property specified in
    /// [UAX #31](http://unicode.org/reports/tr31/#NFKC_Modifications),
    /// mostly similar to ID_Start but modified for closure under NFKx.
    #[allow(clippy::wrong_self_convention)]
    fn is_xid_start(self) -> bool;

    /// Check whether the specified character satisfies the 'XID_Continue' Unicode property.
    ///
    /// 'XID_Continue' is a Unicode Derived Property specified in
    /// [UAX #31](http://unicode.org/reports/tr31/#NFKC_Modifications),
    /// mostly similar to 'ID_Continue' but modified for closure under NFKx.
    #[allow(clippy::wrong_self_convention)]
    fn is_xid_continue(self) -> bool;
}

impl UnicodeXID for char {
    #[inline]
    fn is_xid_start(self) -> bool {
        // fast-path for ASCII letters
        ('a' <= self && self <= 'z')
            || ('A' <= self && self <= 'Z')
            || (self > '\x7f' && bsearch_range_table(self, XID_START))
    }

    #[inline]
    fn is_xid_continue(self) -> bool {
        // fast-path for ASCII letters, digits, and underscore
        ('a' <= self && self <= 'z')
            || ('A' <= self && self <= 'Z')
            || ('0' <= self && self <= '9')
            || self == '_'
            || (self > '\x7f' && bsearch_range_table(self, XID_CONTINUE))
    }
}
