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
jclass array_list_class = nullptr;
jclass enum_class = nullptr;
jclass integer_class = nullptr;
jclass long_class = nullptr;
jclass calendar_list_response_class = nullptr;
jclass calendar_event_list_response_class = nullptr;
jclass calendar_event_response_class = nullptr;
jclass calendar_event_create_response_class = nullptr;
jclass calendar_descriptor_class = nullptr;
jclass calendar_event_class = nullptr;
jclass calendar_query_class = nullptr;
jclass calendar_draft_class = nullptr;
jclass calendar_recurrence_rule_class = nullptr;
jclass calendar_recurrence_weekday_class = nullptr;
jclass calendar_attendee_class = nullptr;
jclass calendar_absolute_reminder_class = nullptr;
jclass calendar_relative_reminder_class = nullptr;
jclass calendar_reminder_absolute_class = nullptr;
jclass calendar_reminder_relative_class = nullptr;
jclass calendar_access_class = nullptr;
jclass calendar_availability_class = nullptr;
jclass calendar_participant_status_class = nullptr;
jclass calendar_recurrence_frequency_class = nullptr;
jmethodID list_calendars_method = nullptr;
jmethodID list_calendar_events_method = nullptr;
jmethodID read_calendar_event_method = nullptr;
jmethodID create_calendar_event_method = nullptr;
jmethodID update_calendar_event_method = nullptr;
jmethodID delete_calendar_event_method = nullptr;
jmethodID list_size_method = nullptr;
jmethodID list_get_method = nullptr;
jmethodID array_list_constructor = nullptr;
jmethodID array_list_add_method = nullptr;
jmethodID enum_ordinal_method = nullptr;
jmethodID integer_value_of_method = nullptr;
jmethodID integer_int_value_method = nullptr;
jmethodID long_value_of_method = nullptr;
jmethodID long_long_value_method = nullptr;
jmethodID calendar_list_response_get_status_method = nullptr;
jmethodID calendar_list_response_get_calendars_method = nullptr;
jmethodID calendar_event_list_response_get_status_method = nullptr;
jmethodID calendar_event_list_response_get_events_method = nullptr;
jmethodID calendar_event_response_get_status_method = nullptr;
jmethodID calendar_event_response_get_event_method = nullptr;
jmethodID calendar_event_create_response_get_status_method = nullptr;
jmethodID calendar_event_create_response_get_id_method = nullptr;
jmethodID calendar_descriptor_get_id_method = nullptr;
jmethodID calendar_descriptor_get_title_method = nullptr;
jmethodID calendar_descriptor_get_source_method = nullptr;
jmethodID calendar_descriptor_get_owner_method = nullptr;
jmethodID calendar_descriptor_get_color_argb_method = nullptr;
jmethodID calendar_descriptor_get_primary_method = nullptr;
jmethodID calendar_descriptor_get_access_method = nullptr;
jmethodID calendar_event_get_id_method = nullptr;
jmethodID calendar_event_get_calendar_id_method = nullptr;
jmethodID calendar_event_get_title_method = nullptr;
jmethodID calendar_event_get_notes_method = nullptr;
jmethodID calendar_event_get_location_method = nullptr;
jmethodID calendar_event_get_start_unix_ns_method = nullptr;
jmethodID calendar_event_get_end_unix_ns_method = nullptr;
jmethodID calendar_event_get_all_day_method = nullptr;
jmethodID calendar_event_get_canceled_method = nullptr;
jmethodID calendar_event_get_time_zone_method = nullptr;
jmethodID calendar_event_get_availability_method = nullptr;
jmethodID calendar_event_get_url_method = nullptr;
jmethodID calendar_event_get_organizer_name_method = nullptr;
jmethodID calendar_event_get_organizer_email_method = nullptr;
jmethodID calendar_event_get_recurring_method = nullptr;
jmethodID calendar_event_get_recurrence_master_id_method = nullptr;
jmethodID calendar_event_get_recurrence_id_unix_ns_method = nullptr;
jmethodID calendar_event_get_recurrence_rule_method = nullptr;
jmethodID calendar_event_get_attendees_method = nullptr;
jmethodID calendar_event_get_reminders_method = nullptr;
jmethodID calendar_query_constructor = nullptr;
jmethodID calendar_draft_constructor = nullptr;
jmethodID recurrence_rule_constructor = nullptr;
jmethodID recurrence_rule_get_frequency_method = nullptr;
jmethodID recurrence_rule_get_interval_method = nullptr;
jmethodID recurrence_rule_get_count_method = nullptr;
jmethodID recurrence_rule_get_until_unix_ns_method = nullptr;
jmethodID recurrence_rule_get_by_week_days_method = nullptr;
jmethodID recurrence_rule_get_by_weekday_ordinals_method = nullptr;
jmethodID recurrence_rule_get_by_month_days_method = nullptr;
jmethodID recurrence_rule_get_by_months_method = nullptr;
jmethodID recurrence_rule_get_by_year_days_method = nullptr;
jmethodID recurrence_rule_get_by_week_numbers_method = nullptr;
jmethodID recurrence_rule_get_by_set_positions_method = nullptr;
jmethodID recurrence_weekday_constructor = nullptr;
jmethodID recurrence_weekday_get_day_method = nullptr;
jmethodID recurrence_weekday_get_week_number_method = nullptr;
jmethodID attendee_constructor = nullptr;
jmethodID attendee_get_id_method = nullptr;
jmethodID attendee_get_name_method = nullptr;
jmethodID attendee_get_email_method = nullptr;
jmethodID attendee_get_optional_method = nullptr;
jmethodID attendee_get_organizer_method = nullptr;
jmethodID attendee_get_response_status_method = nullptr;
jmethodID absolute_reminder_constructor = nullptr;
jmethodID absolute_reminder_get_absolute_unix_ns_method = nullptr;
jmethodID relative_reminder_constructor = nullptr;
jmethodID relative_reminder_get_minutes_before_start_method = nullptr;
jmethodID reminder_absolute_constructor = nullptr;
jmethodID reminder_relative_constructor = nullptr;
jmethodID reminder_absolute_get_value_method = nullptr;
jmethodID reminder_relative_get_value_method = nullptr;
jfieldID calendar_availability_busy_field = nullptr;
jfieldID calendar_availability_free_field = nullptr;
jfieldID calendar_availability_tentative_field = nullptr;
jfieldID calendar_availability_out_of_office_field = nullptr;
jfieldID calendar_availability_unavailable_field = nullptr;
jfieldID calendar_availability_unknown_field = nullptr;
jfieldID calendar_participant_status_unknown_field = nullptr;
jfieldID calendar_participant_status_pending_field = nullptr;
jfieldID calendar_participant_status_accepted_field = nullptr;
jfieldID calendar_participant_status_tentative_field = nullptr;
jfieldID calendar_participant_status_declined_field = nullptr;
jfieldID calendar_participant_status_delegated_field = nullptr;
jfieldID calendar_participant_status_completed_field = nullptr;
jfieldID calendar_participant_status_in_process_field = nullptr;
jfieldID calendar_recurrence_frequency_daily_field = nullptr;
jfieldID calendar_recurrence_frequency_weekly_field = nullptr;
jfieldID calendar_recurrence_frequency_monthly_field = nullptr;
jfieldID calendar_recurrence_frequency_yearly_field = nullptr;
thread_local std::deque<std::string> result_string_storage;
thread_local std::deque<std::vector<HostCalendarDescriptor>> result_descriptor_storage;
thread_local std::deque<std::vector<HostCalendarEvent>> result_event_storage;
thread_local std::deque<std::vector<HostCalendarAttendee>> result_attendee_storage;
thread_local std::deque<std::vector<HostCalendarReminder>> result_reminder_storage;
thread_local std::deque<std::vector<HostCalendarRecurrenceWeekday>> result_weekday_storage;
thread_local std::deque<std::vector<uint8_t>> result_u8_storage;
thread_local std::deque<std::vector<int8_t>> result_i8_storage;
thread_local std::deque<std::vector<int16_t>> result_i16_storage;

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

