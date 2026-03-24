#include "../types.h"
#include "../jni.h"
#include "../registry.h"
#include "methods.h"

#include <deque>
#include <string>
#include <vector>

namespace {

jclass bridge_class = nullptr;
jclass list_class = nullptr;
jclass contact_page_response_class = nullptr;
jclass contact_page_class = nullptr;
jclass contact_response_class = nullptr;
jclass contact_create_response_class = nullptr;
jclass contact_class = nullptr;
jclass contact_name_class = nullptr;
jclass contact_phone_class = nullptr;
jclass contact_email_class = nullptr;
jclass contact_address_class = nullptr;
jclass contact_organization_class = nullptr;
jmethodID list_contacts_method = nullptr;
jmethodID search_contacts_method = nullptr;
jmethodID read_contact_method = nullptr;
jmethodID create_contact_method = nullptr;
jmethodID update_contact_method = nullptr;
jmethodID delete_contact_method = nullptr;
jmethodID list_size_method = nullptr;
jmethodID list_get_method = nullptr;
jmethodID contact_page_response_get_status_method = nullptr;
jmethodID contact_page_response_get_page_method = nullptr;
jmethodID contact_page_get_contacts_method = nullptr;
jmethodID contact_page_get_next_cursor_method = nullptr;
jmethodID contact_page_get_has_more_method = nullptr;
jmethodID contact_response_get_status_method = nullptr;
jmethodID contact_response_get_contact_method = nullptr;
jmethodID contact_create_response_get_status_method = nullptr;
jmethodID contact_create_response_get_id_method = nullptr;
jmethodID contact_get_id_method = nullptr;
jmethodID contact_get_name_method = nullptr;
jmethodID contact_get_phones_method = nullptr;
jmethodID contact_get_emails_method = nullptr;
jmethodID contact_get_addresses_method = nullptr;
jmethodID contact_get_organization_method = nullptr;
jmethodID contact_get_note_method = nullptr;
jmethodID contact_name_get_given_name_method = nullptr;
jmethodID contact_name_get_middle_name_method = nullptr;
jmethodID contact_name_get_family_name_method = nullptr;
jmethodID contact_name_get_prefix_method = nullptr;
jmethodID contact_name_get_suffix_method = nullptr;
jmethodID contact_name_get_nickname_method = nullptr;
jmethodID contact_name_get_phonetic_given_name_method = nullptr;
jmethodID contact_name_get_phonetic_family_name_method = nullptr;
jmethodID contact_phone_constructor = nullptr;
jmethodID contact_phone_get_label_method = nullptr;
jmethodID contact_phone_get_number_method = nullptr;
jmethodID contact_phone_get_normalized_number_method = nullptr;
jmethodID contact_phone_get_primary_method = nullptr;
jmethodID contact_email_constructor = nullptr;
jmethodID contact_email_get_label_method = nullptr;
jmethodID contact_email_get_address_method = nullptr;
jmethodID contact_email_get_primary_method = nullptr;
jmethodID contact_address_constructor = nullptr;
jmethodID contact_address_get_label_method = nullptr;
jmethodID contact_address_get_street_method = nullptr;
jmethodID contact_address_get_city_method = nullptr;
jmethodID contact_address_get_region_method = nullptr;
jmethodID contact_address_get_postal_code_method = nullptr;
jmethodID contact_address_get_country_method = nullptr;
jmethodID contact_address_get_country_code_method = nullptr;
jmethodID contact_organization_get_company_method = nullptr;
jmethodID contact_organization_get_department_method = nullptr;
jmethodID contact_organization_get_title_method = nullptr;
thread_local std::deque<std::string> result_string_storage;
thread_local std::deque<std::vector<HostContactPhone>> result_phone_storage;
thread_local std::deque<std::vector<HostContactEmail>> result_email_storage;
thread_local std::deque<std::vector<HostContactAddress>> result_address_storage;
thread_local std::vector<HostContact> result_contact_storage;

/// Convert one Java string into one native string reference backed by stable storage.
NativeStringRef string_ref_from_java(
    JNIEnv *env,
    jstring value,
    std::deque<std::string> *storage
) {
    if (value == nullptr) {
        return NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    if (chars == nullptr) {
        return NativeStringRef {
            .data = nullptr,
            .len = 0,
        };
    }

    jsize length = env->GetStringUTFLength(value);
    storage->emplace_back(chars, chars + length);
    env->ReleaseStringUTFChars(value, chars);

    const std::string &owned = storage->back();

    return NativeStringRef {
        .data = reinterpret_cast<const uint8_t *>(owned.data()),
        .len = static_cast<uint32_t>(owned.size()),
    };
}

/// Resolve the shared java.util.List helpers.
bool resolve_list_methods(JNIEnv *env) {
    if (list_class == nullptr) {
        jclass local_class = env->FindClass("java/util/List");
        if (local_class == nullptr) {
            return false;
        }

        list_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (list_class == nullptr) {
            return false;
        }
    }

    if (list_size_method == nullptr) {
        list_size_method = env->GetMethodID(list_class, "size", "()I");
    }

    if (list_get_method == nullptr) {
        list_get_method = env->GetMethodID(list_class, "get", "(I)Ljava/lang/Object;");
    }

    return list_size_method != nullptr && list_get_method != nullptr;
}

/// Resolve the contact bridge entrypoints from one runtime bridge instance.
bool resolve_bridge_methods(JNIEnv *env, jobject bridge) {
    if (bridge_class == nullptr) {
        jclass local_class = env->GetObjectClass(bridge);
        if (local_class == nullptr) {
            return false;
        }

        bridge_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (bridge_class == nullptr) {
            return false;
        }
    }

    if (list_contacts_method == nullptr) {
        list_contacts_method = env->GetMethodID(
            bridge_class,
            "listContacts",
            "(Ljava/lang/String;ZIZZZZZ)Ldev/destack/runtime/android/module/contact/RuntimeHostContactPageResponse;"
        );
    }

    if (search_contacts_method == nullptr) {
        search_contacts_method = env->GetMethodID(
            bridge_class,
            "searchContacts",
            "(Ljava/lang/String;Ljava/lang/String;ZIZZZZZ)Ldev/destack/runtime/android/module/contact/RuntimeHostContactPageResponse;"
        );
    }

    if (read_contact_method == nullptr) {
        read_contact_method = env->GetMethodID(
            bridge_class,
            "readContact",
            "(Ljava/lang/String;)Ldev/destack/runtime/android/module/contact/RuntimeHostContactResponse;"
        );
    }

    if (create_contact_method == nullptr) {
        create_contact_method = env->GetMethodID(
            bridge_class,
            "createContact",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;[Ldev/destack/runtime/android/module/contact/RuntimeHostContactPhone;[Ldev/destack/runtime/android/module/contact/RuntimeHostContactEmail;[Ldev/destack/runtime/android/module/contact/RuntimeHostContactAddress;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ldev/destack/runtime/android/module/contact/RuntimeHostContactCreateResponse;"
        );
    }

    if (update_contact_method == nullptr) {
        update_contact_method = env->GetMethodID(
            bridge_class,
            "updateContact",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;[Ldev/destack/runtime/android/module/contact/RuntimeHostContactPhone;[Ldev/destack/runtime/android/module/contact/RuntimeHostContactEmail;[Ldev/destack/runtime/android/module/contact/RuntimeHostContactAddress;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"
        );
    }

    if (delete_contact_method == nullptr) {
        delete_contact_method = env->GetMethodID(
            bridge_class,
            "deleteContact",
            "(Ljava/lang/String;)I"
        );
    }

    return
        list_contacts_method != nullptr &&
        search_contacts_method != nullptr &&
        read_contact_method != nullptr &&
        create_contact_method != nullptr &&
        update_contact_method != nullptr &&
        delete_contact_method != nullptr;
}

/// Resolve the shared contact response and payload accessors.
bool resolve_contact_types(JNIEnv *env) {
    if (contact_page_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactPageResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_page_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_page_response_class == nullptr) {
            return false;
        }
    }

    if (contact_page_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactPage"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_page_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_page_class == nullptr) {
            return false;
        }
    }

    if (contact_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_response_class == nullptr) {
            return false;
        }
    }

    if (contact_create_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactCreateResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_create_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_create_response_class == nullptr) {
            return false;
        }
    }

    if (contact_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContact"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_class == nullptr) {
            return false;
        }
    }

    if (contact_name_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactName"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_name_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_name_class == nullptr) {
            return false;
        }
    }

    if (contact_phone_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactPhone"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_phone_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_phone_class == nullptr) {
            return false;
        }
    }

    if (contact_email_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactEmail"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_email_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_email_class == nullptr) {
            return false;
        }
    }

