#ifndef DESTACK_RUNTIME_ANDROID_BRIDGE_TYPES_H
#define DESTACK_RUNTIME_ANDROID_BRIDGE_TYPES_H

#include <jni.h>

#include <stdint.h>

/// The host status code for one successful bridge call.
constexpr uint32_t HOST_STATUS_OK = 0;
/// The host status code for one missing bridge binding.
constexpr uint32_t HOST_STATUS_NOT_FOUND = 3;
/// The host status code for one buffer that was too small.
constexpr uint32_t HOST_STATUS_BUFFER_TOO_SMALL = 5;
/// The host status code for one generic bridge failure.
constexpr uint32_t HOST_STATUS_FAILED = 6;

/// One mutable byte slice passed through the Android bridge.
struct NativeSlice {
    /// The slice data pointer.
    uint8_t *data;
    /// The slice length in bytes.
    uint32_t len;
};

/// One borrowed string reference passed through the Android bridge.
struct NativeStringRef {
    /// The string data pointer.
    const uint8_t *data;
    /// The string length in bytes.
    uint32_t len;
};

/// One borrowed string-slice reference passed through the Android bridge.
struct NativeStringSlice {
    /// The slice data pointer.
    const NativeStringRef *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One optional string reference passed through the Android bridge.
struct HostOptionalStringRef {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped string reference.
    NativeStringRef value;
};

/// One optional `u32` passed through the Android bridge.
struct HostOptionalU32 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    uint32_t value;
};

/// One optional `u64` passed through the Android bridge.
struct HostOptionalU64 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    uint64_t value;
};

/// One optional `i8` passed through the Android bridge.
struct HostOptionalI8 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    int8_t value;
};