/// Convert one nullable Java string into one optional native string reference.
HostOptionalStringRef optional_string_ref_from_java(
    JNIEnv *env,
    jstring value,
    std::deque<std::string> *storage
) {
    return HostOptionalStringRef {
        .has_value = value != nullptr,
        .value = string_ref_from_java(env, value, storage),
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

/// Resolve the shared java.util.ArrayList helpers.
bool resolve_array_list_methods(JNIEnv *env) {
    if (array_list_class == nullptr) {
        jclass local_class = env->FindClass("java/util/ArrayList");
        if (local_class == nullptr) {
            return false;
        }

        array_list_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (array_list_class == nullptr) {
            return false;
        }
    }

    if (array_list_constructor == nullptr) {
        array_list_constructor = env->GetMethodID(array_list_class, "<init>", "(I)V");
    }

    if (array_list_add_method == nullptr) {
        array_list_add_method = env->GetMethodID(
            array_list_class,
            "add",
            "(Ljava/lang/Object;)Z"
        );
    }

    return array_list_constructor != nullptr && array_list_add_method != nullptr;
}

/// Resolve the shared java.lang.Enum helpers.
bool resolve_enum_methods(JNIEnv *env) {
    if (enum_class == nullptr) {
        jclass local_class = env->FindClass("java/lang/Enum");
        if (local_class == nullptr) {
            return false;
        }

        enum_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (enum_class == nullptr) {
            return false;
        }
    }

    if (enum_ordinal_method == nullptr) {
        enum_ordinal_method = env->GetMethodID(enum_class, "ordinal", "()I");
    }

    return enum_ordinal_method != nullptr;
}

/// Resolve the shared boxed-number helpers.
bool resolve_number_methods(JNIEnv *env) {
    if (integer_class == nullptr) {
        jclass local_class = env->FindClass("java/lang/Integer");
        if (local_class == nullptr) {
            return false;
        }

        integer_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (integer_class == nullptr) {
            return false;
        }
    }

    if (long_class == nullptr) {
        jclass local_class = env->FindClass("java/lang/Long");
        if (local_class == nullptr) {
            return false;
        }

        long_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (long_class == nullptr) {
            return false;
        }
    }

    if (integer_value_of_method == nullptr) {
        integer_value_of_method = env->GetStaticMethodID(
            integer_class,
            "valueOf",
            "(I)Ljava/lang/Integer;"
        );
    }

    if (integer_int_value_method == nullptr) {
        integer_int_value_method = env->GetMethodID(integer_class, "intValue", "()I");
    }

    if (long_value_of_method == nullptr) {
        long_value_of_method = env->GetStaticMethodID(
            long_class,
            "valueOf",
            "(J)Ljava/lang/Long;"
        );
    }

    if (long_long_value_method == nullptr) {
        long_long_value_method = env->GetMethodID(long_class, "longValue", "()J");
    }

    return
        integer_value_of_method != nullptr &&
        integer_int_value_method != nullptr &&
        long_value_of_method != nullptr &&
        long_long_value_method != nullptr;
}

/// Resolve the calendar bridge entrypoints from one runtime bridge instance.
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

    if (list_calendars_method == nullptr) {
        list_calendars_method = env->GetMethodID(
            bridge_class,
            "calendarList",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarListResponse;"
        );
    }

    if (list_calendar_events_method == nullptr) {
        list_calendar_events_method = env->GetMethodID(
            bridge_class,
            "calendarEventList",
            "(Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventQuery;)Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventListResponse;"
        );
    }

    if (read_calendar_event_method == nullptr) {
        read_calendar_event_method = env->GetMethodID(
            bridge_class,
            "calendarEventRead",
            "(Ljava/lang/String;)Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventReadResponse;"
        );
    }

    if (create_calendar_event_method == nullptr) {
        create_calendar_event_method = env->GetMethodID(
            bridge_class,
            "calendarEventCreate",
            "(Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventDraft;)Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventCreateResponse;"
        );
    }

    if (update_calendar_event_method == nullptr) {
        update_calendar_event_method = env->GetMethodID(
            bridge_class,
            "calendarEventUpdate",
            "(Ljava/lang/String;Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventDraft;)I"
        );
    }

    if (delete_calendar_event_method == nullptr) {
        delete_calendar_event_method = env->GetMethodID(
            bridge_class,
            "calendarEventDelete",
            "(Ljava/lang/String;)I"
        );
    }

    return
        list_calendars_method != nullptr &&
        list_calendar_events_method != nullptr &&
        read_calendar_event_method != nullptr &&
        create_calendar_event_method != nullptr &&
        update_calendar_event_method != nullptr &&
        delete_calendar_event_method != nullptr;
}

/// Resolve the response and payload accessors for one calendar callback path.
bool resolve_calendar_types(JNIEnv *env) {
    if (calendar_list_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarListResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_list_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_list_response_class == nullptr) {
            return false;
        }
    }

    if (calendar_event_list_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventListResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_event_list_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_event_list_response_class == nullptr) {
            return false;
        }
    }

    if (calendar_event_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventReadResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_event_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_event_response_class == nullptr) {
            return false;
        }
    }

    if (calendar_event_create_response_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventCreateResponse"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_event_create_response_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_event_create_response_class == nullptr) {
            return false;
        }
    }

    if (calendar_descriptor_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarDescriptor"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_descriptor_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_descriptor_class == nullptr) {
            return false;
        }
    }

    if (calendar_event_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarEvent"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_event_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_event_class == nullptr) {
            return false;
        }
    }

    if (calendar_query_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventQuery"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_query_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_query_class == nullptr) {
            return false;
        }
    }

    if (calendar_draft_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarEventDraft"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_draft_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_draft_class == nullptr) {
            return false;
        }
    }

    if (calendar_recurrence_rule_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceRule"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_recurrence_rule_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_recurrence_rule_class == nullptr) {
            return false;
        }
    }

    if (calendar_recurrence_weekday_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceWeekday"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_recurrence_weekday_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_recurrence_weekday_class == nullptr) {
            return false;
        }
    }

    if (calendar_attendee_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarAttendee"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_attendee_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_attendee_class == nullptr) {
            return false;
        }
    }

    if (calendar_absolute_reminder_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarAbsoluteReminder"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_absolute_reminder_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_absolute_reminder_class == nullptr) {
            return false;
        }
    }

    if (calendar_relative_reminder_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarRelativeReminder"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_relative_reminder_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_relative_reminder_class == nullptr) {
            return false;
        }
    }

    if (calendar_reminder_absolute_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarReminder$Absolute"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_reminder_absolute_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_reminder_absolute_class == nullptr) {
            return false;
        }
    }

    if (calendar_reminder_relative_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarReminder$Relative"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_reminder_relative_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_reminder_relative_class == nullptr) {
            return false;
        }
    }

    if (calendar_access_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarAccess"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_access_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_access_class == nullptr) {
            return false;
        }
    }

    if (calendar_availability_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_availability_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_availability_class == nullptr) {
            return false;
        }
    }

    if (calendar_participant_status_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_participant_status_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_participant_status_class == nullptr) {
            return false;
        }
    }

    if (calendar_recurrence_frequency_class == nullptr) {
        jclass local_class = env->FindClass(
            "dev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceFrequency"
        );
        if (local_class == nullptr) {
            return false;
        }

        calendar_recurrence_frequency_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));
        env->DeleteLocalRef(local_class);
        if (calendar_recurrence_frequency_class == nullptr) {
            return false;
        }
    }

    if (calendar_list_response_get_status_method == nullptr) {
        calendar_list_response_get_status_method = env->GetMethodID(
            calendar_list_response_class,
            "getStatus",
            "()I"
        );
    }

    if (calendar_list_response_get_calendars_method == nullptr) {
        calendar_list_response_get_calendars_method = env->GetMethodID(
            calendar_list_response_class,
            "getCalendars",
            "()Ljava/util/List;"
        );
    }

    if (calendar_event_list_response_get_status_method == nullptr) {
        calendar_event_list_response_get_status_method = env->GetMethodID(
            calendar_event_list_response_class,
            "getStatus",
            "()I"
        );
    }

    if (calendar_event_list_response_get_events_method == nullptr) {
        calendar_event_list_response_get_events_method = env->GetMethodID(
            calendar_event_list_response_class,
            "getEvents",
            "()Ljava/util/List;"
        );
    }

    if (calendar_event_response_get_status_method == nullptr) {
        calendar_event_response_get_status_method = env->GetMethodID(
            calendar_event_response_class,
            "getStatus",
            "()I"
        );
    }

    if (calendar_event_response_get_event_method == nullptr) {
        calendar_event_response_get_event_method = env->GetMethodID(
            calendar_event_response_class,
            "getEvent",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarEvent;"
        );
    }

    if (calendar_event_create_response_get_status_method == nullptr) {
        calendar_event_create_response_get_status_method = env->GetMethodID(
            calendar_event_create_response_class,
            "getStatus",
            "()I"
        );
    }

    if (calendar_event_create_response_get_id_method == nullptr) {
        calendar_event_create_response_get_id_method = env->GetMethodID(
            calendar_event_create_response_class,
            "getId",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_descriptor_get_id_method == nullptr) {
        calendar_descriptor_get_id_method = env->GetMethodID(
            calendar_descriptor_class,
            "getId",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_descriptor_get_title_method == nullptr) {
        calendar_descriptor_get_title_method = env->GetMethodID(
            calendar_descriptor_class,
            "getTitle",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_descriptor_get_source_method == nullptr) {
        calendar_descriptor_get_source_method = env->GetMethodID(
            calendar_descriptor_class,
            "getSource",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_descriptor_get_owner_method == nullptr) {
        calendar_descriptor_get_owner_method = env->GetMethodID(
            calendar_descriptor_class,
            "getOwner",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_descriptor_get_color_argb_method == nullptr) {
        calendar_descriptor_get_color_argb_method = env->GetMethodID(
            calendar_descriptor_class,
            "getColorArgb",
            "()I"
        );
    }

    if (calendar_descriptor_get_primary_method == nullptr) {
        calendar_descriptor_get_primary_method = env->GetMethodID(
            calendar_descriptor_class,
            "getPrimary",
            "()Z"
        );
    }

    if (calendar_descriptor_get_access_method == nullptr) {
        calendar_descriptor_get_access_method = env->GetMethodID(
            calendar_descriptor_class,
            "getAccess",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAccess;"
        );
    }

    if (calendar_event_get_id_method == nullptr) {
        calendar_event_get_id_method = env->GetMethodID(
            calendar_event_class,
            "getId",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_calendar_id_method == nullptr) {
        calendar_event_get_calendar_id_method = env->GetMethodID(
            calendar_event_class,
            "getCalendarId",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_title_method == nullptr) {
        calendar_event_get_title_method = env->GetMethodID(
            calendar_event_class,
            "getTitle",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_notes_method == nullptr) {
        calendar_event_get_notes_method = env->GetMethodID(
            calendar_event_class,
            "getNotes",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_location_method == nullptr) {
        calendar_event_get_location_method = env->GetMethodID(
            calendar_event_class,
            "getLocation",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_start_unix_ns_method == nullptr) {
        calendar_event_get_start_unix_ns_method = env->GetMethodID(
            calendar_event_class,
            "getStartUnixNs",
            "()J"
        );
    }

    if (calendar_event_get_end_unix_ns_method == nullptr) {
        calendar_event_get_end_unix_ns_method = env->GetMethodID(
            calendar_event_class,
            "getEndUnixNs",
            "()J"
        );
    }

    if (calendar_event_get_all_day_method == nullptr) {
        calendar_event_get_all_day_method = env->GetMethodID(
            calendar_event_class,
            "getAllDay",
            "()Z"
        );
    }

    if (calendar_event_get_canceled_method == nullptr) {
        calendar_event_get_canceled_method = env->GetMethodID(
            calendar_event_class,
            "getCanceled",
            "()Z"
        );
    }

    if (calendar_event_get_time_zone_method == nullptr) {
        calendar_event_get_time_zone_method = env->GetMethodID(
            calendar_event_class,
            "getTimeZone",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_availability_method == nullptr) {
        calendar_event_get_availability_method = env->GetMethodID(
            calendar_event_class,
            "getAvailability",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;"
        );
    }

    if (calendar_event_get_url_method == nullptr) {
        calendar_event_get_url_method = env->GetMethodID(
            calendar_event_class,
            "getUrl",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_organizer_name_method == nullptr) {
        calendar_event_get_organizer_name_method = env->GetMethodID(
            calendar_event_class,
            "getOrganizerName",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_organizer_email_method == nullptr) {
        calendar_event_get_organizer_email_method = env->GetMethodID(
            calendar_event_class,
            "getOrganizerEmail",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_recurring_method == nullptr) {
        calendar_event_get_recurring_method = env->GetMethodID(
            calendar_event_class,
            "getRecurring",
            "()Z"
        );
    }

    if (calendar_event_get_recurrence_master_id_method == nullptr) {
        calendar_event_get_recurrence_master_id_method = env->GetMethodID(
            calendar_event_class,
            "getRecurrenceMasterId",
            "()Ljava/lang/String;"
        );
    }

    if (calendar_event_get_recurrence_id_unix_ns_method == nullptr) {
        calendar_event_get_recurrence_id_unix_ns_method = env->GetMethodID(
            calendar_event_class,
            "getRecurrenceIdUnixNs",
            "()Ljava/lang/Long;"
        );
    }

    if (calendar_event_get_recurrence_rule_method == nullptr) {
        calendar_event_get_recurrence_rule_method = env->GetMethodID(
            calendar_event_class,
            "getRecurrenceRule",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceRule;"
        );
    }

    if (calendar_event_get_attendees_method == nullptr) {
        calendar_event_get_attendees_method = env->GetMethodID(
            calendar_event_class,
            "getAttendees",
            "()Ljava/util/List;"
        );
    }

    if (calendar_event_get_reminders_method == nullptr) {
        calendar_event_get_reminders_method = env->GetMethodID(
            calendar_event_class,
            "getReminders",
            "()Ljava/util/List;"
        );
    }

    if (calendar_query_constructor == nullptr) {
        calendar_query_constructor = env->GetMethodID(
            calendar_query_class,
            "<init>",
            "(Ljava/util/List;JJLjava/lang/Integer;ZZZ)V"
        );
    }

    if (calendar_draft_constructor == nullptr) {
        calendar_draft_constructor = env->GetMethodID(
            calendar_draft_class,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;JJZLjava/lang/String;Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;Ljava/lang/String;Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceRule;Ljava/util/List;Ljava/util/List;)V"
        );
    }

    if (recurrence_rule_constructor == nullptr) {
        recurrence_rule_constructor = env->GetMethodID(
            calendar_recurrence_rule_class,
            "<init>",
            "(Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceFrequency;ILjava/lang/Integer;Ljava/lang/Long;Ljava/util/List;Ljava/util/List;Ljava/util/List;Ljava/util/List;Ljava/util/List;Ljava/util/List;Ljava/util/List;)V"
        );
    }

    if (recurrence_rule_get_frequency_method == nullptr) {
        recurrence_rule_get_frequency_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getFrequency",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceFrequency;"
        );
    }

    if (recurrence_rule_get_interval_method == nullptr) {
        recurrence_rule_get_interval_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getInterval",
            "()I"
        );
    }

    if (recurrence_rule_get_count_method == nullptr) {
        recurrence_rule_get_count_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getCount",
            "()Ljava/lang/Integer;"
        );
    }

    if (recurrence_rule_get_until_unix_ns_method == nullptr) {
        recurrence_rule_get_until_unix_ns_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getUntilUnixNs",
            "()Ljava/lang/Long;"
        );
    }

    if (recurrence_rule_get_by_week_days_method == nullptr) {
        recurrence_rule_get_by_week_days_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getByWeekDays",
            "()Ljava/util/List;"
        );
    }

    if (recurrence_rule_get_by_weekday_ordinals_method == nullptr) {
        recurrence_rule_get_by_weekday_ordinals_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getByWeekdayOrdinals",
            "()Ljava/util/List;"
        );
    }

    if (recurrence_rule_get_by_month_days_method == nullptr) {
        recurrence_rule_get_by_month_days_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getByMonthDays",
            "()Ljava/util/List;"
        );
    }

    if (recurrence_rule_get_by_months_method == nullptr) {
        recurrence_rule_get_by_months_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getByMonths",
            "()Ljava/util/List;"
        );
    }

    if (recurrence_rule_get_by_year_days_method == nullptr) {
        recurrence_rule_get_by_year_days_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getByYearDays",
            "()Ljava/util/List;"
        );
    }

    if (recurrence_rule_get_by_week_numbers_method == nullptr) {
        recurrence_rule_get_by_week_numbers_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getByWeekNumbers",
            "()Ljava/util/List;"
        );
    }

    if (recurrence_rule_get_by_set_positions_method == nullptr) {
        recurrence_rule_get_by_set_positions_method = env->GetMethodID(
            calendar_recurrence_rule_class,
            "getBySetPositions",
            "()Ljava/util/List;"
        );
    }

    if (recurrence_weekday_constructor == nullptr) {
        recurrence_weekday_constructor = env->GetMethodID(
            calendar_recurrence_weekday_class,
            "<init>",
            "(ILjava/lang/Integer;)V"
        );
    }

    if (recurrence_weekday_get_day_method == nullptr) {
        recurrence_weekday_get_day_method = env->GetMethodID(
            calendar_recurrence_weekday_class,
            "getDay",
            "()I"
        );
    }

    if (recurrence_weekday_get_week_number_method == nullptr) {
        recurrence_weekday_get_week_number_method = env->GetMethodID(
            calendar_recurrence_weekday_class,
            "getWeekNumber",
            "()Ljava/lang/Integer;"
        );
    }

    if (attendee_constructor == nullptr) {
        attendee_constructor = env->GetMethodID(
            calendar_attendee_class,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;ZZLdev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;)V"
        );
    }

    if (attendee_get_id_method == nullptr) {
        attendee_get_id_method = env->GetMethodID(
            calendar_attendee_class,
            "getId",
            "()Ljava/lang/String;"
        );
    }

    if (attendee_get_name_method == nullptr) {
        attendee_get_name_method = env->GetMethodID(
            calendar_attendee_class,
            "getName",
            "()Ljava/lang/String;"
        );
    }

    if (attendee_get_email_method == nullptr) {
        attendee_get_email_method = env->GetMethodID(
            calendar_attendee_class,
            "getEmail",
            "()Ljava/lang/String;"
        );
    }

    if (attendee_get_optional_method == nullptr) {
        attendee_get_optional_method = env->GetMethodID(
            calendar_attendee_class,
            "getOptional",
            "()Z"
        );
    }

    if (attendee_get_organizer_method == nullptr) {
        attendee_get_organizer_method = env->GetMethodID(
            calendar_attendee_class,
            "getOrganizer",
            "()Z"
        );
    }

    if (attendee_get_response_status_method == nullptr) {
        attendee_get_response_status_method = env->GetMethodID(
            calendar_attendee_class,
            "getResponseStatus",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (absolute_reminder_constructor == nullptr) {
        absolute_reminder_constructor = env->GetMethodID(
            calendar_absolute_reminder_class,
            "<init>",
            "(J)V"
        );
    }

    if (absolute_reminder_get_absolute_unix_ns_method == nullptr) {
        absolute_reminder_get_absolute_unix_ns_method = env->GetMethodID(
            calendar_absolute_reminder_class,
            "getAbsoluteUnixNs",
            "()J"
        );
    }

    if (relative_reminder_constructor == nullptr) {
        relative_reminder_constructor = env->GetMethodID(
            calendar_relative_reminder_class,
            "<init>",
            "(I)V"
        );
    }

    if (relative_reminder_get_minutes_before_start_method == nullptr) {
        relative_reminder_get_minutes_before_start_method = env->GetMethodID(
            calendar_relative_reminder_class,
            "getMinutesBeforeStart",
            "()I"
        );
    }

    if (reminder_absolute_constructor == nullptr) {
        reminder_absolute_constructor = env->GetMethodID(
            calendar_reminder_absolute_class,
            "<init>",
            "(Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAbsoluteReminder;)V"
        );
    }

    if (reminder_relative_constructor == nullptr) {
        reminder_relative_constructor = env->GetMethodID(
            calendar_reminder_relative_class,
            "<init>",
            "(Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRelativeReminder;)V"
        );
    }

    if (reminder_absolute_get_value_method == nullptr) {
        reminder_absolute_get_value_method = env->GetMethodID(
            calendar_reminder_absolute_class,
            "getValue",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAbsoluteReminder;"
        );
    }

    if (reminder_relative_get_value_method == nullptr) {
        reminder_relative_get_value_method = env->GetMethodID(
            calendar_reminder_relative_class,
            "getValue",
            "()Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRelativeReminder;"
        );
    }

    if (calendar_availability_busy_field == nullptr) {
        calendar_availability_busy_field = env->GetStaticFieldID(
            calendar_availability_class,
            "Busy",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;"
        );
    }

    if (calendar_availability_free_field == nullptr) {
        calendar_availability_free_field = env->GetStaticFieldID(
            calendar_availability_class,
            "Free",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;"
        );
    }

    if (calendar_availability_tentative_field == nullptr) {
        calendar_availability_tentative_field = env->GetStaticFieldID(
            calendar_availability_class,
            "Tentative",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;"
        );
    }

    if (calendar_availability_out_of_office_field == nullptr) {
        calendar_availability_out_of_office_field = env->GetStaticFieldID(
            calendar_availability_class,
            "OutOfOffice",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;"
        );
    }

    if (calendar_availability_unavailable_field == nullptr) {
        calendar_availability_unavailable_field = env->GetStaticFieldID(
            calendar_availability_class,
            "Unavailable",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;"
        );
    }

    if (calendar_availability_unknown_field == nullptr) {
        calendar_availability_unknown_field = env->GetStaticFieldID(
            calendar_availability_class,
            "Unknown",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarAvailability;"
        );
    }

    if (calendar_participant_status_unknown_field == nullptr) {
        calendar_participant_status_unknown_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "Unknown",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_participant_status_pending_field == nullptr) {
        calendar_participant_status_pending_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "Pending",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_participant_status_accepted_field == nullptr) {
        calendar_participant_status_accepted_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "Accepted",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_participant_status_tentative_field == nullptr) {
        calendar_participant_status_tentative_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "Tentative",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_participant_status_declined_field == nullptr) {
        calendar_participant_status_declined_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "Declined",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_participant_status_delegated_field == nullptr) {
        calendar_participant_status_delegated_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "Delegated",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_participant_status_completed_field == nullptr) {
        calendar_participant_status_completed_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "Completed",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_participant_status_in_process_field == nullptr) {
        calendar_participant_status_in_process_field = env->GetStaticFieldID(
            calendar_participant_status_class,
            "InProcess",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarParticipantStatus;"
        );
    }

    if (calendar_recurrence_frequency_daily_field == nullptr) {
        calendar_recurrence_frequency_daily_field = env->GetStaticFieldID(
            calendar_recurrence_frequency_class,
            "Daily",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceFrequency;"
        );
    }

    if (calendar_recurrence_frequency_weekly_field == nullptr) {
        calendar_recurrence_frequency_weekly_field = env->GetStaticFieldID(
            calendar_recurrence_frequency_class,
            "Weekly",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceFrequency;"
        );
    }

    if (calendar_recurrence_frequency_monthly_field == nullptr) {
        calendar_recurrence_frequency_monthly_field = env->GetStaticFieldID(
            calendar_recurrence_frequency_class,
            "Monthly",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceFrequency;"
        );
    }

    if (calendar_recurrence_frequency_yearly_field == nullptr) {
        calendar_recurrence_frequency_yearly_field = env->GetStaticFieldID(
            calendar_recurrence_frequency_class,
            "Yearly",
            "Ldev/destack/runtime/android/module/calendar/RuntimeHostCalendarRecurrenceFrequency;"
        );
    }

    return
        calendar_list_response_get_status_method != nullptr &&
        calendar_list_response_get_calendars_method != nullptr &&
        calendar_event_list_response_get_status_method != nullptr &&
        calendar_event_list_response_get_events_method != nullptr &&
        calendar_event_response_get_status_method != nullptr &&
        calendar_event_response_get_event_method != nullptr &&
        calendar_event_create_response_get_status_method != nullptr &&
        calendar_event_create_response_get_id_method != nullptr &&
        calendar_query_constructor != nullptr &&
        calendar_draft_constructor != nullptr &&
        recurrence_rule_constructor != nullptr &&
        recurrence_weekday_constructor != nullptr &&
        attendee_constructor != nullptr &&
        absolute_reminder_constructor != nullptr &&
        relative_reminder_constructor != nullptr &&
        reminder_absolute_constructor != nullptr &&
        reminder_relative_constructor != nullptr;
}

/// Allocate one mutable array list.
jobject new_array_list(JNIEnv *env, jint capacity) {
    return env->NewObject(array_list_class, array_list_constructor, capacity);
}

/// Append one object to one array list.
bool array_list_add(JNIEnv *env, jobject list, jobject value) {
    return env->CallBooleanMethod(list, array_list_add_method, value) == JNI_TRUE;
}

/// Box one optional `u32` for one Java constructor argument.
jobject new_optional_integer(JNIEnv *env, HostOptionalU32 value) {
    if (!value.has_value) {
        return nullptr;
    }

    return env->CallStaticObjectMethod(
        integer_class,
        integer_value_of_method,
        static_cast<jint>(value.value)
    );
}

/// Box one optional `u64` for one Java constructor argument.
jobject new_optional_long(JNIEnv *env, HostOptionalU64 value) {
    if (!value.has_value) {
        return nullptr;
    }

    return env->CallStaticObjectMethod(
        long_class,
        long_value_of_method,
        static_cast<jlong>(value.value)
    );
}

/// Box one optional `i8` for one Java constructor argument.
jobject new_optional_i8(JNIEnv *env, HostOptionalI8 value) {
    if (!value.has_value) {
        return nullptr;
    }

    return env->CallStaticObjectMethod(
        integer_class,
        integer_value_of_method,
        static_cast<jint>(value.value)
    );
}

/// Resolve one availability enum object for one bridge input payload.
jobject calendar_availability_object(JNIEnv *env, HostCalendarAvailability availability) {
    jfieldID field = calendar_availability_unknown_field;

    switch (availability) {
        case HOST_CALENDAR_AVAILABILITY_BUSY:
            field = calendar_availability_busy_field;
            break;
        case HOST_CALENDAR_AVAILABILITY_FREE:
            field = calendar_availability_free_field;
            break;
        case HOST_CALENDAR_AVAILABILITY_TENTATIVE:
            field = calendar_availability_tentative_field;
            break;
        case HOST_CALENDAR_AVAILABILITY_OUT_OF_OFFICE:
            field = calendar_availability_out_of_office_field;
            break;
        case HOST_CALENDAR_AVAILABILITY_UNAVAILABLE:
            field = calendar_availability_unavailable_field;
            break;
        case HOST_CALENDAR_AVAILABILITY_UNKNOWN:
        default:
            field = calendar_availability_unknown_field;
            break;
    }

    return env->GetStaticObjectField(calendar_availability_class, field);
}

/// Resolve one participant-status enum object for one bridge input payload.
jobject calendar_participant_status_object(
    JNIEnv *env,
    HostCalendarParticipantStatus status
) {
    jfieldID field = calendar_participant_status_unknown_field;

    switch (status) {
        case HOST_CALENDAR_PARTICIPANT_STATUS_PENDING:
            field = calendar_participant_status_pending_field;
            break;
        case HOST_CALENDAR_PARTICIPANT_STATUS_ACCEPTED:
            field = calendar_participant_status_accepted_field;
            break;
        case HOST_CALENDAR_PARTICIPANT_STATUS_TENTATIVE:
            field = calendar_participant_status_tentative_field;
            break;
        case HOST_CALENDAR_PARTICIPANT_STATUS_DECLINED:
            field = calendar_participant_status_declined_field;
            break;
        case HOST_CALENDAR_PARTICIPANT_STATUS_DELEGATED:
            field = calendar_participant_status_delegated_field;
            break;
        case HOST_CALENDAR_PARTICIPANT_STATUS_COMPLETED:
            field = calendar_participant_status_completed_field;
            break;
        case HOST_CALENDAR_PARTICIPANT_STATUS_IN_PROCESS:
            field = calendar_participant_status_in_process_field;
            break;
        case HOST_CALENDAR_PARTICIPANT_STATUS_UNKNOWN:
        default:
            field = calendar_participant_status_unknown_field;
            break;
    }

    return env->GetStaticObjectField(calendar_participant_status_class, field);
}

/// Resolve one recurrence-frequency enum object for one bridge input payload.
jobject calendar_recurrence_frequency_object(
    JNIEnv *env,
    HostCalendarRecurrenceFrequency frequency
) {
    jfieldID field = calendar_recurrence_frequency_daily_field;

    switch (frequency) {
        case HOST_CALENDAR_RECURRENCE_FREQUENCY_WEEKLY:
            field = calendar_recurrence_frequency_weekly_field;
            break;
        case HOST_CALENDAR_RECURRENCE_FREQUENCY_MONTHLY:
            field = calendar_recurrence_frequency_monthly_field;
            break;
        case HOST_CALENDAR_RECURRENCE_FREQUENCY_YEARLY:
            field = calendar_recurrence_frequency_yearly_field;
            break;
        case HOST_CALENDAR_RECURRENCE_FREQUENCY_DAILY:
        default:
            field = calendar_recurrence_frequency_daily_field;
            break;
    }

    return env->GetStaticObjectField(calendar_recurrence_frequency_class, field);
}

/// Build one Java `List<String>` from one native string slice.
jobject new_java_string_list(JNIEnv *env, NativeStringSlice values) {
    jobject list = new_array_list(env, static_cast<jint>(values.len));
    if (list == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jstring value = new_java_string(env, values.data[index]);
        if (value == nullptr) {
            env->DeleteLocalRef(list);
            return nullptr;
        }

        bool did_add = array_list_add(env, list, value);
        env->DeleteLocalRef(value);
        if (!did_add || env->ExceptionCheck()) {
            env->DeleteLocalRef(list);
            return nullptr;
        }
    }

    return list;
}

/// Build one Java `List<Integer>` from one `u8` slice.
jobject new_u8_list(JNIEnv *env, NativeU8Slice values) {
    jobject list = new_array_list(env, static_cast<jint>(values.len));
    if (list == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jobject value = env->CallStaticObjectMethod(
            integer_class,
            integer_value_of_method,
            static_cast<jint>(values.data[index])
        );
        if (value == nullptr) {
            env->DeleteLocalRef(list);
            return nullptr;
        }

        bool did_add = array_list_add(env, list, value);
        env->DeleteLocalRef(value);
        if (!did_add || env->ExceptionCheck()) {
            env->DeleteLocalRef(list);
            return nullptr;
        }
    }

    return list;
}

/// Build one Java `List<Integer>` from one `i8` slice.
jobject new_i8_list(JNIEnv *env, NativeI8Slice values) {
    jobject list = new_array_list(env, static_cast<jint>(values.len));
    if (list == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jobject value = env->CallStaticObjectMethod(
            integer_class,
            integer_value_of_method,
            static_cast<jint>(values.data[index])
        );
        if (value == nullptr) {
            env->DeleteLocalRef(list);
            return nullptr;
        }

        bool did_add = array_list_add(env, list, value);
        env->DeleteLocalRef(value);
        if (!did_add || env->ExceptionCheck()) {
            env->DeleteLocalRef(list);
            return nullptr;
        }
    }

    return list;
}

/// Build one Java `List<Integer>` from one `i16` slice.
jobject new_i16_list(JNIEnv *env, NativeI16Slice values) {
    jobject list = new_array_list(env, static_cast<jint>(values.len));
    if (list == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jobject value = env->CallStaticObjectMethod(
            integer_class,
            integer_value_of_method,
            static_cast<jint>(values.data[index])
        );
        if (value == nullptr) {
            env->DeleteLocalRef(list);
            return nullptr;
        }

        bool did_add = array_list_add(env, list, value);
        env->DeleteLocalRef(value);
        if (!did_add || env->ExceptionCheck()) {
            env->DeleteLocalRef(list);
            return nullptr;
        }
    }

    return list;
}

/// Build one Java recurrence-weekday object.
jobject new_calendar_recurrence_weekday(JNIEnv *env, HostCalendarRecurrenceWeekday weekday) {
    jobject week_number = new_optional_i8(env, weekday.week_number);
    jobject value = env->NewObject(
        calendar_recurrence_weekday_class,
        recurrence_weekday_constructor,
        static_cast<jint>(weekday.day),
        week_number
    );

    if (week_number != nullptr) {
        env->DeleteLocalRef(week_number);
    }

    return value;
}

/// Build one Java `List<RuntimeHostCalendarRecurrenceWeekday>`.
jobject new_calendar_recurrence_weekday_list(
    JNIEnv *env,
    HostCalendarRecurrenceWeekdaySlice values
) {
    jobject list = new_array_list(env, static_cast<jint>(values.len));
    if (list == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < values.len; index += 1) {
        jobject value = new_calendar_recurrence_weekday(env, values.data[index]);
        if (value == nullptr) {
            env->DeleteLocalRef(list);
            return nullptr;
        }

        bool did_add = array_list_add(env, list, value);
        env->DeleteLocalRef(value);
        if (!did_add || env->ExceptionCheck()) {
            env->DeleteLocalRef(list);
            return nullptr;
        }
    }

    return list;
}

/// Build one Java recurrence-rule object.
jobject new_calendar_recurrence_rule(JNIEnv *env, HostCalendarRecurrenceRule rule) {
    jobject frequency = calendar_recurrence_frequency_object(env, rule.frequency);
    jobject count = new_optional_integer(env, rule.count);
    jobject until_unix_ns = new_optional_long(env, rule.until_unix_ns);
    jobject by_week_days = new_u8_list(env, rule.by_week_days);
    jobject by_weekday_ordinals = new_calendar_recurrence_weekday_list(
        env,
        rule.by_weekday_ordinals
    );
    jobject by_month_days = new_i8_list(env, rule.by_month_days);
    jobject by_months = new_u8_list(env, rule.by_months);
    jobject by_year_days = new_i16_list(env, rule.by_year_days);
    jobject by_week_numbers = new_i8_list(env, rule.by_week_numbers);
    jobject by_set_positions = new_i16_list(env, rule.by_set_positions);

    if (frequency == nullptr || by_week_days == nullptr || by_weekday_ordinals == nullptr ||
        by_month_days == nullptr || by_months == nullptr || by_year_days == nullptr ||
        by_week_numbers == nullptr || by_set_positions == nullptr) {
        if (frequency != nullptr) env->DeleteLocalRef(frequency);
        if (count != nullptr) env->DeleteLocalRef(count);
        if (until_unix_ns != nullptr) env->DeleteLocalRef(until_unix_ns);
        if (by_week_days != nullptr) env->DeleteLocalRef(by_week_days);
        if (by_weekday_ordinals != nullptr) env->DeleteLocalRef(by_weekday_ordinals);
        if (by_month_days != nullptr) env->DeleteLocalRef(by_month_days);
        if (by_months != nullptr) env->DeleteLocalRef(by_months);
        if (by_year_days != nullptr) env->DeleteLocalRef(by_year_days);
        if (by_week_numbers != nullptr) env->DeleteLocalRef(by_week_numbers);
        if (by_set_positions != nullptr) env->DeleteLocalRef(by_set_positions);
        return nullptr;
    }

    jobject value = env->NewObject(
        calendar_recurrence_rule_class,
        recurrence_rule_constructor,
        frequency,
        static_cast<jint>(rule.interval),
        count,
        until_unix_ns,
        by_week_days,
        by_weekday_ordinals,
        by_month_days,
        by_months,
        by_year_days,
        by_week_numbers,
        by_set_positions
    );

    env->DeleteLocalRef(frequency);
    if (count != nullptr) env->DeleteLocalRef(count);
    if (until_unix_ns != nullptr) env->DeleteLocalRef(until_unix_ns);
    env->DeleteLocalRef(by_week_days);
    env->DeleteLocalRef(by_weekday_ordinals);
    env->DeleteLocalRef(by_month_days);
    env->DeleteLocalRef(by_months);
    env->DeleteLocalRef(by_year_days);
    env->DeleteLocalRef(by_week_numbers);
    env->DeleteLocalRef(by_set_positions);

    return value;
}

/// Build one Java attendee object.
jobject new_calendar_attendee(JNIEnv *env, HostCalendarAttendee attendee) {
    jstring identifier = attendee.id.has_value ? new_java_string(env, attendee.id.value) : nullptr;
    jstring name = attendee.name.has_value ? new_java_string(env, attendee.name.value) : nullptr;
    jstring email = attendee.email.has_value ? new_java_string(env, attendee.email.value) : nullptr;
    jobject response_status = calendar_participant_status_object(env, attendee.response_status);

    jobject value = env->NewObject(
        calendar_attendee_class,
        attendee_constructor,
        identifier,
        name,
        email,
        attendee.optional ? JNI_TRUE : JNI_FALSE,
        attendee.organizer ? JNI_TRUE : JNI_FALSE,
        response_status
    );

    if (identifier != nullptr) env->DeleteLocalRef(identifier);
    if (name != nullptr) env->DeleteLocalRef(name);
    if (email != nullptr) env->DeleteLocalRef(email);
    if (response_status != nullptr) env->DeleteLocalRef(response_status);

    return value;
}

/// Build one Java attendee list.
jobject new_calendar_attendee_list(JNIEnv *env, HostCalendarAttendeeSlice attendees) {
    jobject list = new_array_list(env, static_cast<jint>(attendees.len));
    if (list == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < attendees.len; index += 1) {
        jobject value = new_calendar_attendee(env, attendees.data[index]);
        if (value == nullptr) {
            env->DeleteLocalRef(list);
            return nullptr;
        }

        bool did_add = array_list_add(env, list, value);
        env->DeleteLocalRef(value);
        if (!did_add || env->ExceptionCheck()) {
            env->DeleteLocalRef(list);
            return nullptr;
        }
    }

    return list;
}

/// Build one Java reminder object.
jobject new_calendar_reminder(JNIEnv *env, HostCalendarReminder reminder) {
    if (reminder.kind == HOST_CALENDAR_REMINDER_KIND_ABSOLUTE) {
        jobject payload = env->NewObject(
            calendar_absolute_reminder_class,
            absolute_reminder_constructor,
            static_cast<jlong>(reminder.absolute_unix_ns)
        );
        if (payload == nullptr) {
            return nullptr;
        }

        jobject value = env->NewObject(
            calendar_reminder_absolute_class,
            reminder_absolute_constructor,
            payload
        );
        env->DeleteLocalRef(payload);
        return value;
    }

    jobject payload = env->NewObject(
        calendar_relative_reminder_class,
        relative_reminder_constructor,
        static_cast<jint>(reminder.minutes_before_start)
    );
    if (payload == nullptr) {
        return nullptr;
    }

    jobject value = env->NewObject(
        calendar_reminder_relative_class,
        reminder_relative_constructor,
        payload
    );
    env->DeleteLocalRef(payload);

    return value;
}

/// Build one Java reminder list.
jobject new_calendar_reminder_list(JNIEnv *env, HostCalendarReminderSlice reminders) {
    jobject list = new_array_list(env, static_cast<jint>(reminders.len));
    if (list == nullptr) {
        return nullptr;
    }

    for (uint32_t index = 0; index < reminders.len; index += 1) {
        jobject value = new_calendar_reminder(env, reminders.data[index]);
        if (value == nullptr) {
            env->DeleteLocalRef(list);
            return nullptr;
        }

        bool did_add = array_list_add(env, list, value);
        env->DeleteLocalRef(value);
        if (!did_add || env->ExceptionCheck()) {
            env->DeleteLocalRef(list);
            return nullptr;
        }
    }

    return list;
}

/// Build one Java query object from one native calendar query payload.
jobject new_calendar_query(JNIEnv *env, HostCalendarQuery query) {
    jobject calendar_ids = new_java_string_list(env, query.calendar_ids);
    jobject limit = new_optional_integer(env, query.limit);
    if (calendar_ids == nullptr) {
        if (limit != nullptr) {
            env->DeleteLocalRef(limit);
        }
        return nullptr;
    }

    jobject value = env->NewObject(
        calendar_query_class,
        calendar_query_constructor,
        calendar_ids,
        static_cast<jlong>(query.start_unix_ns),
        static_cast<jlong>(query.end_unix_ns),
        limit,
        query.include_canceled ? JNI_TRUE : JNI_FALSE,
        query.include_declined ? JNI_TRUE : JNI_FALSE,
        query.include_recurrence_instances ? JNI_TRUE : JNI_FALSE
    );

    env->DeleteLocalRef(calendar_ids);
    if (limit != nullptr) {
        env->DeleteLocalRef(limit);
    }

    return value;
}

/// Build one Java draft object from one native calendar draft payload.
jobject new_calendar_draft(JNIEnv *env, HostCalendarEventDraft draft) {
    jstring calendar_id = new_java_string(env, draft.calendar_id);
    jstring title = new_java_string(env, draft.title);
    jstring notes = draft.notes.has_value ? new_java_string(env, draft.notes.value) : nullptr;
    jstring location = draft.location.has_value ? new_java_string(env, draft.location.value) : nullptr;
    jstring time_zone = draft.time_zone.has_value ? new_java_string(env, draft.time_zone.value) : nullptr;
    jobject availability = calendar_availability_object(env, draft.availability);
    jstring url = draft.url.has_value ? new_java_string(env, draft.url.value) : nullptr;
    jobject recurrence_rule = draft.has_recurrence_rule
        ? new_calendar_recurrence_rule(env, draft.recurrence_rule)
        : nullptr;
    jobject attendees = draft.has_attendees
        ? new_calendar_attendee_list(env, draft.attendees)
        : nullptr;
    jobject reminders = draft.has_reminders
        ? new_calendar_reminder_list(env, draft.reminders)
        : nullptr;

    if (calendar_id == nullptr || title == nullptr || availability == nullptr) {
        if (calendar_id != nullptr) env->DeleteLocalRef(calendar_id);
        if (title != nullptr) env->DeleteLocalRef(title);
        if (notes != nullptr) env->DeleteLocalRef(notes);
        if (location != nullptr) env->DeleteLocalRef(location);
        if (time_zone != nullptr) env->DeleteLocalRef(time_zone);
        if (availability != nullptr) env->DeleteLocalRef(availability);
        if (url != nullptr) env->DeleteLocalRef(url);
        if (recurrence_rule != nullptr) env->DeleteLocalRef(recurrence_rule);
        if (attendees != nullptr) env->DeleteLocalRef(attendees);
        if (reminders != nullptr) env->DeleteLocalRef(reminders);
        return nullptr;
    }

    jobject value = env->NewObject(
        calendar_draft_class,
        calendar_draft_constructor,
        calendar_id,
        title,
        notes,
        location,
        static_cast<jlong>(draft.start_unix_ns),
        static_cast<jlong>(draft.end_unix_ns),
        draft.all_day ? JNI_TRUE : JNI_FALSE,
        time_zone,
        availability,
        url,
        recurrence_rule,
        attendees,
        reminders
    );

    env->DeleteLocalRef(calendar_id);
    env->DeleteLocalRef(title);
    if (notes != nullptr) env->DeleteLocalRef(notes);
    if (location != nullptr) env->DeleteLocalRef(location);
    if (time_zone != nullptr) env->DeleteLocalRef(time_zone);
    env->DeleteLocalRef(availability);
    if (url != nullptr) env->DeleteLocalRef(url);
    if (recurrence_rule != nullptr) env->DeleteLocalRef(recurrence_rule);
    if (attendees != nullptr) env->DeleteLocalRef(attendees);
    if (reminders != nullptr) env->DeleteLocalRef(reminders);

    return value;
}

/// Decode one boxed integer into one optional `u32`.
HostOptionalU32 decode_optional_u32(JNIEnv *env, jobject value) {
    if (value == nullptr) {
        return HostOptionalU32 {
            .has_value = false,
            .value = 0,
        };
    }

    jint resolved = env->CallIntMethod(value, integer_int_value_method);
    return HostOptionalU32 {
        .has_value = true,
        .value = static_cast<uint32_t>(resolved),
    };
}

/// Decode one boxed integer into one optional `i8`.
HostOptionalI8 decode_optional_i8(JNIEnv *env, jobject value) {
    if (value == nullptr) {
        return HostOptionalI8 {
            .has_value = false,
            .value = 0,
        };
    }

    jint resolved = env->CallIntMethod(value, integer_int_value_method);
    return HostOptionalI8 {
        .has_value = true,
        .value = static_cast<int8_t>(resolved),
    };
}

/// Decode one boxed long into one optional `u64`.
HostOptionalU64 decode_optional_u64(JNIEnv *env, jobject value) {
    if (value == nullptr) {
        return HostOptionalU64 {
            .has_value = false,
            .value = 0,
        };
    }

    jlong resolved = env->CallLongMethod(value, long_long_value_method);
    return HostOptionalU64 {
        .has_value = true,
        .value = static_cast<uint64_t>(resolved),
    };
}

/// Decode one Java enum object into one repr(C) integer-tag enum.
template <typename T>
T decode_enum_ordinal(JNIEnv *env, jobject value) {
    jint ordinal = env->CallIntMethod(value, enum_ordinal_method);
    return static_cast<T>(ordinal + 1);
}

/// Decode one `List<Integer>` into one `u8` slice backed by stable storage.
NativeU8Slice decode_u8_list(JNIEnv *env, jobject list) {
    if (list == nullptr) {
        return NativeU8Slice {
            .data = nullptr,
            .len = 0,
        };
    }

    jint size = env->CallIntMethod(list, list_size_method);
    result_u8_storage.emplace_back();
    auto &storage = result_u8_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        jint resolved = env->CallIntMethod(value, integer_int_value_method);
        storage.push_back(static_cast<uint8_t>(resolved));
        env->DeleteLocalRef(value);
    }

    return NativeU8Slice {
        .data = storage.data(),
        .len = static_cast<uint32_t>(storage.size()),
    };
}

/// Decode one `List<Integer>` into one `i8` slice backed by stable storage.
NativeI8Slice decode_i8_list(JNIEnv *env, jobject list) {
    if (list == nullptr) {
        return NativeI8Slice {
            .data = nullptr,
            .len = 0,
        };
    }

    jint size = env->CallIntMethod(list, list_size_method);
    result_i8_storage.emplace_back();
    auto &storage = result_i8_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        jint resolved = env->CallIntMethod(value, integer_int_value_method);
        storage.push_back(static_cast<int8_t>(resolved));
        env->DeleteLocalRef(value);
    }

    return NativeI8Slice {
        .data = storage.data(),
        .len = static_cast<uint32_t>(storage.size()),
    };
}

/// Decode one `List<Integer>` into one `i16` slice backed by stable storage.
NativeI16Slice decode_i16_list(JNIEnv *env, jobject list) {
    if (list == nullptr) {
        return NativeI16Slice {
            .data = nullptr,
            .len = 0,
        };
    }

    jint size = env->CallIntMethod(list, list_size_method);
    result_i16_storage.emplace_back();
    auto &storage = result_i16_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        jint resolved = env->CallIntMethod(value, integer_int_value_method);
        storage.push_back(static_cast<int16_t>(resolved));
        env->DeleteLocalRef(value);
    }

    return NativeI16Slice {
        .data = storage.data(),
        .len = static_cast<uint32_t>(storage.size()),
    };
}