    if (contact_address_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactAddress"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_address_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_address_class == nullptr) {
            return false;
        }
    }

    if (contact_organization_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/contact/RuntimeHostContactOrganization"
        );
        if (local_class == nullptr) {
            return false;
        }

        contact_organization_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (contact_organization_class == nullptr) {
            return false;
        }
    }

    if (contact_page_response_get_status_method == nullptr) {
        contact_page_response_get_status_method = env->GetMethodID(
            contact_page_response_class,
            "getStatus",
            "()I"
        );
    }

    if (contact_page_response_get_page_method == nullptr) {
        contact_page_response_get_page_method = env->GetMethodID(
            contact_page_response_class,
            "getPage",
            "()Ldev/destack/runtime/android/module/contact/RuntimeHostContactPage;"
        );
    }

    if (contact_page_get_contacts_method == nullptr) {
        contact_page_get_contacts_method = env->GetMethodID(
            contact_page_class,
            "getContacts",
            "()Ljava/util/List;"
        );
    }

    if (contact_page_get_next_cursor_method == nullptr) {
        contact_page_get_next_cursor_method = env->GetMethodID(
            contact_page_class,
            "getNextCursor",
            "()Ljava/lang/String;"
        );
    }

    if (contact_page_get_has_more_method == nullptr) {
        contact_page_get_has_more_method = env->GetMethodID(
            contact_page_class,
            "getHasMore",
            "()Z"
        );
    }

    if (contact_response_get_status_method == nullptr) {
        contact_response_get_status_method = env->GetMethodID(
            contact_response_class,
            "getStatus",
            "()I"
        );
    }

    if (contact_response_get_contact_method == nullptr) {
        contact_response_get_contact_method = env->GetMethodID(
            contact_response_class,
            "getContact",
            "()Ldev/destack/runtime/android/module/contact/RuntimeHostContact;"
        );
    }

    if (contact_create_response_get_status_method == nullptr) {
        contact_create_response_get_status_method = env->GetMethodID(
            contact_create_response_class,
            "getStatus",
            "()I"
        );
    }

    if (contact_create_response_get_id_method == nullptr) {
        contact_create_response_get_id_method = env->GetMethodID(
            contact_create_response_class,
            "getId",
            "()Ljava/lang/String;"
        );
    }

    if (contact_get_id_method == nullptr) {
        contact_get_id_method = env->GetMethodID(contact_class, "getId", "()Ljava/lang/String;");
    }

    if (contact_get_name_method == nullptr) {
        contact_get_name_method = env->GetMethodID(
            contact_class,
            "getName",
            "()Ldev/destack/runtime/android/module/contact/RuntimeHostContactName;"
        );
    }

    if (contact_get_phones_method == nullptr) {
        contact_get_phones_method = env->GetMethodID(contact_class, "getPhones", "()Ljava/util/List;");
    }

    if (contact_get_emails_method == nullptr) {
        contact_get_emails_method = env->GetMethodID(contact_class, "getEmails", "()Ljava/util/List;");
    }

    if (contact_get_addresses_method == nullptr) {
        contact_get_addresses_method = env->GetMethodID(contact_class, "getAddresses", "()Ljava/util/List;");
    }

    if (contact_get_organization_method == nullptr) {
        contact_get_organization_method = env->GetMethodID(
            contact_class,
            "getOrganization",
            "()Ldev/destack/runtime/android/module/contact/RuntimeHostContactOrganization;"
        );
    }

    if (contact_get_note_method == nullptr) {
        contact_get_note_method = env->GetMethodID(contact_class, "getNote", "()Ljava/lang/String;");
    }

    if (contact_name_get_given_name_method == nullptr) {
        contact_name_get_given_name_method = env->GetMethodID(contact_name_class, "getGivenName", "()Ljava/lang/String;");
    }

    if (contact_name_get_middle_name_method == nullptr) {
        contact_name_get_middle_name_method = env->GetMethodID(contact_name_class, "getMiddleName", "()Ljava/lang/String;");
    }

    if (contact_name_get_family_name_method == nullptr) {
        contact_name_get_family_name_method = env->GetMethodID(contact_name_class, "getFamilyName", "()Ljava/lang/String;");
    }

    if (contact_name_get_prefix_method == nullptr) {
        contact_name_get_prefix_method = env->GetMethodID(contact_name_class, "getPrefix", "()Ljava/lang/String;");
    }

    if (contact_name_get_suffix_method == nullptr) {
        contact_name_get_suffix_method = env->GetMethodID(contact_name_class, "getSuffix", "()Ljava/lang/String;");
    }

    if (contact_name_get_nickname_method == nullptr) {
        contact_name_get_nickname_method = env->GetMethodID(contact_name_class, "getNickname", "()Ljava/lang/String;");
    }

    if (contact_name_get_phonetic_given_name_method == nullptr) {
        contact_name_get_phonetic_given_name_method = env->GetMethodID(contact_name_class, "getPhoneticGivenName", "()Ljava/lang/String;");
    }

    if (contact_name_get_phonetic_family_name_method == nullptr) {
        contact_name_get_phonetic_family_name_method = env->GetMethodID(contact_name_class, "getPhoneticFamilyName", "()Ljava/lang/String;");
    }

    if (contact_phone_constructor == nullptr) {
        contact_phone_constructor = env->GetMethodID(
            contact_phone_class,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Z)V"
        );
    }

    if (contact_phone_get_label_method == nullptr) {
        contact_phone_get_label_method = env->GetMethodID(contact_phone_class, "getLabel", "()Ljava/lang/String;");
    }

    if (contact_phone_get_number_method == nullptr) {
        contact_phone_get_number_method = env->GetMethodID(contact_phone_class, "getNumber", "()Ljava/lang/String;");
    }

    if (contact_phone_get_normalized_number_method == nullptr) {
        contact_phone_get_normalized_number_method = env->GetMethodID(contact_phone_class, "getNormalizedNumber", "()Ljava/lang/String;");
    }

    if (contact_phone_get_primary_method == nullptr) {
        contact_phone_get_primary_method = env->GetMethodID(contact_phone_class, "getPrimary", "()Z");
    }

    if (contact_email_constructor == nullptr) {
        contact_email_constructor = env->GetMethodID(
            contact_email_class,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Z)V"
        );
    }

    if (contact_email_get_label_method == nullptr) {
        contact_email_get_label_method = env->GetMethodID(contact_email_class, "getLabel", "()Ljava/lang/String;");
    }

    if (contact_email_get_address_method == nullptr) {
        contact_email_get_address_method = env->GetMethodID(contact_email_class, "getAddress", "()Ljava/lang/String;");
    }

    if (contact_email_get_primary_method == nullptr) {
        contact_email_get_primary_method = env->GetMethodID(contact_email_class, "getPrimary", "()Z");
    }

    if (contact_address_constructor == nullptr) {
        contact_address_constructor = env->GetMethodID(
            contact_address_class,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"
        );
    }

    if (contact_address_get_label_method == nullptr) {
        contact_address_get_label_method = env->GetMethodID(contact_address_class, "getLabel", "()Ljava/lang/String;");
    }

    if (contact_address_get_street_method == nullptr) {
        contact_address_get_street_method = env->GetMethodID(contact_address_class, "getStreet", "()Ljava/lang/String;");
    }

    if (contact_address_get_city_method == nullptr) {
        contact_address_get_city_method = env->GetMethodID(contact_address_class, "getCity", "()Ljava/lang/String;");
    }

    if (contact_address_get_region_method == nullptr) {
        contact_address_get_region_method = env->GetMethodID(contact_address_class, "getRegion", "()Ljava/lang/String;");
    }

    if (contact_address_get_postal_code_method == nullptr) {
        contact_address_get_postal_code_method = env->GetMethodID(contact_address_class, "getPostalCode", "()Ljava/lang/String;");
    }

    if (contact_address_get_country_method == nullptr) {
        contact_address_get_country_method = env->GetMethodID(contact_address_class, "getCountry", "()Ljava/lang/String;");
    }

    if (contact_address_get_country_code_method == nullptr) {
        contact_address_get_country_code_method = env->GetMethodID(contact_address_class, "getCountryCode", "()Ljava/lang/String;");
    }

    if (contact_organization_get_company_method == nullptr) {
        contact_organization_get_company_method = env->GetMethodID(contact_organization_class, "getCompany", "()Ljava/lang/String;");
    }

    if (contact_organization_get_department_method == nullptr) {
        contact_organization_get_department_method = env->GetMethodID(contact_organization_class, "getDepartment", "()Ljava/lang/String;");
    }

    if (contact_organization_get_title_method == nullptr) {
        contact_organization_get_title_method = env->GetMethodID(contact_organization_class, "getTitle", "()Ljava/lang/String;");
    }

    return
        contact_page_response_get_status_method != nullptr &&
        contact_page_response_get_page_method != nullptr &&
        contact_page_get_contacts_method != nullptr &&
        contact_page_get_next_cursor_method != nullptr &&
        contact_page_get_has_more_method != nullptr &&
        contact_response_get_status_method != nullptr &&
        contact_response_get_contact_method != nullptr &&
        contact_create_response_get_status_method != nullptr &&
        contact_create_response_get_id_method != nullptr &&
        contact_get_id_method != nullptr &&
        contact_get_name_method != nullptr &&
        contact_get_phones_method != nullptr &&
        contact_get_emails_method != nullptr &&
        contact_get_addresses_method != nullptr &&
        contact_get_organization_method != nullptr &&
        contact_get_note_method != nullptr &&
        contact_name_get_given_name_method != nullptr &&
        contact_name_get_middle_name_method != nullptr &&
        contact_name_get_family_name_method != nullptr &&
        contact_name_get_prefix_method != nullptr &&
        contact_name_get_suffix_method != nullptr &&
        contact_name_get_nickname_method != nullptr &&
        contact_name_get_phonetic_given_name_method != nullptr &&
        contact_name_get_phonetic_family_name_method != nullptr &&
        contact_phone_constructor != nullptr &&
        contact_phone_get_label_method != nullptr &&
        contact_phone_get_number_method != nullptr &&
        contact_phone_get_normalized_number_method != nullptr &&
        contact_phone_get_primary_method != nullptr &&
        contact_email_constructor != nullptr &&
        contact_email_get_label_method != nullptr &&
        contact_email_get_address_method != nullptr &&
        contact_email_get_primary_method != nullptr &&
        contact_address_constructor != nullptr &&
        contact_address_get_label_method != nullptr &&
        contact_address_get_street_method != nullptr &&
        contact_address_get_city_method != nullptr &&
        contact_address_get_region_method != nullptr &&
        contact_address_get_postal_code_method != nullptr &&
        contact_address_get_country_method != nullptr &&
        contact_address_get_country_code_method != nullptr &&
        contact_organization_get_company_method != nullptr &&
        contact_organization_get_department_method != nullptr &&
        contact_organization_get_title_method != nullptr;
}

