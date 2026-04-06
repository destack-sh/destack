use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(super) fn operator_member_key(&self, operator_item: LanguageSymbol) -> StaticKey {
        let export_name = operator_item.export_name();
        let mut chars = export_name.chars();
        let first_char = chars.next().unwrap_or_default();

        let mut member_name = String::new();
        member_name.push(first_char.to_ascii_lowercase());
        member_name.push_str(chars.as_str());

        let name_id = self.repository.strings.intern(&member_name);
        StaticKey::Name(name_id)
    }
}