/// Decode one Java recurrence-weekday object.
HostCalendarRecurrenceWeekday decode_calendar_recurrence_weekday(JNIEnv *env, jobject weekday) {
    jobject week_number = env->CallObjectMethod(
        weekday,
        recurrence_weekday_get_week_number_method
    );
    HostCalendarRecurrenceWeekday value = {
        .day = static_cast<uint8_t>(
            env->CallIntMethod(weekday, recurrence_weekday_get_day_method)
        ),
        .week_number = decode_optional_i8(env, week_number),
    };

    if (week_number != nullptr) {
        env->DeleteLocalRef(week_number);
    }

    return value;
}

/// Decode one `List<RuntimeHostCalendarRecurrenceWeekday>`.
HostCalendarRecurrenceWeekdaySlice decode_calendar_recurrence_weekday_list(
    JNIEnv *env,
    jobject list
) {
    if (list == nullptr) {
        return HostCalendarRecurrenceWeekdaySlice {
            .data = nullptr,
            .len = 0,
        };
    }

    jint size = env->CallIntMethod(list, list_size_method);
    result_weekday_storage.emplace_back();
    auto &storage = result_weekday_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        storage.push_back(decode_calendar_recurrence_weekday(env, value));
        env->DeleteLocalRef(value);
    }

    return HostCalendarRecurrenceWeekdaySlice {
        .data = storage.data(),
        .len = static_cast<uint32_t>(storage.size()),
    };
}