/// Build one Java phone object from one native phone payload.
jobject new_contact_phone(JNIEnv *env, const HostContactPhone &value) {
    jstring label = new_java_string(env, value.label);
    jstring number = new_java_string(env, value.number);
    jstring normalized_number = new_java_string(env, value.normalized_number);
    if (label == nullptr || number == nullptr || normalized_number == nullptr) {
        if (label != nullptr) {
            env->DeleteLocalRef(label);
        }
        if (number != nullptr) {
            env->DeleteLocalRef(number);
        }
        if (normalized_number != nullptr) {
            env->DeleteLocalRef(normalized_number);
        }

        return nullptr;
    }

    jobject phone = env->NewObject(
        contact_phone_class,
        contact_phone_constructor,
        label,
        number,
        normalized_number,
        value.primary ? JNI_TRUE : JNI_FALSE
    );

    env->DeleteLocalRef(label);
    env->DeleteLocalRef(number);
    env->DeleteLocalRef(normalized_number);

    return phone;
}

/// Build one Java email object from one native email payload.
jobject new_contact_email(JNIEnv *env, const HostContactEmail &value) {
    jstring label = new_java_string(env, value.label);
    jstring address = new_java_string(env, value.address);
    if (label == nullptr || address == nullptr) {
        if (label != nullptr) {
            env->DeleteLocalRef(label);
        }
        if (address != nullptr) {
            env->DeleteLocalRef(address);
        }

        return nullptr;
    }

    jobject email = env->NewObject(
        contact_email_class,
        contact_email_constructor,
        label,
        address,
        value.primary ? JNI_TRUE : JNI_FALSE
    );

    env->DeleteLocalRef(label);
    env->DeleteLocalRef(address);

    return email;
}

