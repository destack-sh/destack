use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host contact query payload.
        struct HostContactQuery {
            /// Whether the query carries one cursor.
            has_cursor: bool,
            /// The opaque cursor from one prior list or search call.
            cursor: string_ref,
            /// Whether the query carries one limit.
            has_limit: bool,
            /// The maximum returned contacts for this page.
            limit: u32,
            /// Whether phone values should be returned.
            include_phones: bool,
            /// Whether email values should be returned.
            include_emails: bool,
            /// Whether postal-address values should be returned.
            include_addresses: bool,
            /// Whether organization metadata should be returned.
            include_organization: bool,
            /// Whether note fields should be returned.
            include_notes: bool,
        }

        /// One host contact-name payload.
        struct HostContactName {
            /// The given or first name.
            given_name: string_ref,
            /// The middle name.
            middle_name: string_ref,
            /// The family or last name.
            family_name: string_ref,
            /// The honorific prefix.
            prefix: string_ref,
            /// The honorific suffix.
            suffix: string_ref,
            /// The nickname.
            nickname: string_ref,
            /// The phonetic given name.
            phonetic_given_name: string_ref,
            /// The phonetic family name.
            phonetic_family_name: string_ref,
        }

        /// One host contact-phone payload.
        struct HostContactPhone {
            /// The user-visible label for this phone value.
            label: string_ref,
            /// The original phone number string.
            number: string_ref,
            /// The normalized phone number string when available.
            normalized_number: string_ref,
            /// Whether this phone value is marked as primary.
            primary: bool,
        }

        /// One host contact-email payload.
        struct HostContactEmail {
            /// The user-visible label for this email value.
            label: string_ref,
            /// The email address.
            address: string_ref,
            /// Whether this email value is marked as primary.
            primary: bool,
        }

        /// One host contact-address payload.
        struct HostContactAddress {
            /// The user-visible label for this address value.
            label: string_ref,
            /// The street-line payload.
            street: string_ref,
            /// The city payload.
            city: string_ref,
            /// The region or state payload.
            region: string_ref,
            /// The postal-code payload.
            postal_code: string_ref,
            /// The country payload.
            country: string_ref,
            /// The country-code payload.
            country_code: string_ref,
        }

        /// One host contact-organization payload.
        struct HostContactOrganization {
            /// The company or organization name.
            company: string_ref,
            /// The department name.
            department: string_ref,
            /// The job title.
            title: string_ref,
        }

        /// One host contact payload.
        struct HostContact {
            /// The stable host contact identifier.
            id: string_ref,
            /// The structured name payload.
            name: HostContactName,
            /// The phone values.
            phones: slice(HostContactPhone),
            /// The email values.
            emails: slice(HostContactEmail),
            /// The address values.
            addresses: slice(HostContactAddress),
            /// The organization metadata.
            organization: HostContactOrganization,
            /// The contact note payload.
            note: string_ref,
        }

        /// One host contact draft payload.
        struct HostContactDraft {
            /// The structured name payload.
            name: HostContactName,
            /// The phone values.
            phones: slice(HostContactPhone),
            /// The email values.
            emails: slice(HostContactEmail),
            /// The address values.
            addresses: slice(HostContactAddress),
            /// The organization metadata.
            organization: HostContactOrganization,
            /// The contact note payload.
            note: string_ref,
        }

        /// One host contact page payload.
        struct HostContactPage {
            /// The listed contacts for this page.
            contacts: slice(HostContact),
            /// The opaque next-page cursor.
            next_cursor: string_ref,
            /// Whether more contacts are available.
            has_more: bool,
        }

        /// One host contact-page response payload.
        struct HostContactPageResponse {
            /// The request status code.
            status: host_status,
            /// Whether the response includes one page.
            has_page: bool,
            /// The returned contact page when available.
            page: HostContactPage,
        }

        /// One host contact-read response payload.
        struct HostContactResponse {
            /// The request status code.
            status: host_status,
            /// Whether the response includes one contact.
            has_contact: bool,
            /// The returned contact when available.
            contact: HostContact,
        }

        /// One host contact-create response payload.
        struct HostContactCreateResponse {
            /// The request status code.
            status: host_status,
            /// Whether the response includes one created contact identifier.
            has_id: bool,
            /// The created contact identifier when available.
            id: string_ref,
        }
    }
}