/// Decode one Java recurrence-rule object.
HostCalendarRecurrenceRule decode_calendar_recurrence_rule(JNIEnv *env, jobject rule) {
    jobject frequency = env->CallObjectMethod(rule, recurrence_rule_get_frequency_method);
    jobject count = env->CallObjectMethod(rule, recurrence_rule_get_count_method);
    jobject until_unix_ns = env->CallObjectMethod(rule, recurrence_rule_get_until_unix_ns_method);
    jobject by_week_days = env->CallObjectMethod(rule, recurrence_rule_get_by_week_days_method);
    jobject by_weekday_ordinals = env->CallObjectMethod(
        rule,
        recurrence_rule_get_by_weekday_ordinals_method
    );
    jobject by_month_days = env->CallObjectMethod(rule, recurrence_rule_get_by_month_days_method);
    jobject by_months = env->CallObjectMethod(rule, recurrence_rule_get_by_months_method);
    jobject by_year_days = env->CallObjectMethod(rule, recurrence_rule_get_by_year_days_method);
    jobject by_week_numbers = env->CallObjectMethod(
        rule,
        recurrence_rule_get_by_week_numbers_method
    );
    jobject by_set_positions = env->CallObjectMethod(
        rule,
        recurrence_rule_get_by_set_positions_method
    );

    HostCalendarRecurrenceRule value = {
        .frequency = decode_enum_ordinal<HostCalendarRecurrenceFrequency>(env, frequency),
        .interval = static_cast<uint32_t>(
            env->CallIntMethod(rule, recurrence_rule_get_interval_method)
        ),
        .count = decode_optional_u32(env, count),
        .until_unix_ns = decode_optional_u64(env, until_unix_ns),
        .by_week_days = decode_u8_list(env, by_week_days),
        .by_weekday_ordinals = decode_calendar_recurrence_weekday_list(
            env,
            by_weekday_ordinals
        ),
        .by_month_days = decode_i8_list(env, by_month_days),
        .by_months = decode_u8_list(env, by_months),
        .by_year_days = decode_i16_list(env, by_year_days),
        .by_week_numbers = decode_i8_list(env, by_week_numbers),
        .by_set_positions = decode_i16_list(env, by_set_positions),
    };

    env->DeleteLocalRef(frequency);
    if (count != nullptr) env->DeleteLocalRef(count);
    if (until_unix_ns != nullptr) env->DeleteLocalRef(until_unix_ns);
    if (by_week_days != nullptr) env->DeleteLocalRef(by_week_days);
    if (by_weekday_ordinals != nullptr) env->DeleteLocalRef(by_weekday_ordinals);
    if (by_month_days != nullptr) env->DeleteLocalRef(by_month_days);
    if (by_months != nullptr) env->DeleteLocalRef(by_months);
    if (by_year_days != nullptr) env->DeleteLocalRef(by_year_days);
    if (by_week_numbers != nullptr) env->DeleteLocalRef(by_week_numbers);
    if (by_set_positions != nullptr) env->DeleteLocalRef(by_set_positions);

    return value;
}