/// Build one Java address object from one native address payload.
jobject new_contact_address(JNIEnv *env, const HostContactAddress &value) {
    jstring label = new_java_string(env, value.label);
    jstring street = new_java_string(env, value.street);
    jstring city = new_java_string(env, value.city);
    jstring region = new_java_string(env, value.region);
    jstring postal_code = new_java_string(env, value.postal_code);
    jstring country = new_java_string(env, value.country);
    jstring country_code = new_java_string(env, value.country_code);
    if (label == nullptr || street == nullptr || city == nullptr || region == nullptr ||
        postal_code == nullptr || country == nullptr || country_code == nullptr) {
        if (label != nullptr) {
            env->DeleteLocalRef(label);
        }
        if (street != nullptr) {
            env->DeleteLocalRef(street);
        }
        if (city != nullptr) {
            env->DeleteLocalRef(city);
        }
        if (region != nullptr) {
            env->DeleteLocalRef(region);
        }
        if (postal_code != nullptr) {
            env->DeleteLocalRef(postal_code);
        }
        if (country != nullptr) {
            env->DeleteLocalRef(country);
        }
        if (country_code != nullptr) {
            env->DeleteLocalRef(country_code);
        }

        return nullptr;
    }

    jobject address = env->NewObject(
        contact_address_class,
        contact_address_constructor,
        label,
        street,
        city,
        region,
        postal_code,
        country,
        country_code
    );

    env->DeleteLocalRef(label);
    env->DeleteLocalRef(street);
    env->DeleteLocalRef(city);
    env->DeleteLocalRef(region);
    env->DeleteLocalRef(postal_code);
    env->DeleteLocalRef(country);
    env->DeleteLocalRef(country_code);

    return address;
}