/// One `u8` slice passed through the Android bridge.
struct NativeU8Slice {
    /// The slice data pointer.
    const uint8_t *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One `i8` slice passed through the Android bridge.
struct NativeI8Slice {
    /// The slice data pointer.
    const int8_t *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One `i16` slice passed through the Android bridge.
struct NativeI16Slice {
    /// The slice data pointer.
    const int16_t *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One deferred document-pick request passed through the Android bridge.
struct HostDocumentRequest {
    /// The stable request identifier for this interactive host flow.
    uint64_t request_id;
    /// MIME-type filters, empty means any type.
    NativeStringSlice mime_types;
    /// File-extension filters without the leading dot.
    NativeStringSlice extensions;
    /// Whether multiple documents may be selected.
    bool allows_multiple_selection;
    /// Whether directory selection is allowed.
    bool allows_directory_selection;
    /// Whether the host should copy selected files into one runtime-visible sandbox path when possible.
    bool copies_to_sandbox;
};

/// One document descriptor passed through the Android bridge.
struct HostDocumentDescriptor {
    /// The stable URI or content identifier returned by the host.
    NativeStringRef uri;
    /// The normalized document name.
    NativeStringRef name;
    /// Whether the host provided one MIME type.
    bool has_mime_type;
    /// The normalized content type when available.
    NativeStringRef mime_type;
    /// Whether the host provided one size.
    bool has_size_bytes;
    /// The document size in bytes when available.
    uint64_t size_bytes;
    /// Whether the host provided one modification timestamp.
    bool has_modified_unix_ns;
    /// The document modification timestamp in UTC nanoseconds when available.
    uint64_t modified_unix_ns;
    /// Whether this descriptor represents one directory.
    bool is_directory;
    /// Whether the host provided one local path.
    bool has_local_path;
    /// The optional host-local path when the host exposes one directly.
    NativeStringRef local_path;
};

/// One document descriptor slice passed through the Android bridge.
struct HostDocumentDescriptorSlice {
    /// The slice data pointer.
    const HostDocumentDescriptor *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One deferred permission request passed through the Android bridge.
struct HostPermissionRequest {
    /// The stable request identifier for this interactive host flow.
    uint64_t request_id;
    /// The normalized permission names requested by the runtime.
    NativeStringSlice permissions;
};

/// One typed contact query passed through the Android bridge.
struct HostContactQuery {
    /// Whether the query carries one cursor.
    bool has_cursor;
    /// The opaque cursor from one prior list or search call.
    NativeStringRef cursor;
    /// Whether the query carries one limit.
    bool has_limit;
    /// The maximum returned contacts for this page.
    uint32_t limit;
    /// Whether phone values should be returned.
    bool include_phones;
    /// Whether email values should be returned.
    bool include_emails;
    /// Whether postal-address values should be returned.
    bool include_addresses;
    /// Whether organization metadata should be returned.
    bool include_organization;
    /// Whether note fields should be returned.
    bool include_notes;
};

/// One typed contact-name payload passed through the Android bridge.
struct HostContactName {
    /// The given or first name.
    NativeStringRef given_name;
    /// The middle name.
    NativeStringRef middle_name;
    /// The family or last name.
    NativeStringRef family_name;
    /// The honorific prefix.
    NativeStringRef prefix;
    /// The honorific suffix.
    NativeStringRef suffix;
    /// The nickname.
    NativeStringRef nickname;
    /// The phonetic given name.
    NativeStringRef phonetic_given_name;
    /// The phonetic family name.
    NativeStringRef phonetic_family_name;
};

/// One typed contact-phone payload passed through the Android bridge.
struct HostContactPhone {
    /// The user-visible label for this phone value.
    NativeStringRef label;
    /// The original phone number string.
    NativeStringRef number;
    /// The normalized phone number string when available.
    NativeStringRef normalized_number;
    /// Whether this phone value is marked as primary.
    bool primary;
};

/// One typed contact-phone slice passed through the Android bridge.
struct HostContactPhoneSlice {
    /// The slice data pointer.
    const HostContactPhone *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One typed contact-email payload passed through the Android bridge.
struct HostContactEmail {
    /// The user-visible label for this email value.
    NativeStringRef label;
    /// The email address.
    NativeStringRef address;
    /// Whether this email value is marked as primary.
    bool primary;
};

/// One typed contact-email slice passed through the Android bridge.
struct HostContactEmailSlice {
    /// The slice data pointer.
    const HostContactEmail *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One typed contact-address payload passed through the Android bridge.
struct HostContactAddress {
    /// The user-visible label for this address value.
    NativeStringRef label;
    /// The street-line payload.
    NativeStringRef street;
    /// The city payload.
    NativeStringRef city;
    /// The region or state payload.
    NativeStringRef region;
    /// The postal-code payload.
    NativeStringRef postal_code;
    /// The country payload.
    NativeStringRef country;
    /// The country-code payload.
    NativeStringRef country_code;
};

/// One typed contact-address slice passed through the Android bridge.
struct HostContactAddressSlice {
    /// The slice data pointer.
    const HostContactAddress *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One typed contact-organization payload passed through the Android bridge.
struct HostContactOrganization {
    /// The company or organization name.
    NativeStringRef company;
    /// The department name.
    NativeStringRef department;
    /// The job title.
    NativeStringRef title;
};

/// One typed contact payload passed through the Android bridge.
struct HostContact {
    /// The stable host contact identifier.
    NativeStringRef id;
    /// The structured name payload.
    HostContactName name;
    /// The phone values.
    HostContactPhoneSlice phones;
    /// The email values.
    HostContactEmailSlice emails;
    /// The address values.
    HostContactAddressSlice addresses;
    /// The organization metadata.
    HostContactOrganization organization;
    /// The contact note payload.
    NativeStringRef note;
};

/// One typed contact slice passed through the Android bridge.
struct HostContactSlice {
    /// The slice data pointer.
    const HostContact *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One typed contact draft passed through the Android bridge.
struct HostContactDraft {
    /// The structured name payload.
    HostContactName name;
    /// The phone values.
    HostContactPhoneSlice phones;
    /// The email values.
    HostContactEmailSlice emails;
    /// The address values.
    HostContactAddressSlice addresses;
    /// The organization metadata.
    HostContactOrganization organization;
    /// The contact note payload.
    NativeStringRef note;
};

/// One typed contact page passed through the Android bridge.
struct HostContactPage {
    /// The returned contacts for this page.
    HostContactSlice contacts;
    /// The opaque next-page cursor.
    NativeStringRef next_cursor;
    /// Whether more contacts are available.
    bool has_more;
};

/// The calendar access level passed through the Android bridge.
enum HostCalendarAccess : int32_t {
    /// Read-only access.
    HOST_CALENDAR_ACCESS_READ = 1,
    /// Read-write access.
    HOST_CALENDAR_ACCESS_WRITE = 2,
};

/// The calendar availability class passed through the Android bridge.
enum HostCalendarAvailability : int32_t {
    /// Busy slot.
    HOST_CALENDAR_AVAILABILITY_BUSY = 1,
    /// Free slot.
    HOST_CALENDAR_AVAILABILITY_FREE = 2,
    /// Tentative slot.
    HOST_CALENDAR_AVAILABILITY_TENTATIVE = 3,
    /// Out-of-office slot.
    HOST_CALENDAR_AVAILABILITY_OUT_OF_OFFICE = 4,
    /// Unavailable slot.
    HOST_CALENDAR_AVAILABILITY_UNAVAILABLE = 5,
    /// Unknown slot.
    HOST_CALENDAR_AVAILABILITY_UNKNOWN = 6,
};

/// The participant response status passed through the Android bridge.
enum HostCalendarParticipantStatus : int32_t {
    /// Unknown response.
    HOST_CALENDAR_PARTICIPANT_STATUS_UNKNOWN = 1,
    /// Pending response.
    HOST_CALENDAR_PARTICIPANT_STATUS_PENDING = 2,
    /// Accepted response.
    HOST_CALENDAR_PARTICIPANT_STATUS_ACCEPTED = 3,
    /// Tentative response.
    HOST_CALENDAR_PARTICIPANT_STATUS_TENTATIVE = 4,
    /// Declined response.
    HOST_CALENDAR_PARTICIPANT_STATUS_DECLINED = 5,
    /// Delegated response.
    HOST_CALENDAR_PARTICIPANT_STATUS_DELEGATED = 6,
    /// Completed response.
    HOST_CALENDAR_PARTICIPANT_STATUS_COMPLETED = 7,
    /// In-process response.
    HOST_CALENDAR_PARTICIPANT_STATUS_IN_PROCESS = 8,
};

/// The recurrence frequency passed through the Android bridge.
enum HostCalendarRecurrenceFrequency : int32_t {
    /// Daily recurrence.
    HOST_CALENDAR_RECURRENCE_FREQUENCY_DAILY = 1,
    /// Weekly recurrence.
    HOST_CALENDAR_RECURRENCE_FREQUENCY_WEEKLY = 2,
    /// Monthly recurrence.
    HOST_CALENDAR_RECURRENCE_FREQUENCY_MONTHLY = 3,
    /// Yearly recurrence.
    HOST_CALENDAR_RECURRENCE_FREQUENCY_YEARLY = 4,
};

/// The reminder kind passed through the Android bridge.
enum HostCalendarReminderKind : int32_t {
    /// One absolute reminder.
    HOST_CALENDAR_REMINDER_KIND_ABSOLUTE = 1,
    /// One relative reminder.
    HOST_CALENDAR_REMINDER_KIND_RELATIVE = 2,
};

/// One recurrence weekday payload passed through the Android bridge.
struct HostCalendarRecurrenceWeekday {
    /// The ISO-8601 weekday number.
    uint8_t day;
    /// The optional week number.
    HostOptionalI8 week_number;
};

/// One recurrence weekday slice passed through the Android bridge.
struct HostCalendarRecurrenceWeekdaySlice {
    /// The slice data pointer.
    const HostCalendarRecurrenceWeekday *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One recurrence rule payload passed through the Android bridge.
struct HostCalendarRecurrenceRule {
    /// The recurrence frequency.
    HostCalendarRecurrenceFrequency frequency;
    /// The recurrence interval.
    uint32_t interval;
    /// The optional occurrence count.
    HostOptionalU32 count;
    /// The optional recurrence end timestamp.
    HostOptionalU64 until_unix_ns;
    /// The weekday numbers.
    NativeU8Slice by_week_days;
    /// The structured weekday selectors.
    HostCalendarRecurrenceWeekdaySlice by_weekday_ordinals;
    /// The day-of-month set.
    NativeI8Slice by_month_days;
    /// The month set.
    NativeU8Slice by_months;
    /// The day-of-year set.
    NativeI16Slice by_year_days;
    /// The week-of-year set.
    NativeI8Slice by_week_numbers;
    /// The set-position filters.
    NativeI16Slice by_set_positions;
};

/// One calendar attendee payload passed through the Android bridge.
struct HostCalendarAttendee {
    /// The optional attendee identifier.
    HostOptionalStringRef id;
    /// The optional attendee name.
    HostOptionalStringRef name;
    /// The optional attendee email address.
    HostOptionalStringRef email;
    /// Whether this attendee is optional.
    bool optional;
    /// Whether this attendee is the organizer.
    bool organizer;
    /// The attendee response status.
    HostCalendarParticipantStatus response_status;
};

/// One calendar attendee slice passed through the Android bridge.
struct HostCalendarAttendeeSlice {
    /// The slice data pointer.
    const HostCalendarAttendee *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One calendar reminder payload passed through the Android bridge.
struct HostCalendarReminder {
    /// The reminder variant kind.
    HostCalendarReminderKind kind;
    /// The absolute timestamp for one absolute reminder.
    uint64_t absolute_unix_ns;
    /// The minutes-before-start value for one relative reminder.
    int32_t minutes_before_start;
};

/// One calendar reminder slice passed through the Android bridge.
struct HostCalendarReminderSlice {
    /// The slice data pointer.
    const HostCalendarReminder *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One calendar descriptor passed through the Android bridge.
struct HostCalendarDescriptor {
    /// The stable calendar identifier.
    NativeStringRef id;
    /// The calendar title.
    NativeStringRef title;
    /// The source or account label.
    NativeStringRef source;
    /// The optional owner label.
    HostOptionalStringRef owner;
    /// The ARGB color value.
    uint32_t color_argb;
    /// Whether this is the primary write target.
    bool primary;
    /// The calendar access level.
    HostCalendarAccess access;
};

/// One calendar descriptor slice passed through the Android bridge.
struct HostCalendarDescriptorSlice {
    /// The slice data pointer.
    const HostCalendarDescriptor *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One calendar event query passed through the Android bridge.
struct HostCalendarQuery {
    /// The selected calendar identifiers.
    NativeStringSlice calendar_ids;
    /// The query start timestamp.
    uint64_t start_unix_ns;
    /// The query end timestamp.
    uint64_t end_unix_ns;
    /// The optional page limit.
    HostOptionalU32 limit;
    /// Whether canceled events should be included.
    bool include_canceled;
    /// Whether declined events should be included.
    bool include_declined;
    /// Whether recurrence instances should be included.
    bool include_recurrence_instances;
};

/// One calendar event payload passed through the Android bridge.
struct HostCalendarEvent {
    /// The stable event identifier.
    NativeStringRef id;
    /// The calendar identifier.
    NativeStringRef calendar_id;
    /// The event title.
    NativeStringRef title;
    /// The optional notes.
    HostOptionalStringRef notes;
    /// The optional location.
    HostOptionalStringRef location;
    /// The start timestamp.
    uint64_t start_unix_ns;
    /// The end timestamp.
    uint64_t end_unix_ns;
    /// Whether this event is all-day.
    bool all_day;
    /// Whether this event is canceled.
    bool canceled;
    /// The optional timezone identifier.
    HostOptionalStringRef time_zone;
    /// The availability class.
    HostCalendarAvailability availability;
    /// The optional URL.
    HostOptionalStringRef url;
    /// The optional organizer name.
    HostOptionalStringRef organizer_name;
    /// The optional organizer email address.
    HostOptionalStringRef organizer_email;
    /// Whether this event is recurring.
    bool recurring;
    /// The optional recurrence master identifier.
    HostOptionalStringRef recurrence_master_id;
    /// The optional recurrence instance timestamp.
    HostOptionalU64 recurrence_id_unix_ns;
    /// Whether the recurrence rule field is present.
    bool has_recurrence_rule;
    /// The recurrence rule payload.
    HostCalendarRecurrenceRule recurrence_rule;
    /// Whether the attendees field is present.
    bool has_attendees;
    /// The attendee slice.
    HostCalendarAttendeeSlice attendees;
    /// Whether the reminders field is present.
    bool has_reminders;
    /// The reminder slice.
    HostCalendarReminderSlice reminders;
};

/// One calendar event slice passed through the Android bridge.
struct HostCalendarEventSlice {
    /// The slice data pointer.
    const HostCalendarEvent *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One calendar event draft passed through the Android bridge.
struct HostCalendarEventDraft {
    /// The calendar identifier.
    NativeStringRef calendar_id;
    /// The event title.
    NativeStringRef title;
    /// The optional notes.
    HostOptionalStringRef notes;
    /// The optional location.
    HostOptionalStringRef location;
    /// The start timestamp.
    uint64_t start_unix_ns;
    /// The end timestamp.
    uint64_t end_unix_ns;
    /// Whether this event is all-day.
    bool all_day;
    /// The optional timezone identifier.
    HostOptionalStringRef time_zone;
    /// The availability class.
    HostCalendarAvailability availability;
    /// The optional URL.
    HostOptionalStringRef url;
    /// Whether the recurrence rule field is present.
    bool has_recurrence_rule;
    /// The recurrence rule payload.
    HostCalendarRecurrenceRule recurrence_rule;
    /// Whether the attendees field is present.
    bool has_attendees;
    /// The attendee slice.
    HostCalendarAttendeeSlice attendees;
    /// Whether the reminders field is present.
    bool has_reminders;
    /// The reminder slice.
    HostCalendarReminderSlice reminders;
};

/// One simplified notification request passed through the Android bridge.
struct HostNotificationRequest {
    /// The stable runtime notification identifier.
    NativeStringRef identifier;
    /// The primary notification title.
    NativeStringRef title;
    /// The primary notification body text.
    NativeStringRef body;
};

/// The location accuracy preference passed through the Android bridge.
enum LocationAccuracy : int32_t {
    /// Passive updates with minimal power use.
    LOCATION_ACCURACY_PASSIVE = 1,
    /// Coarse accuracy.
    LOCATION_ACCURACY_LOW = 2,
    /// Balanced power and accuracy.
    LOCATION_ACCURACY_BALANCED = 3,
    /// Fine accuracy.
    LOCATION_ACCURACY_HIGH = 4,
    /// Best available accuracy.
    LOCATION_ACCURACY_BEST = 5,
};

/// One location watch options payload passed through the Android bridge.
struct LocationWatchOptions {
    /// The requested accuracy preference.
    LocationAccuracy accuracy;
    /// The minimum interval between updates in nanoseconds.
    uint64_t minimum_interval_ns;
    /// The minimum distance delta in meters.
    double minimum_distance_meters;
    /// Whether heading should be included when available.
    bool include_heading;
};

/// One location sample payload passed through the Android bridge.
struct LocationSample {
    /// The latitude in degrees.
    double latitude_degrees;
    /// The longitude in degrees.
    double longitude_degrees;
    /// The altitude in meters above mean sea level.
    double altitude_meters;
    /// The horizontal accuracy radius in meters.
    double horizontal_accuracy_meters;
    /// The vertical accuracy in meters.
    double vertical_accuracy_meters;
    /// The speed in meters per second.
    double speed_meters_per_second;
    /// The heading in degrees.
    double heading_degrees;
    /// The UTC timestamp in nanoseconds.
    uint64_t timestamp_unix_ns;
};

/// The typed media-asset kind passed through the Android bridge.
enum HostMediaAssetKind : int32_t {
    /// One image asset.
    HOST_MEDIA_ASSET_KIND_IMAGE = 1,
    /// One video asset.
    HOST_MEDIA_ASSET_KIND_VIDEO = 2,
    /// One audio asset.
    HOST_MEDIA_ASSET_KIND_AUDIO = 3,
    /// One non-standard asset.
    HOST_MEDIA_ASSET_KIND_OTHER = 4,
};

/// One typed media-kind slice passed through the Android bridge.
struct HostMediaAssetKindSlice {
    /// The slice data pointer.
    const HostMediaAssetKind *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One typed media query passed through the Android bridge.
struct HostMediaQuery {
    /// Whether the query carries one cursor.
    bool has_cursor;
    /// The opaque cursor from one prior media-list call.
    NativeStringRef cursor;
    /// Whether the query carries one limit.
    bool has_limit;
    /// The maximum returned assets for this page.
    uint32_t limit;
    /// The included asset kinds, empty means all kinds.
    HostMediaAssetKindSlice kinds;
    /// Whether hidden assets should be included.
    bool include_hidden;
};

/// One typed media descriptor passed through the Android bridge.
struct HostMediaAssetDescriptor {
    /// The stable asset identifier.
    NativeStringRef id;
    /// The host URI for this asset.
    NativeStringRef uri;
    /// The asset filename payload.
    NativeStringRef filename;
    /// The asset MIME type payload when available.
    NativeStringRef mime_type;
    /// The asset class.
    HostMediaAssetKind kind;
    /// The asset width in pixels when available.
    uint32_t width;
    /// The asset height in pixels when available.
    uint32_t height;
    /// The asset duration in milliseconds for time-based assets.
    uint64_t duration_ms;
    /// The asset size in bytes when available.
    uint64_t size_bytes;
    /// The asset creation timestamp in UTC nanoseconds when available.
    uint64_t created_unix_ns;
    /// The asset modification timestamp in UTC nanoseconds when available.
    uint64_t modified_unix_ns;
};

/// One typed media-descriptor slice passed through the Android bridge.
struct HostMediaAssetDescriptorSlice {
    /// The slice data pointer.
    const HostMediaAssetDescriptor *data;
    /// The slice length in elements.
    uint32_t len;
};

/// One typed media page passed through the Android bridge.
struct HostMediaPage {
    /// The returned assets for this page.
    HostMediaAssetDescriptorSlice assets;
    /// Whether the page carries one next cursor.
    bool has_next_cursor;
    /// The opaque next-page cursor when available.
    NativeStringRef next_cursor;
    /// Whether more assets are available.
    bool has_more;
};

/// The simplified notification event kind passed through the Android bridge.
enum HostNotificationEventKind : uint32_t {
    /// The notification was delivered.
    HOST_NOTIFICATION_EVENT_DELIVERED = 1,
    /// The notification was activated by the user.
    HOST_NOTIFICATION_EVENT_ACTIVATED = 2,
    /// The notification was dismissed.
    HOST_NOTIFICATION_EVENT_DISMISSED = 3,
};

/// One simplified notification event passed through the Android bridge.
struct HostNotificationEvent {
    /// The notification interaction kind.
    HostNotificationEventKind kind;
    /// The event sequence number for this stream.
    uint64_t sequence;
    /// The monotonic event timestamp in nanoseconds.
    uint64_t timestamp_ns;
    /// The simplified request associated with this event.
    HostNotificationRequest request;
    /// Whether the host provided one action identifier.
    bool has_action_identifier;
    /// The action identifier for interactive notifications when available.
    NativeStringRef action_identifier;
};

/// The document callbacks registered for one runtime session.
struct AndroidHostDocumentCallbacks {
    /// The deferred document-pick request callback.
    uint32_t (*pick)(uint64_t session_handle, HostDocumentRequest request);
};

/// The permission callbacks registered for one runtime session.
struct AndroidHostPermissionCallbacks {
    /// The permission-settings callback.
    uint32_t (*open_settings)(uint64_t session_handle);
    /// The permission request callback.
    uint32_t (*request)(uint64_t session_handle, HostPermissionRequest request);
};

/// The calendar callbacks registered for one runtime session.
struct AndroidHostCalendarCallbacks {
    /// The calendar-list callback.
    uint32_t (*list)(
        uint64_t session_handle,
        HostCalendarDescriptorSlice *output_calendars
    );
    /// The calendar event-list callback.
    uint32_t (*event_list)(
        uint64_t session_handle,
        HostCalendarQuery query,
        HostCalendarEventSlice *output_events
    );
    /// The calendar event-read callback.
    uint32_t (*event_read)(
        uint64_t session_handle,
        NativeStringRef identifier,
        HostCalendarEvent *output_event
    );
    /// The calendar event-create callback.
    uint32_t (*event_create)(
        uint64_t session_handle,
        HostCalendarEventDraft draft,
        NativeStringRef *output_identifier
    );
    /// The calendar event-update callback.
    uint32_t (*event_update)(
        uint64_t session_handle,
        NativeStringRef identifier,
        HostCalendarEventDraft draft
    );
    /// The calendar event-delete callback.
    uint32_t (*event_delete)(
        uint64_t session_handle,
        NativeStringRef identifier
    );
};

/// The intent callbacks registered for one runtime session.
struct AndroidHostIntentCallbacks {
    /// The can-open-url callback.
    uint32_t (*can_open_url)(
        uint64_t session_handle,
        NativeStringRef url,
        bool *is_supported
    );
    /// The open-url callback.
    uint32_t (*open_url)(uint64_t session_handle, NativeStringRef url);
    /// The open-path callback.
    uint32_t (*open_path)(uint64_t session_handle, NativeStringRef path);
    /// The share-text callback.
    uint32_t (*share_text)(
        uint64_t session_handle,
        NativeStringRef text,
        bool has_mime_type,
        NativeStringRef mime_type
    );
    /// The share-paths callback.
    uint32_t (*share_paths)(
        uint64_t session_handle,
        NativeStringSlice paths,
        bool has_mime_type,
        NativeStringRef mime_type
    );
};

/// The contact callbacks registered for one runtime session.
struct AndroidHostContactCallbacks {
    /// The contact-list callback.
    uint32_t (*list)(
        uint64_t session_handle,
        HostContactQuery query,
        HostContactPage *output_page
    );
    /// The contact-search callback.
    uint32_t (*search)(
        uint64_t session_handle,
        NativeStringRef query_text,
        HostContactQuery query,
        HostContactPage *output_page
    );
    /// The contact-read callback.
    uint32_t (*read)(
        uint64_t session_handle,
        NativeStringRef identifier,
        HostContact *output_contact
    );
    /// The contact-create callback.
    uint32_t (*create)(
        uint64_t session_handle,
        HostContactDraft draft,
        NativeStringRef *output_identifier
    );
    /// The contact-update callback.
    uint32_t (*update)(
        uint64_t session_handle,
        NativeStringRef identifier,
        HostContactDraft draft
    );
    /// The contact-delete callback.
    uint32_t (*delete_contact)(
        uint64_t session_handle,
        NativeStringRef identifier
    );
};

/// The location callbacks registered for one runtime session.
struct AndroidHostLocationCallbacks {
    /// The location-services-enabled callback.
    uint32_t (*services_enabled)(
        uint64_t session_handle,
        bool *is_enabled
    );
    /// The last-known-location callback.
    uint32_t (*last_known)(
        uint64_t session_handle,
        LocationSample *sample
    );
    /// The location-watch-open callback.
    uint32_t (*watch_open)(
        uint64_t session_handle,
        NativeStringRef watch_id,
        LocationWatchOptions options
    );
    /// The location-watch-close callback.
    uint32_t (*watch_close)(
        uint64_t session_handle,
        NativeStringRef watch_id
    );
};

/// The notification callbacks registered for one runtime session.
struct AndroidHostNotificationCallbacks {
    /// The cancel callback.
    uint32_t (*cancel)(uint64_t session_handle, NativeStringRef identifier);
    /// The cancel-all callback.
    uint32_t (*cancel_all)(uint64_t session_handle);
    /// The post callback.
    uint32_t (*post)(uint64_t session_handle, HostNotificationRequest request);
};

/// The media callbacks registered for one runtime session.
struct AndroidHostMediaCallbacks {
    /// The media-list callback.
    uint32_t (*list)(
        uint64_t session_handle,
        HostMediaQuery query,
        HostMediaPage *output_page
    );
    /// The media-read callback.
    uint32_t (*describe)(
        uint64_t session_handle,
        NativeStringRef identifier,
        HostMediaAssetDescriptor *output_descriptor
    );
    /// The media-import callback.
    uint32_t (*import_path)(
        uint64_t session_handle,
        NativeStringRef path,
        int32_t kind,
        NativeStringRef *output_identifier
    );
    /// The media-delete callback.
    uint32_t (*delete_media)(
        uint64_t session_handle,
        NativeStringSlice identifiers,
        uint32_t *deleted_count
    );
};

/// The mobile runtime-bridge callbacks registered for one runtime session.
struct AndroidRuntimeBridgeBindings {
    /// The document callbacks.
    AndroidHostDocumentCallbacks document;
    /// The permission callbacks.
    AndroidHostPermissionCallbacks permission;
    /// The calendar callbacks.
    AndroidHostCalendarCallbacks calendar;
    /// The contact callbacks.
    AndroidHostContactCallbacks contact;
    /// The intent callbacks.
    AndroidHostIntentCallbacks intent;
    /// The location callbacks.
    AndroidHostLocationCallbacks location;
    /// The media callbacks.
    AndroidHostMediaCallbacks media;
    /// The notification callbacks.
    AndroidHostNotificationCallbacks notification;
};

/// One runtime status returned by one bridge ingress call.
struct RuntimeStatus {
    /// The status code.
    uint32_t code;
    /// The optional runtime error identifier.
    uint64_t error_id;
};

/// The runtime function that registers one mobile bridge callback table for one session.
using RegisterRuntimeBridgeBindingsFunction =
    uint32_t (*)(uint64_t session_handle, AndroidRuntimeBridgeBindings callbacks);
/// The runtime function that unregisters one mobile bridge callback table for one session.
using UnregisterRuntimeBridgeBindingsFunction =
    void (*)(uint64_t session_handle);
/// The runtime function that receives one document result.
using NotifyDocumentResultFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        uint64_t request_id,
        HostDocumentDescriptorSlice documents
    );
/// The runtime function that receives one notification event.
using NotifyNotificationEventFunction =
    RuntimeStatus (*)(uint64_t session_handle, HostNotificationEvent event);
/// The runtime function that receives one intent open-url event.
using NotifyIntentOpenUrlFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        bool has_source,
        NativeStringRef source,
        NativeStringRef url
    );
/// The runtime function that receives one intent open-file event.
using NotifyIntentOpenFileFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        bool has_source,
        NativeStringRef source,
        NativeStringRef path,
        bool has_mime_type,
        NativeStringRef mime_type
    );
/// The runtime function that receives one intent share-text event.
using NotifyIntentShareTextFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        bool has_source,
        NativeStringRef source,
        NativeStringRef text,
        bool has_mime_type,
        NativeStringRef mime_type
    );
/// The runtime function that receives one intent share-files event.
using NotifyIntentShareFilesFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        bool has_source,
        NativeStringRef source,
        NativeStringSlice paths,
        bool has_mime_type,
        NativeStringRef mime_type
    );
/// The runtime function that receives one intent custom-action event.
using NotifyIntentCustomActionFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        bool has_source,
        NativeStringRef source,
        NativeStringRef action,
        bool has_url,
        NativeStringRef url,
        NativeStringSlice paths,
        bool has_text,
        NativeStringRef text,
        bool has_mime_type,
        NativeStringRef mime_type
    );
/// The runtime function that receives one permission result.
using NotifyPermissionResultFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        bool has_request_id,
        uint64_t request_id,
        NativeStringRef permission,
        bool granted
    );
/// The runtime function that receives one location sample.
using NotifyLocationSampleFunction =
    RuntimeStatus (*)(
        uint64_t session_handle,
        NativeStringRef watch_id,
        LocationSample sample
    );
#endif