/// Decode one Java attendee object.
HostCalendarAttendee decode_calendar_attendee(JNIEnv *env, jobject attendee) {
    jstring identifier = reinterpret_cast<jstring>(
        env->CallObjectMethod(attendee, attendee_get_id_method)
    );
    jstring name = reinterpret_cast<jstring>(
        env->CallObjectMethod(attendee, attendee_get_name_method)
    );
    jstring email = reinterpret_cast<jstring>(
        env->CallObjectMethod(attendee, attendee_get_email_method)
    );
    jobject response_status = env->CallObjectMethod(
        attendee,
        attendee_get_response_status_method
    );

    HostCalendarAttendee value = {
        .id = optional_string_ref_from_java(env, identifier, &result_string_storage),
        .name = optional_string_ref_from_java(env, name, &result_string_storage),
        .email = optional_string_ref_from_java(env, email, &result_string_storage),
        .optional = env->CallBooleanMethod(attendee, attendee_get_optional_method) == JNI_TRUE,
        .organizer = env->CallBooleanMethod(attendee, attendee_get_organizer_method) == JNI_TRUE,
        .response_status = decode_enum_ordinal<HostCalendarParticipantStatus>(
            env,
            response_status
        ),
    };

    if (identifier != nullptr) env->DeleteLocalRef(identifier);
    if (name != nullptr) env->DeleteLocalRef(name);
    if (email != nullptr) env->DeleteLocalRef(email);
    if (response_status != nullptr) env->DeleteLocalRef(response_status);

    return value;
}