/// Build one Java object array from one native phone slice.
jobjectArray new_contact_phone_array(JNIEnv *env, HostContactPhoneSlice values) {
    jobjectArray array = env->NewObjectArray(
        static_cast<jsize>(values.len),
        contact_phone_class,
        nullptr
    );
    if (array == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jobject element = new_contact_phone(env, values.data[index]);
        if (element == nullptr) {
            env->DeleteLocalRef(array);
            return nullptr;
        }

        env->SetObjectArrayElement(array, static_cast<jsize>(index), element);
        env->DeleteLocalRef(element);
    }

    return array;
}

/// Build one Java object array from one native email slice.
jobjectArray new_contact_email_array(JNIEnv *env, HostContactEmailSlice values) {
    jobjectArray array = env->NewObjectArray(
        static_cast<jsize>(values.len),
        contact_email_class,
        nullptr
    );
    if (array == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jobject element = new_contact_email(env, values.data[index]);
        if (element == nullptr) {
            env->DeleteLocalRef(array);
            return nullptr;
        }

        env->SetObjectArrayElement(array, static_cast<jsize>(index), element);
        env->DeleteLocalRef(element);
    }

    return array;
}

/// Build one Java object array from one native address slice.
jobjectArray new_contact_address_array(JNIEnv *env, HostContactAddressSlice values) {
    jobjectArray array = env->NewObjectArray(
        static_cast<jsize>(values.len),
        contact_address_class,
        nullptr
    );
    if (array == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jobject element = new_contact_address(env, values.data[index]);
        if (element == nullptr) {
            env->DeleteLocalRef(array);
            return nullptr;
        }

        env->SetObjectArrayElement(array, static_cast<jsize>(index), element);
        env->DeleteLocalRef(element);
    }

    return array;
}

/// Decode one Java contact-name payload.
HostContactName decode_contact_name(
    JNIEnv *env,
    jobject value,
    std::deque<std::string> *string_storage
) {
    HostContactName name = {};

    jstring given_name = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_given_name_method)
    );
    jstring middle_name = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_middle_name_method)
    );
    jstring family_name = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_family_name_method)
    );
    jstring prefix = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_prefix_method)
    );
    jstring suffix = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_suffix_method)
    );
    jstring nickname = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_nickname_method)
    );
    jstring phonetic_given_name = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_phonetic_given_name_method)
    );
    jstring phonetic_family_name = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_name_get_phonetic_family_name_method)
    );

    name.given_name = string_ref_from_java(env, given_name, string_storage);
    name.middle_name = string_ref_from_java(env, middle_name, string_storage);
    name.family_name = string_ref_from_java(env, family_name, string_storage);
    name.prefix = string_ref_from_java(env, prefix, string_storage);
    name.suffix = string_ref_from_java(env, suffix, string_storage);
    name.nickname = string_ref_from_java(env, nickname, string_storage);
    name.phonetic_given_name = string_ref_from_java(env, phonetic_given_name, string_storage);
    name.phonetic_family_name = string_ref_from_java(env, phonetic_family_name, string_storage);

    return name;
}

/// Decode one Java phone object into one native payload.
HostContactPhone decode_contact_phone(
    JNIEnv *env,
    jobject value,
    std::deque<std::string> *string_storage
) {
    HostContactPhone phone = {};

    jstring label = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_phone_get_label_method)
    );
    jstring number = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_phone_get_number_method)
    );
    jstring normalized_number = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_phone_get_normalized_number_method)
    );

    phone.label = string_ref_from_java(env, label, string_storage);
    phone.number = string_ref_from_java(env, number, string_storage);
    phone.normalized_number = string_ref_from_java(env, normalized_number, string_storage);
    phone.primary = env->CallBooleanMethod(value, contact_phone_get_primary_method) == JNI_TRUE;

    return phone;
}

/// Decode one Java email object into one native payload.
HostContactEmail decode_contact_email(
    JNIEnv *env,
    jobject value,
    std::deque<std::string> *string_storage
) {
    HostContactEmail email = {};

    jstring label = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_email_get_label_method)
    );
    jstring address = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_email_get_address_method)
    );

    email.label = string_ref_from_java(env, label, string_storage);
    email.address = string_ref_from_java(env, address, string_storage);
    email.primary = env->CallBooleanMethod(value, contact_email_get_primary_method) == JNI_TRUE;

    return email;
}

/// Decode one Java address object into one native payload.
HostContactAddress decode_contact_address(
    JNIEnv *env,
    jobject value,
    std::deque<std::string> *string_storage
) {
    HostContactAddress address = {};

    jstring label = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_address_get_label_method)
    );
    jstring street = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_address_get_street_method)
    );
    jstring city = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_address_get_city_method)
    );
    jstring region = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_address_get_region_method)
    );
    jstring postal_code = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_address_get_postal_code_method)
    );
    jstring country = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_address_get_country_method)
    );
    jstring country_code = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_address_get_country_code_method)
    );

    address.label = string_ref_from_java(env, label, string_storage);
    address.street = string_ref_from_java(env, street, string_storage);
    address.city = string_ref_from_java(env, city, string_storage);
    address.region = string_ref_from_java(env, region, string_storage);
    address.postal_code = string_ref_from_java(env, postal_code, string_storage);
    address.country = string_ref_from_java(env, country, string_storage);
    address.country_code = string_ref_from_java(env, country_code, string_storage);

    return address;
}

/// Decode one Java list of phones into one native slice.
HostContactPhoneSlice decode_contact_phones(
    JNIEnv *env,
    jobject list,
    std::deque<std::string> *string_storage,
    std::deque<std::vector<HostContactPhone>> *phone_storage
) {
    HostContactPhoneSlice slice = {};
    if (list == nullptr) {
        return slice;
    }

    jint count = env->CallIntMethod(list, list_size_method);
    phone_storage->emplace_back();
    std::vector<HostContactPhone> &phones = phone_storage->back();
    phones.reserve(static_cast<size_t>(count));

    for (jint index = 0; index < count; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        phones.push_back(decode_contact_phone(env, value, string_storage));
        env->DeleteLocalRef(value);
    }

    slice.data = phones.data();
    slice.len = static_cast<uint32_t>(phones.size());

    return slice;
}

/// Decode one Java list of emails into one native slice.
HostContactEmailSlice decode_contact_emails(
    JNIEnv *env,
    jobject list,
    std::deque<std::string> *string_storage,
    std::deque<std::vector<HostContactEmail>> *email_storage
) {
    HostContactEmailSlice slice = {};
    if (list == nullptr) {
        return slice;
    }

    jint count = env->CallIntMethod(list, list_size_method);
    email_storage->emplace_back();
    std::vector<HostContactEmail> &emails = email_storage->back();
    emails.reserve(static_cast<size_t>(count));

    for (jint index = 0; index < count; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        emails.push_back(decode_contact_email(env, value, string_storage));
        env->DeleteLocalRef(value);
    }

    slice.data = emails.data();
    slice.len = static_cast<uint32_t>(emails.size());

    return slice;
}

/// Decode one Java list of addresses into one native slice.
HostContactAddressSlice decode_contact_addresses(
    JNIEnv *env,
    jobject list,
    std::deque<std::string> *string_storage,
    std::deque<std::vector<HostContactAddress>> *address_storage
) {
    HostContactAddressSlice slice = {};
    if (list == nullptr) {
        return slice;
    }

    jint count = env->CallIntMethod(list, list_size_method);
    address_storage->emplace_back();
    std::vector<HostContactAddress> &addresses = address_storage->back();
    addresses.reserve(static_cast<size_t>(count));

    for (jint index = 0; index < count; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        addresses.push_back(decode_contact_address(env, value, string_storage));
        env->DeleteLocalRef(value);
    }

    slice.data = addresses.data();
    slice.len = static_cast<uint32_t>(addresses.size());

    return slice;
}

/// Decode one Java organization object into one native payload.
HostContactOrganization decode_contact_organization(
    JNIEnv *env,
    jobject value,
    std::deque<std::string> *string_storage
) {
    HostContactOrganization organization = {};

    jstring company = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_organization_get_company_method)
    );
    jstring department = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_organization_get_department_method)
    );
    jstring title = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_organization_get_title_method)
    );

    organization.company = string_ref_from_java(env, company, string_storage);
    organization.department = string_ref_from_java(env, department, string_storage);
    organization.title = string_ref_from_java(env, title, string_storage);

    return organization;
}

/// Decode one Java contact object into one native payload.
HostContact decode_contact(
    JNIEnv *env,
    jobject value,
    std::deque<std::string> *string_storage,
    std::deque<std::vector<HostContactPhone>> *phone_storage,
    std::deque<std::vector<HostContactEmail>> *email_storage,
    std::deque<std::vector<HostContactAddress>> *address_storage
) {
    HostContact contact = {};

    jstring identifier = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_get_id_method)
    );
    jobject name = env->CallObjectMethod(value, contact_get_name_method);
    jobject phones = env->CallObjectMethod(value, contact_get_phones_method);
    jobject emails = env->CallObjectMethod(value, contact_get_emails_method);
    jobject addresses = env->CallObjectMethod(value, contact_get_addresses_method);
    jobject organization = env->CallObjectMethod(value, contact_get_organization_method);
    jstring note = reinterpret_cast<jstring>(
        env->CallObjectMethod(value, contact_get_note_method)
    );

    contact.id = string_ref_from_java(env, identifier, string_storage);
    contact.name = decode_contact_name(env, name, string_storage);
    contact.phones = decode_contact_phones(env, phones, string_storage, phone_storage);
    contact.emails = decode_contact_emails(env, emails, string_storage, email_storage);
    contact.addresses = decode_contact_addresses(env, addresses, string_storage, address_storage);
    contact.organization = decode_contact_organization(env, organization, string_storage);
    contact.note = string_ref_from_java(env, note, string_storage);

    env->DeleteLocalRef(name);
    env->DeleteLocalRef(phones);
    env->DeleteLocalRef(emails);
    env->DeleteLocalRef(addresses);
    env->DeleteLocalRef(organization);

    return contact;
}