/// Decode one attendee list into one stable native slice.
HostCalendarAttendeeSlice decode_calendar_attendee_list(JNIEnv *env, jobject list) {
    if (list == nullptr) {
        return HostCalendarAttendeeSlice {
            .data = nullptr,
            .len = 0,
        };
    }

    jint size = env->CallIntMethod(list, list_size_method);
    result_attendee_storage.emplace_back();
    auto &storage = result_attendee_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        storage.push_back(decode_calendar_attendee(env, value));
        env->DeleteLocalRef(value);
    }

    return HostCalendarAttendeeSlice {
        .data = storage.data(),
        .len = static_cast<uint32_t>(storage.size()),
    };
}

/// Decode one Java reminder object.
HostCalendarReminder decode_calendar_reminder(JNIEnv *env, jobject reminder) {
    if (env->IsInstanceOf(reminder, calendar_reminder_absolute_class)) {
        jobject value = env->CallObjectMethod(reminder, reminder_absolute_get_value_method);
        jlong absolute_unix_ns = env->CallLongMethod(
            value,
            absolute_reminder_get_absolute_unix_ns_method
        );
        env->DeleteLocalRef(value);

        return HostCalendarReminder {
            .kind = HOST_CALENDAR_REMINDER_KIND_ABSOLUTE,
            .absolute_unix_ns = static_cast<uint64_t>(absolute_unix_ns),
            .minutes_before_start = 0,
        };
    }

    jobject value = env->CallObjectMethod(reminder, reminder_relative_get_value_method);
    jint minutes_before_start = env->CallIntMethod(
        value,
        relative_reminder_get_minutes_before_start_method
    );
    env->DeleteLocalRef(value);

    return HostCalendarReminder {
        .kind = HOST_CALENDAR_REMINDER_KIND_RELATIVE,
        .absolute_unix_ns = 0,
        .minutes_before_start = static_cast<int32_t>(minutes_before_start),
    };
}