/// Decode one Java contact-page response into one native payload.
uint32_t decode_contact_page_response(
    JNIEnv *env,
    jobject response,
    HostContactPage *output_page
) {
    jint status = env->CallIntMethod(response, contact_page_response_get_status_method);
    if (status != static_cast<jint>(HOST_STATUS_OK)) {
        return static_cast<uint32_t>(status);
    }

    jobject page = env->CallObjectMethod(response, contact_page_response_get_page_method);
    if (page == nullptr) {
        return static_cast<uint32_t>(status);
    }

    result_string_storage.clear();
    result_phone_storage.clear();
    result_email_storage.clear();
    result_address_storage.clear();
    result_contact_storage.clear();

    jobject contacts = env->CallObjectMethod(page, contact_page_get_contacts_method);
    jint count = env->CallIntMethod(contacts, list_size_method);
    result_contact_storage.reserve(static_cast<size_t>(count));

    for (jint index = 0; index < count; index += 1) {
        jobject value = env->CallObjectMethod(contacts, list_get_method, index);
        result_contact_storage.push_back(
            decode_contact(
                env,
                value,
                &result_string_storage,
                &result_phone_storage,
                &result_email_storage,
                &result_address_storage
            )
        );
        env->DeleteLocalRef(value);
    }

    jstring next_cursor = reinterpret_cast<jstring>(
        env->CallObjectMethod(page, contact_page_get_next_cursor_method)
    );
    jboolean has_more = env->CallBooleanMethod(page, contact_page_get_has_more_method);

    output_page->contacts.data = result_contact_storage.data();
    output_page->contacts.len = static_cast<uint32_t>(result_contact_storage.size());
    output_page->next_cursor = string_ref_from_java(env, next_cursor, &result_string_storage);
    output_page->has_more = has_more == JNI_TRUE;

    env->DeleteLocalRef(contacts);
    env->DeleteLocalRef(page);

    return static_cast<uint32_t>(status);
}

/// Decode one Java contact-read response into one native payload.
uint32_t decode_contact_response(
    JNIEnv *env,
    jobject response,
    HostContact *output_contact
) {
    jint status = env->CallIntMethod(response, contact_response_get_status_method);
    if (status != static_cast<jint>(HOST_STATUS_OK)) {
        return static_cast<uint32_t>(status);
    }

    jobject contact = env->CallObjectMethod(response, contact_response_get_contact_method);
    if (contact == nullptr) {
        return static_cast<uint32_t>(status);
    }

    result_string_storage.clear();
    result_phone_storage.clear();
    result_email_storage.clear();
    result_address_storage.clear();

    *output_contact = decode_contact(
        env,
        contact,
        &result_string_storage,
        &result_phone_storage,
        &result_email_storage,
        &result_address_storage
    );

    env->DeleteLocalRef(contact);

    return static_cast<uint32_t>(status);
}

}

/// Resolve the contact bridge methods from one runtime bridge instance.
bool resolve_contact_methods(JNIEnv *env, jobject bridge) {
    return
        resolve_list_methods(env) &&
        resolve_bridge_methods(env, bridge) &&
        resolve_contact_types(env);
}

/// Call the contact-list entrypoint on one registered bridge.
uint32_t call_contact_list_for_session(
    uint64_t session_handle,
    HostContactQuery query,
    HostContactPage *output_page
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring cursor = query.has_cursor ? new_java_string(env, query.cursor) : nullptr;
    if (query.has_cursor && cursor == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        list_contacts_method,
        cursor,
        query.has_limit ? JNI_TRUE : JNI_FALSE,
        static_cast<jint>(query.limit),
        query.include_phones ? JNI_TRUE : JNI_FALSE,
        query.include_emails ? JNI_TRUE : JNI_FALSE,
        query.include_addresses ? JNI_TRUE : JNI_FALSE,
        query.include_organization ? JNI_TRUE : JNI_FALSE,
        query.include_notes ? JNI_TRUE : JNI_FALSE
    );

    env->DeleteLocalRef(bridge);
    if (cursor != nullptr) {
        env->DeleteLocalRef(cursor);
    }

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    uint32_t status = decode_contact_page_response(env, response, output_page);
    env->DeleteLocalRef(response);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call the contact-search entrypoint on one registered bridge.
uint32_t call_contact_search_for_session(
    uint64_t session_handle,
    NativeStringRef query_text,
    HostContactQuery query,
    HostContactPage *output_page
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring query_text_value = new_java_string(env, query_text);
    jstring cursor = query.has_cursor ? new_java_string(env, query.cursor) : nullptr;
    if (query_text_value == nullptr || (query.has_cursor && cursor == nullptr)) {
        env->DeleteLocalRef(bridge);
        if (query_text_value != nullptr) {
            env->DeleteLocalRef(query_text_value);
        }
        if (cursor != nullptr) {
            env->DeleteLocalRef(cursor);
        }
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        search_contacts_method,
        query_text_value,
        cursor,
        query.has_limit ? JNI_TRUE : JNI_FALSE,
        static_cast<jint>(query.limit),
        query.include_phones ? JNI_TRUE : JNI_FALSE,
        query.include_emails ? JNI_TRUE : JNI_FALSE,
        query.include_addresses ? JNI_TRUE : JNI_FALSE,
        query.include_organization ? JNI_TRUE : JNI_FALSE,
        query.include_notes ? JNI_TRUE : JNI_FALSE
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(query_text_value);
    if (cursor != nullptr) {
        env->DeleteLocalRef(cursor);
    }

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    uint32_t status = decode_contact_page_response(env, response, output_page);
    env->DeleteLocalRef(response);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call the contact-read entrypoint on one registered bridge.
uint32_t call_contact_read_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostContact *output_contact
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring identifier_value = new_java_string(env, identifier);
    if (identifier_value == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        read_contact_method,
        identifier_value
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(identifier_value);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    uint32_t status = decode_contact_response(env, response, output_contact);
    env->DeleteLocalRef(response);

    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call the contact-create entrypoint on one registered bridge.
uint32_t call_contact_create_for_session(
    uint64_t session_handle,
    HostContactDraft draft,
    NativeStringRef *output_identifier
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring given_name = new_java_string(env, draft.name.given_name);
    jstring middle_name = new_java_string(env, draft.name.middle_name);
    jstring family_name = new_java_string(env, draft.name.family_name);
    jstring prefix = new_java_string(env, draft.name.prefix);
    jstring suffix = new_java_string(env, draft.name.suffix);
    jstring nickname = new_java_string(env, draft.name.nickname);
    jstring phonetic_given_name = new_java_string(env, draft.name.phonetic_given_name);
    jstring phonetic_family_name = new_java_string(env, draft.name.phonetic_family_name);
    jobjectArray phones = new_contact_phone_array(env, draft.phones);
    jobjectArray emails = new_contact_email_array(env, draft.emails);
    jobjectArray addresses = new_contact_address_array(env, draft.addresses);
    jstring company = new_java_string(env, draft.organization.company);
    jstring department = new_java_string(env, draft.organization.department);
    jstring title = new_java_string(env, draft.organization.title);
    jstring note = new_java_string(env, draft.note);

    if (given_name == nullptr || middle_name == nullptr || family_name == nullptr ||
        prefix == nullptr || suffix == nullptr || nickname == nullptr ||
        phonetic_given_name == nullptr || phonetic_family_name == nullptr ||
        phones == nullptr || emails == nullptr || addresses == nullptr ||
        company == nullptr || department == nullptr || title == nullptr || note == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        create_contact_method,
        given_name,
        middle_name,
        family_name,
        prefix,
        suffix,
        nickname,
        phonetic_given_name,
        phonetic_family_name,
        phones,
        emails,
        addresses,
        company,
        department,
        title,
        note
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(given_name);
    env->DeleteLocalRef(middle_name);
    env->DeleteLocalRef(family_name);
    env->DeleteLocalRef(prefix);
    env->DeleteLocalRef(suffix);
    env->DeleteLocalRef(nickname);
    env->DeleteLocalRef(phonetic_given_name);
    env->DeleteLocalRef(phonetic_family_name);
    env->DeleteLocalRef(phones);
    env->DeleteLocalRef(emails);
    env->DeleteLocalRef(addresses);
    env->DeleteLocalRef(company);
    env->DeleteLocalRef(department);
    env->DeleteLocalRef(title);
    env->DeleteLocalRef(note);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(response, contact_create_response_get_status_method);
    if (status == static_cast<jint>(HOST_STATUS_OK)) {
        jstring identifier_value = reinterpret_cast<jstring>(
            env->CallObjectMethod(response, contact_create_response_get_id_method)
        );
        result_string_storage.clear();
        *output_identifier = string_ref_from_java(env, identifier_value, &result_string_storage);
    }

    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    return static_cast<uint32_t>(status);
}

/// Call the contact-update entrypoint on one registered bridge.
uint32_t call_contact_update_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostContactDraft draft
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring identifier_value = new_java_string(env, identifier);
    jstring given_name = new_java_string(env, draft.name.given_name);
    jstring middle_name = new_java_string(env, draft.name.middle_name);
    jstring family_name = new_java_string(env, draft.name.family_name);
    jstring prefix = new_java_string(env, draft.name.prefix);
    jstring suffix = new_java_string(env, draft.name.suffix);
    jstring nickname = new_java_string(env, draft.name.nickname);
    jstring phonetic_given_name = new_java_string(env, draft.name.phonetic_given_name);
    jstring phonetic_family_name = new_java_string(env, draft.name.phonetic_family_name);
    jobjectArray phones = new_contact_phone_array(env, draft.phones);
    jobjectArray emails = new_contact_email_array(env, draft.emails);
    jobjectArray addresses = new_contact_address_array(env, draft.addresses);
    jstring company = new_java_string(env, draft.organization.company);
    jstring department = new_java_string(env, draft.organization.department);
    jstring title = new_java_string(env, draft.organization.title);
    jstring note = new_java_string(env, draft.note);

    if (identifier_value == nullptr || given_name == nullptr || middle_name == nullptr ||
        family_name == nullptr || prefix == nullptr || suffix == nullptr ||
        nickname == nullptr || phonetic_given_name == nullptr ||
        phonetic_family_name == nullptr || phones == nullptr || emails == nullptr ||
        addresses == nullptr || company == nullptr || department == nullptr ||
        title == nullptr || note == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        update_contact_method,
        identifier_value,
        given_name,
        middle_name,
        family_name,
        prefix,
        suffix,
        nickname,
        phonetic_given_name,
        phonetic_family_name,
        phones,
        emails,
        addresses,
        company,
        department,
        title,
        note
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(identifier_value);
    env->DeleteLocalRef(given_name);
    env->DeleteLocalRef(middle_name);
    env->DeleteLocalRef(family_name);
    env->DeleteLocalRef(prefix);
    env->DeleteLocalRef(suffix);
    env->DeleteLocalRef(nickname);
    env->DeleteLocalRef(phonetic_given_name);
    env->DeleteLocalRef(phonetic_family_name);
    env->DeleteLocalRef(phones);
    env->DeleteLocalRef(emails);
    env->DeleteLocalRef(addresses);
    env->DeleteLocalRef(company);
    env->DeleteLocalRef(department);
    env->DeleteLocalRef(title);
    env->DeleteLocalRef(note);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    detach_jni_thread(did_attach_thread);

    return static_cast<uint32_t>(status);
}

/// Call the contact-delete entrypoint on one registered bridge.
uint32_t call_contact_delete_for_session(
    uint64_t session_handle,
    NativeStringRef identifier
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jstring identifier_value = new_java_string(env, identifier);
    if (identifier_value == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        delete_contact_method,
        identifier_value
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(identifier_value);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    detach_jni_thread(did_attach_thread);

    return static_cast<uint32_t>(status);
}