/// Decode one reminder list into one stable native slice.
HostCalendarReminderSlice decode_calendar_reminder_list(JNIEnv *env, jobject list) {
    if (list == nullptr) {
        return HostCalendarReminderSlice {
            .data = nullptr,
            .len = 0,
        };
    }

    jint size = env->CallIntMethod(list, list_size_method);
    result_reminder_storage.emplace_back();
    auto &storage = result_reminder_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(list, list_get_method, index);
        storage.push_back(decode_calendar_reminder(env, value));
        env->DeleteLocalRef(value);
    }

    return HostCalendarReminderSlice {
        .data = storage.data(),
        .len = static_cast<uint32_t>(storage.size()),
    };
}

/// Decode one Java calendar descriptor object.
HostCalendarDescriptor decode_calendar_descriptor(JNIEnv *env, jobject descriptor) {
    jstring identifier = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, calendar_descriptor_get_id_method)
    );
    jstring title = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, calendar_descriptor_get_title_method)
    );
    jstring source = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, calendar_descriptor_get_source_method)
    );
    jstring owner = reinterpret_cast<jstring>(
        env->CallObjectMethod(descriptor, calendar_descriptor_get_owner_method)
    );
    jobject access = env->CallObjectMethod(descriptor, calendar_descriptor_get_access_method);

    HostCalendarDescriptor value = {
        .id = string_ref_from_java(env, identifier, &result_string_storage),
        .title = string_ref_from_java(env, title, &result_string_storage),
        .source = string_ref_from_java(env, source, &result_string_storage),
        .owner = optional_string_ref_from_java(env, owner, &result_string_storage),
        .color_argb = static_cast<uint32_t>(
            env->CallIntMethod(descriptor, calendar_descriptor_get_color_argb_method)
        ),
        .primary = env->CallBooleanMethod(descriptor, calendar_descriptor_get_primary_method) == JNI_TRUE,
        .access = decode_enum_ordinal<HostCalendarAccess>(env, access),
    };

    env->DeleteLocalRef(identifier);
    env->DeleteLocalRef(title);
    env->DeleteLocalRef(source);
    if (owner != nullptr) env->DeleteLocalRef(owner);
    env->DeleteLocalRef(access);

    return value;
}

/// Decode one Java calendar event object.
HostCalendarEvent decode_calendar_event(JNIEnv *env, jobject event) {
    jstring identifier = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_id_method)
    );
    jstring calendar_id = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_calendar_id_method)
    );
    jstring title = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_title_method)
    );
    jstring notes = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_notes_method)
    );
    jstring location = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_location_method)
    );
    jstring time_zone = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_time_zone_method)
    );
    jobject availability = env->CallObjectMethod(event, calendar_event_get_availability_method);
    jstring url = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_url_method)
    );
    jstring organizer_name = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_organizer_name_method)
    );
    jstring organizer_email = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_organizer_email_method)
    );
    jstring recurrence_master_id = reinterpret_cast<jstring>(
        env->CallObjectMethod(event, calendar_event_get_recurrence_master_id_method)
    );
    jobject recurrence_id_unix_ns = env->CallObjectMethod(
        event,
        calendar_event_get_recurrence_id_unix_ns_method
    );
    jobject recurrence_rule = env->CallObjectMethod(
        event,
        calendar_event_get_recurrence_rule_method
    );
    jobject attendees = env->CallObjectMethod(event, calendar_event_get_attendees_method);
    jobject reminders = env->CallObjectMethod(event, calendar_event_get_reminders_method);

    HostCalendarEvent value = {
        .id = string_ref_from_java(env, identifier, &result_string_storage),
        .calendar_id = string_ref_from_java(env, calendar_id, &result_string_storage),
        .title = string_ref_from_java(env, title, &result_string_storage),
        .notes = optional_string_ref_from_java(env, notes, &result_string_storage),
        .location = optional_string_ref_from_java(env, location, &result_string_storage),
        .start_unix_ns = static_cast<uint64_t>(
            env->CallLongMethod(event, calendar_event_get_start_unix_ns_method)
        ),
        .end_unix_ns = static_cast<uint64_t>(
            env->CallLongMethod(event, calendar_event_get_end_unix_ns_method)
        ),
        .all_day = env->CallBooleanMethod(event, calendar_event_get_all_day_method) == JNI_TRUE,
        .canceled = env->CallBooleanMethod(event, calendar_event_get_canceled_method) == JNI_TRUE,
        .time_zone = optional_string_ref_from_java(env, time_zone, &result_string_storage),
        .availability = decode_enum_ordinal<HostCalendarAvailability>(env, availability),
        .url = optional_string_ref_from_java(env, url, &result_string_storage),
        .organizer_name = optional_string_ref_from_java(
            env,
            organizer_name,
            &result_string_storage
        ),
        .organizer_email = optional_string_ref_from_java(
            env,
            organizer_email,
            &result_string_storage
        ),
        .recurring = env->CallBooleanMethod(event, calendar_event_get_recurring_method) == JNI_TRUE,
        .recurrence_master_id = optional_string_ref_from_java(
            env,
            recurrence_master_id,
            &result_string_storage
        ),
        .recurrence_id_unix_ns = decode_optional_u64(env, recurrence_id_unix_ns),
        .has_recurrence_rule = recurrence_rule != nullptr,
        .recurrence_rule = recurrence_rule != nullptr
            ? decode_calendar_recurrence_rule(env, recurrence_rule)
            : HostCalendarRecurrenceRule {
                .frequency = HOST_CALENDAR_RECURRENCE_FREQUENCY_DAILY,
                .interval = 1,
                .count = {.has_value = false, .value = 0},
                .until_unix_ns = {.has_value = false, .value = 0},
                .by_week_days = {.data = nullptr, .len = 0},
                .by_weekday_ordinals = {.data = nullptr, .len = 0},
                .by_month_days = {.data = nullptr, .len = 0},
                .by_months = {.data = nullptr, .len = 0},
                .by_year_days = {.data = nullptr, .len = 0},
                .by_week_numbers = {.data = nullptr, .len = 0},
                .by_set_positions = {.data = nullptr, .len = 0},
            },
        .has_attendees = attendees != nullptr,
        .attendees = decode_calendar_attendee_list(env, attendees),
        .has_reminders = reminders != nullptr,
        .reminders = decode_calendar_reminder_list(env, reminders),
    };

    env->DeleteLocalRef(identifier);
    env->DeleteLocalRef(calendar_id);
    env->DeleteLocalRef(title);
    if (notes != nullptr) env->DeleteLocalRef(notes);
    if (location != nullptr) env->DeleteLocalRef(location);
    if (time_zone != nullptr) env->DeleteLocalRef(time_zone);
    env->DeleteLocalRef(availability);
    if (url != nullptr) env->DeleteLocalRef(url);
    if (organizer_name != nullptr) env->DeleteLocalRef(organizer_name);
    if (organizer_email != nullptr) env->DeleteLocalRef(organizer_email);
    if (recurrence_master_id != nullptr) env->DeleteLocalRef(recurrence_master_id);
    if (recurrence_id_unix_ns != nullptr) env->DeleteLocalRef(recurrence_id_unix_ns);
    if (recurrence_rule != nullptr) env->DeleteLocalRef(recurrence_rule);
    if (attendees != nullptr) env->DeleteLocalRef(attendees);
    if (reminders != nullptr) env->DeleteLocalRef(reminders);

    return value;
}

/// Decode one calendar-list response.
uint32_t decode_calendar_list_response(
    JNIEnv *env,
    jobject response,
    HostCalendarDescriptorSlice *output_calendars
) {
    result_string_storage.clear();
    result_descriptor_storage.clear();
    result_attendee_storage.clear();
    result_reminder_storage.clear();
    result_weekday_storage.clear();
    result_u8_storage.clear();
    result_i8_storage.clear();
    result_i16_storage.clear();

    jint status = env->CallIntMethod(response, calendar_list_response_get_status_method);
    if (status != static_cast<jint>(HOST_STATUS_OK)) {
        output_calendars->data = nullptr;
        output_calendars->len = 0;
        return static_cast<uint32_t>(status);
    }

    jobject calendars = env->CallObjectMethod(
        response,
        calendar_list_response_get_calendars_method
    );
    if (calendars == nullptr) {
        output_calendars->data = nullptr;
        output_calendars->len = 0;
        return static_cast<uint32_t>(status);
    }

    jint size = env->CallIntMethod(calendars, list_size_method);
    result_descriptor_storage.emplace_back();
    auto &storage = result_descriptor_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(calendars, list_get_method, index);
        storage.push_back(decode_calendar_descriptor(env, value));
        env->DeleteLocalRef(value);
    }

    output_calendars->data = storage.data();
    output_calendars->len = static_cast<uint32_t>(storage.size());
    env->DeleteLocalRef(calendars);

    return static_cast<uint32_t>(status);
}

/// Decode one calendar event-list response.
uint32_t decode_calendar_event_list_response(
    JNIEnv *env,
    jobject response,
    HostCalendarEventSlice *output_events
) {
    result_string_storage.clear();
    result_event_storage.clear();
    result_attendee_storage.clear();
    result_reminder_storage.clear();
    result_weekday_storage.clear();
    result_u8_storage.clear();
    result_i8_storage.clear();
    result_i16_storage.clear();

    jint status = env->CallIntMethod(response, calendar_event_list_response_get_status_method);
    if (status != static_cast<jint>(HOST_STATUS_OK)) {
        output_events->data = nullptr;
        output_events->len = 0;
        return static_cast<uint32_t>(status);
    }

    jobject events = env->CallObjectMethod(response, calendar_event_list_response_get_events_method);
    if (events == nullptr) {
        output_events->data = nullptr;
        output_events->len = 0;
        return static_cast<uint32_t>(status);
    }

    jint size = env->CallIntMethod(events, list_size_method);
    result_event_storage.emplace_back();
    auto &storage = result_event_storage.back();
    storage.reserve(static_cast<size_t>(size));

    for (jint index = 0; index < size; index += 1) {
        jobject value = env->CallObjectMethod(events, list_get_method, index);
        storage.push_back(decode_calendar_event(env, value));
        env->DeleteLocalRef(value);
    }

    output_events->data = storage.data();
    output_events->len = static_cast<uint32_t>(storage.size());
    env->DeleteLocalRef(events);

    return static_cast<uint32_t>(status);
}

/// Decode one calendar event-read response.
uint32_t decode_calendar_event_response(
    JNIEnv *env,
    jobject response,
    HostCalendarEvent *output_event
) {
    result_string_storage.clear();
    result_event_storage.clear();
    result_attendee_storage.clear();
    result_reminder_storage.clear();
    result_weekday_storage.clear();
    result_u8_storage.clear();
    result_i8_storage.clear();
    result_i16_storage.clear();

    jint status = env->CallIntMethod(response, calendar_event_response_get_status_method);
    if (status != static_cast<jint>(HOST_STATUS_OK)) {
        return static_cast<uint32_t>(status);
    }

    jobject event = env->CallObjectMethod(response, calendar_event_response_get_event_method);
    if (event == nullptr) {
        return static_cast<uint32_t>(status);
    }

    *output_event = decode_calendar_event(env, event);
    env->DeleteLocalRef(event);

    return static_cast<uint32_t>(status);
}

} // namespace

/// Resolve the calendar bridge methods from one runtime bridge instance.
bool resolve_calendar_methods(JNIEnv *env, jobject bridge) {
    return
        resolve_list_methods(env) &&
        resolve_array_list_methods(env) &&
        resolve_enum_methods(env) &&
        resolve_number_methods(env) &&
        resolve_bridge_methods(env, bridge) &&
        resolve_calendar_types(env);
}

/// Call the calendar-list entrypoint on one registered bridge.
uint32_t call_calendar_list_for_session(
    uint64_t session_handle,
    HostCalendarDescriptorSlice *output_calendars
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (output_calendars == nullptr) {
        return HOST_STATUS_FAILED;
    }

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jobject response = env->CallObjectMethod(bridge, list_calendars_method);
    env->DeleteLocalRef(bridge);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    uint32_t status = decode_calendar_list_response(env, response, output_calendars);
    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call the calendar event-list entrypoint on one registered bridge.
uint32_t call_calendar_event_list_for_session(
    uint64_t session_handle,
    HostCalendarQuery query,
    HostCalendarEventSlice *output_events
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (output_events == nullptr) {
        return HOST_STATUS_FAILED;
    }

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jobject query_value = new_calendar_query(env, query);
    if (query_value == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        list_calendar_events_method,
        query_value
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(query_value);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    uint32_t status = decode_calendar_event_list_response(env, response, output_events);
    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call the calendar event-read entrypoint on one registered bridge.
uint32_t call_calendar_event_read_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostCalendarEvent *output_event
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (output_event == nullptr) {
        return HOST_STATUS_FAILED;
    }

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
        read_calendar_event_method,
        identifier_value
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(identifier_value);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    uint32_t status = decode_calendar_event_response(env, response, output_event);
    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    return status;
}

/// Call the calendar event-create entrypoint on one registered bridge.
uint32_t call_calendar_event_create_for_session(
    uint64_t session_handle,
    HostCalendarEventDraft draft,
    NativeStringRef *output_identifier
) {
    JNIEnv *env = nullptr;
    bool did_attach_thread = false;

    if (output_identifier == nullptr) {
        return HOST_STATUS_FAILED;
    }

    if (!resolve_jni_env(&env, &did_attach_thread)) {
        return HOST_STATUS_FAILED;
    }

    jobject bridge = resolve_bridge(env, session_handle);
    if (bridge == nullptr) {
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_NOT_FOUND;
    }

    jobject draft_value = new_calendar_draft(env, draft);
    if (draft_value == nullptr) {
        env->DeleteLocalRef(bridge);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jobject response = env->CallObjectMethod(
        bridge,
        create_calendar_event_method,
        draft_value
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(draft_value);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    result_string_storage.clear();
    jint status = env->CallIntMethod(response, calendar_event_create_response_get_status_method);
    if (status == static_cast<jint>(HOST_STATUS_OK)) {
        jstring identifier_value = reinterpret_cast<jstring>(
            env->CallObjectMethod(response, calendar_event_create_response_get_id_method)
        );
        *output_identifier = string_ref_from_java(
            env,
            identifier_value,
            &result_string_storage
        );
        if (identifier_value != nullptr) {
            env->DeleteLocalRef(identifier_value);
        }
    }

    env->DeleteLocalRef(response);
    detach_jni_thread(did_attach_thread);

    return static_cast<uint32_t>(status);
}

/// Call the calendar event-update entrypoint on one registered bridge.
uint32_t call_calendar_event_update_for_session(
    uint64_t session_handle,
    NativeStringRef identifier,
    HostCalendarEventDraft draft
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
    jobject draft_value = new_calendar_draft(env, draft);
    if (identifier_value == nullptr || draft_value == nullptr) {
        env->DeleteLocalRef(bridge);
        if (identifier_value != nullptr) env->DeleteLocalRef(identifier_value);
        if (draft_value != nullptr) env->DeleteLocalRef(draft_value);
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    jint status = env->CallIntMethod(
        bridge,
        update_calendar_event_method,
        identifier_value,
        draft_value
    );

    env->DeleteLocalRef(bridge);
    env->DeleteLocalRef(identifier_value);
    env->DeleteLocalRef(draft_value);

    if (env->ExceptionCheck()) {
        env->ExceptionClear();
        detach_jni_thread(did_attach_thread);
        return HOST_STATUS_FAILED;
    }

    detach_jni_thread(did_attach_thread);

    return static_cast<uint32_t>(status);
}

/// Call the calendar event-delete entrypoint on one registered bridge.
uint32_t call_calendar_event_delete_for_session(
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
        delete_calendar_event_method,
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
