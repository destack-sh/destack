#ifndef RUNTIME_HOST_IOS_BRIDGE_H
#define RUNTIME_HOST_IOS_BRIDGE_H

#include <stdbool.h>
#include <stdint.h>

/// One runtime status returned by one ingress call.
typedef struct DestackRustRuntimeStatus {
    /// The status code.
    uint32_t code;
    /// The optional runtime error identifier.
    uint64_t error_id;
} DestackRustRuntimeStatus;

/// One string reference passed through the Apple bridge.
typedef struct DestackRustStringRef {
    /// The string data pointer.
    const uint8_t *data;
    /// The string length in bytes.
    uint32_t len;
} DestackRustStringRef;

/// One string slice passed through the Apple bridge.
typedef struct DestackRustStringSlice {
    /// The slice data pointer.
    const DestackRustStringRef *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustStringSlice;

/// One deferred document-pick request passed through the Apple bridge.
typedef struct DestackRustDocumentRequest {
    /// The stable request identifier for this interactive host flow.
    uint64_t request_id;
    /// MIME-type filters, empty means any type.
    DestackRustStringSlice mime_types;
    /// File-extension filters without the leading dot.
    DestackRustStringSlice extensions;
    /// Whether multiple documents may be selected.
    bool allows_multiple_selection;
    /// Whether directory selection is allowed.
    bool allows_directory_selection;
    /// Whether the host should copy selected files into one runtime-visible sandbox path when possible.
    bool copies_to_sandbox;
} DestackRustDocumentRequest;

/// One document descriptor passed through the Apple bridge.
typedef struct DestackRustDocumentDescriptor {
    /// The stable URI or content identifier returned by the host.
    DestackRustStringRef uri;
    /// The normalized document name.
    DestackRustStringRef name;
    /// Whether the host provided one MIME type.
    bool has_mime_type;
    /// The normalized content type when available.
    DestackRustStringRef mime_type;
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
    DestackRustStringRef local_path;
} DestackRustDocumentDescriptor;

/// One document descriptor slice passed through the Apple bridge.
typedef struct DestackRustDocumentDescriptorSlice {
    /// The slice data pointer.
    const DestackRustDocumentDescriptor *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustDocumentDescriptorSlice;

/// One deferred permission request passed through the Apple bridge.
typedef struct DestackRustPermissionRequest {
    /// The stable request identifier for this interactive host flow.
    uint64_t request_id;
    /// The normalized permission names requested by the runtime.
    DestackRustStringSlice permissions;
} DestackRustPermissionRequest;

/// One typed contact query passed through the Apple bridge.
typedef struct DestackRustContactQuery {
    /// Whether the query carries one cursor.
    bool has_cursor;
    /// The opaque cursor from one prior list or search call.
    DestackRustStringRef cursor;
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
} DestackRustContactQuery;

/// One typed contact-name payload passed through the Apple bridge.
typedef struct DestackRustContactName {
    /// The given or first name.
    DestackRustStringRef given_name;
    /// The middle name.
    DestackRustStringRef middle_name;
    /// The family or last name.
    DestackRustStringRef family_name;
    /// The honorific prefix.
    DestackRustStringRef prefix;
    /// The honorific suffix.
    DestackRustStringRef suffix;
    /// The nickname.
    DestackRustStringRef nickname;
    /// The phonetic given name.
    DestackRustStringRef phonetic_given_name;
    /// The phonetic family name.
    DestackRustStringRef phonetic_family_name;
} DestackRustContactName;

/// One typed contact-phone payload passed through the Apple bridge.
typedef struct DestackRustContactPhone {
    /// The user-visible label for this phone value.
    DestackRustStringRef label;
    /// The original phone number string.
    DestackRustStringRef number;
    /// The normalized phone number string when available.
    DestackRustStringRef normalized_number;
    /// Whether this phone value is marked as primary.
    bool primary;
} DestackRustContactPhone;

/// One typed contact-phone slice passed through the Apple bridge.
typedef struct DestackRustContactPhoneSlice {
    /// The slice data pointer.
    const DestackRustContactPhone *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustContactPhoneSlice;

/// One typed contact-email payload passed through the Apple bridge.
typedef struct DestackRustContactEmail {
    /// The user-visible label for this email value.
    DestackRustStringRef label;
    /// The email address.
    DestackRustStringRef address;
    /// Whether this email value is marked as primary.
    bool primary;
} DestackRustContactEmail;

/// One typed contact-email slice passed through the Apple bridge.
typedef struct DestackRustContactEmailSlice {
    /// The slice data pointer.
    const DestackRustContactEmail *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustContactEmailSlice;

/// One typed contact-address payload passed through the Apple bridge.
typedef struct DestackRustContactAddress {
    /// The user-visible label for this address value.
    DestackRustStringRef label;
    /// The street-line payload.
    DestackRustStringRef street;
    /// The city payload.
    DestackRustStringRef city;
    /// The region or state payload.
    DestackRustStringRef region;
    /// The postal-code payload.
    DestackRustStringRef postal_code;
    /// The country payload.
    DestackRustStringRef country;
    /// The country-code payload.
    DestackRustStringRef country_code;
} DestackRustContactAddress;

/// One typed contact-address slice passed through the Apple bridge.
typedef struct DestackRustContactAddressSlice {
    /// The slice data pointer.
    const DestackRustContactAddress *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustContactAddressSlice;

/// One typed contact-organization payload passed through the Apple bridge.
typedef struct DestackRustContactOrganization {
    /// The company or organization name.
    DestackRustStringRef company;
    /// The department name.
    DestackRustStringRef department;
    /// The job title.
    DestackRustStringRef title;
} DestackRustContactOrganization;

/// One typed contact payload passed through the Apple bridge.
typedef struct DestackRustContact {
    /// The stable host contact identifier.
    DestackRustStringRef id;
    /// The structured name payload.
    DestackRustContactName name;
    /// The phone values.
    DestackRustContactPhoneSlice phones;
    /// The email values.
    DestackRustContactEmailSlice emails;
    /// The address values.
    DestackRustContactAddressSlice addresses;
    /// The organization metadata.
    DestackRustContactOrganization organization;
    /// The contact note payload.
    DestackRustStringRef note;
} DestackRustContact;

/// One typed contact slice passed through the Apple bridge.
typedef struct DestackRustContactSlice {
    /// The slice data pointer.
    const DestackRustContact *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustContactSlice;

/// One typed contact draft passed through the Apple bridge.
typedef struct DestackRustContactDraft {
    /// The structured name payload.
    DestackRustContactName name;
    /// The phone values.
    DestackRustContactPhoneSlice phones;
    /// The email values.
    DestackRustContactEmailSlice emails;
    /// The address values.
    DestackRustContactAddressSlice addresses;
    /// The organization metadata.
    DestackRustContactOrganization organization;
    /// The contact note payload.
    DestackRustStringRef note;
} DestackRustContactDraft;

/// One typed contact page passed through the Apple bridge.
typedef struct DestackRustContactPage {
    /// The returned contacts for this page.
    DestackRustContactSlice contacts;
    /// The opaque next-page cursor.
    DestackRustStringRef next_cursor;
    /// Whether more contacts are available.
    bool has_more;
} DestackRustContactPage;

/// The calendar access level passed through the Apple bridge.
typedef enum DestackRustCalendarAccess {
    /// Read-only access.
    DESTACK_RUST_CALENDAR_ACCESS_READ = 1,
    /// Read-write access.
    DESTACK_RUST_CALENDAR_ACCESS_WRITE = 2,
} DestackRustCalendarAccess;

/// The calendar availability class passed through the Apple bridge.
typedef enum DestackRustCalendarAvailability {
    /// Busy slot.
    DESTACK_RUST_CALENDAR_AVAILABILITY_BUSY = 1,
    /// Free slot.
    DESTACK_RUST_CALENDAR_AVAILABILITY_FREE = 2,
    /// Tentative slot.
    DESTACK_RUST_CALENDAR_AVAILABILITY_TENTATIVE = 3,
    /// Out-of-office slot.
    DESTACK_RUST_CALENDAR_AVAILABILITY_OUT_OF_OFFICE = 4,
    /// Unavailable slot.
    DESTACK_RUST_CALENDAR_AVAILABILITY_UNAVAILABLE = 5,
    /// Unknown slot.
    DESTACK_RUST_CALENDAR_AVAILABILITY_UNKNOWN = 6,
} DestackRustCalendarAvailability;

/// The participant response status passed through the Apple bridge.
typedef enum DestackRustCalendarParticipantStatus {
    /// Unknown response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_UNKNOWN = 1,
    /// Pending response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_PENDING = 2,
    /// Accepted response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_ACCEPTED = 3,
    /// Tentative response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_TENTATIVE = 4,
    /// Declined response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_DECLINED = 5,
    /// Delegated response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_DELEGATED = 6,
    /// Completed response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_COMPLETED = 7,
    /// In-process response.
    DESTACK_RUST_CALENDAR_PARTICIPANT_STATUS_IN_PROCESS = 8,
} DestackRustCalendarParticipantStatus;

/// The recurrence frequency passed through the Apple bridge.
typedef enum DestackRustCalendarRecurrenceFrequency {
    /// Daily recurrence.
    DESTACK_RUST_CALENDAR_RECURRENCE_FREQUENCY_DAILY = 1,
    /// Weekly recurrence.
    DESTACK_RUST_CALENDAR_RECURRENCE_FREQUENCY_WEEKLY = 2,
    /// Monthly recurrence.
    DESTACK_RUST_CALENDAR_RECURRENCE_FREQUENCY_MONTHLY = 3,
    /// Yearly recurrence.
    DESTACK_RUST_CALENDAR_RECURRENCE_FREQUENCY_YEARLY = 4,
} DestackRustCalendarRecurrenceFrequency;

/// The reminder kind passed through the Apple bridge.
typedef enum DestackRustCalendarReminderKind {
    /// One absolute reminder.
    DESTACK_RUST_CALENDAR_REMINDER_KIND_ABSOLUTE = 1,
    /// One relative reminder.
    DESTACK_RUST_CALENDAR_REMINDER_KIND_RELATIVE = 2,
} DestackRustCalendarReminderKind;

/// One optional string reference passed through the Apple bridge.
typedef struct DestackRustOptionalStringRef {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped string reference.
    DestackRustStringRef value;
} DestackRustOptionalStringRef;

/// One optional `u32` passed through the Apple bridge.
typedef struct DestackRustOptionalU32 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    uint32_t value;
} DestackRustOptionalU32;

/// One optional `u64` passed through the Apple bridge.
typedef struct DestackRustOptionalU64 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    uint64_t value;
} DestackRustOptionalU64;

/// One optional `i8` passed through the Apple bridge.
typedef struct DestackRustOptionalI8 {
    /// Whether the optional field is present.
    bool has_value;
    /// The wrapped integer value.
    int8_t value;
} DestackRustOptionalI8;

/// One `u8` slice passed through the Apple bridge.
typedef struct DestackRustU8Slice {
    /// The slice data pointer.
    const uint8_t *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustU8Slice;

/// One `i8` slice passed through the Apple bridge.
typedef struct DestackRustI8Slice {
    /// The slice data pointer.
    const int8_t *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustI8Slice;

/// One `i16` slice passed through the Apple bridge.
typedef struct DestackRustI16Slice {
    /// The slice data pointer.
    const int16_t *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustI16Slice;

/// One recurrence weekday payload passed through the Apple bridge.
typedef struct DestackRustCalendarRecurrenceWeekday {
    /// The ISO-8601 weekday number.
    uint8_t day;
    /// The optional week number.
    DestackRustOptionalI8 week_number;
} DestackRustCalendarRecurrenceWeekday;

/// One recurrence weekday slice passed through the Apple bridge.
typedef struct DestackRustCalendarRecurrenceWeekdaySlice {
    /// The slice data pointer.
    const DestackRustCalendarRecurrenceWeekday *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustCalendarRecurrenceWeekdaySlice;

/// One recurrence rule payload passed through the Apple bridge.
typedef struct DestackRustCalendarRecurrenceRule {
    /// The recurrence frequency.
    DestackRustCalendarRecurrenceFrequency frequency;
    /// The recurrence interval.
    uint32_t interval;
    /// The optional occurrence count.
    DestackRustOptionalU32 count;
    /// The optional recurrence end timestamp.
    DestackRustOptionalU64 until_unix_ns;
    /// The weekday numbers.
    DestackRustU8Slice by_week_days;
    /// The structured weekday selectors.
    DestackRustCalendarRecurrenceWeekdaySlice by_weekday_ordinals;
    /// The day-of-month set.
    DestackRustI8Slice by_month_days;
    /// The month set.
    DestackRustU8Slice by_months;
    /// The day-of-year set.
    DestackRustI16Slice by_year_days;
    /// The week-of-year set.
    DestackRustI8Slice by_week_numbers;
    /// The set-position filters.
    DestackRustI16Slice by_set_positions;
} DestackRustCalendarRecurrenceRule;

/// One calendar attendee payload passed through the Apple bridge.
typedef struct DestackRustCalendarAttendee {
    /// The optional attendee identifier.
    DestackRustOptionalStringRef id;
    /// The optional attendee name.
    DestackRustOptionalStringRef name;
    /// The optional attendee email address.
    DestackRustOptionalStringRef email;
    /// Whether this attendee is optional.
    bool optional;
    /// Whether this attendee is the organizer.
    bool organizer;
    /// The attendee response status.
    DestackRustCalendarParticipantStatus response_status;
} DestackRustCalendarAttendee;

/// One calendar attendee slice passed through the Apple bridge.
typedef struct DestackRustCalendarAttendeeSlice {
    /// The slice data pointer.
    const DestackRustCalendarAttendee *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustCalendarAttendeeSlice;

/// One calendar reminder payload passed through the Apple bridge.
typedef struct DestackRustCalendarReminder {
    /// The reminder variant kind.
    DestackRustCalendarReminderKind kind;
    /// The absolute timestamp for one absolute reminder.
    uint64_t absolute_unix_ns;
    /// The minutes-before-start value for one relative reminder.
    int32_t minutes_before_start;
} DestackRustCalendarReminder;

/// One calendar reminder slice passed through the Apple bridge.
typedef struct DestackRustCalendarReminderSlice {
    /// The slice data pointer.
    const DestackRustCalendarReminder *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustCalendarReminderSlice;

/// One calendar descriptor passed through the Apple bridge.
typedef struct DestackRustCalendarDescriptor {
    /// The stable calendar identifier.
    DestackRustStringRef id;
    /// The calendar title.
    DestackRustStringRef title;
    /// The source or account label.
    DestackRustStringRef source;
    /// The optional owner label.
    DestackRustOptionalStringRef owner;
    /// The ARGB color value.
    uint32_t color_argb;
    /// Whether this is the primary write target.
    bool primary;
    /// The calendar access level.
    DestackRustCalendarAccess access;
} DestackRustCalendarDescriptor;

/// One calendar descriptor slice passed through the Apple bridge.
typedef struct DestackRustCalendarDescriptorSlice {
    /// The slice data pointer.
    const DestackRustCalendarDescriptor *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustCalendarDescriptorSlice;

/// One calendar event query passed through the Apple bridge.
typedef struct DestackRustCalendarQuery {
    /// The selected calendar identifiers.
    DestackRustStringSlice calendar_ids;
    /// The query start timestamp.
    uint64_t start_unix_ns;
    /// The query end timestamp.
    uint64_t end_unix_ns;
    /// The optional page limit.
    DestackRustOptionalU32 limit;
    /// Whether canceled events should be included.
    bool include_canceled;
    /// Whether declined events should be included.
    bool include_declined;
    /// Whether recurrence instances should be included.
    bool include_recurrence_instances;
} DestackRustCalendarQuery;

/// One calendar event payload passed through the Apple bridge.
typedef struct DestackRustCalendarEvent {
    /// The stable event identifier.
    DestackRustStringRef id;
    /// The calendar identifier.
    DestackRustStringRef calendar_id;
    /// The event title.
    DestackRustStringRef title;
    /// The optional notes.
    DestackRustOptionalStringRef notes;
    /// The optional location.
    DestackRustOptionalStringRef location;
    /// The start timestamp.
    uint64_t start_unix_ns;
    /// The end timestamp.
    uint64_t end_unix_ns;
    /// Whether this event is all-day.
    bool all_day;
    /// Whether this event is canceled.
    bool canceled;
    /// The optional timezone identifier.
    DestackRustOptionalStringRef time_zone;
    /// The availability class.
    DestackRustCalendarAvailability availability;
    /// The optional URL.
    DestackRustOptionalStringRef url;
    /// The optional organizer name.
    DestackRustOptionalStringRef organizer_name;
    /// The optional organizer email address.
    DestackRustOptionalStringRef organizer_email;
    /// Whether this event is recurring.
    bool recurring;
    /// The optional recurrence master identifier.
    DestackRustOptionalStringRef recurrence_master_id;
    /// The optional recurrence instance timestamp.
    DestackRustOptionalU64 recurrence_id_unix_ns;
    /// Whether the recurrence rule field is present.
    bool has_recurrence_rule;
    /// The recurrence rule payload.
    DestackRustCalendarRecurrenceRule recurrence_rule;
    /// Whether the attendees field is present.
    bool has_attendees;
    /// The attendee slice.
    DestackRustCalendarAttendeeSlice attendees;
    /// Whether the reminders field is present.
    bool has_reminders;
    /// The reminder slice.
    DestackRustCalendarReminderSlice reminders;
} DestackRustCalendarEvent;

/// One calendar event slice passed through the Apple bridge.
typedef struct DestackRustCalendarEventSlice {
    /// The slice data pointer.
    const DestackRustCalendarEvent *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustCalendarEventSlice;

/// One calendar event draft passed through the Apple bridge.
typedef struct DestackRustCalendarEventDraft {
    /// The calendar identifier.
    DestackRustStringRef calendar_id;
    /// The event title.
    DestackRustStringRef title;
    /// The optional notes.
    DestackRustOptionalStringRef notes;
    /// The optional location.
    DestackRustOptionalStringRef location;
    /// The start timestamp.
    uint64_t start_unix_ns;
    /// The end timestamp.
    uint64_t end_unix_ns;
    /// Whether this event is all-day.
    bool all_day;
    /// The optional timezone identifier.
    DestackRustOptionalStringRef time_zone;
    /// The availability class.
    DestackRustCalendarAvailability availability;
    /// The optional URL.
    DestackRustOptionalStringRef url;
    /// Whether the recurrence rule field is present.
    bool has_recurrence_rule;
    /// The recurrence rule payload.
    DestackRustCalendarRecurrenceRule recurrence_rule;
    /// Whether the attendees field is present.
    bool has_attendees;
    /// The attendee slice.
    DestackRustCalendarAttendeeSlice attendees;
    /// Whether the reminders field is present.
    bool has_reminders;
    /// The reminder slice.
    DestackRustCalendarReminderSlice reminders;
} DestackRustCalendarEventDraft;

/// The location accuracy preference passed through the Apple bridge.
typedef enum DestackRustLocationAccuracy {
    /// Passive updates with minimal power use.
    DESTACK_RUST_LOCATION_ACCURACY_PASSIVE = 1,
    /// Coarse accuracy.
    DESTACK_RUST_LOCATION_ACCURACY_LOW = 2,
    /// Balanced power and accuracy.
    DESTACK_RUST_LOCATION_ACCURACY_BALANCED = 3,
    /// Fine accuracy.
    DESTACK_RUST_LOCATION_ACCURACY_HIGH = 4,
    /// Best available accuracy.
    DESTACK_RUST_LOCATION_ACCURACY_BEST = 5
} DestackRustLocationAccuracy;

/// One location watch-options payload passed through the Apple bridge.
typedef struct DestackRustLocationWatchOptions {
    /// The requested accuracy preference.
    DestackRustLocationAccuracy accuracy;
    /// The minimum interval between updates in nanoseconds.
    uint64_t minimum_interval_ns;
    /// The minimum distance delta in meters.
    double minimum_distance_meters;
    /// Whether heading should be included when available.
    bool include_heading;
} DestackRustLocationWatchOptions;

/// One location sample payload passed through the Apple bridge.
typedef struct DestackRustLocationSample {
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
} DestackRustLocationSample;

/// The typed media-asset kind passed through the Apple bridge.
typedef enum DestackRustMediaAssetKind {
    /// One image asset.
    DESTACK_RUST_MEDIA_ASSET_KIND_IMAGE = 1,
    /// One video asset.
    DESTACK_RUST_MEDIA_ASSET_KIND_VIDEO = 2,
    /// One audio asset.
    DESTACK_RUST_MEDIA_ASSET_KIND_AUDIO = 3,
    /// One non-standard asset.
    DESTACK_RUST_MEDIA_ASSET_KIND_OTHER = 4
} DestackRustMediaAssetKind;

/// One typed media-kind slice passed through the Apple bridge.
typedef struct DestackRustMediaAssetKindSlice {
    /// The slice data pointer.
    const DestackRustMediaAssetKind *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustMediaAssetKindSlice;

/// One typed media query passed through the Apple bridge.
typedef struct DestackRustMediaQuery {
    /// Whether the query carries one cursor.
    bool has_cursor;
    /// The opaque cursor from one prior media-list call.
    DestackRustStringRef cursor;
    /// Whether the query carries one limit.
    bool has_limit;
    /// The maximum returned assets for this page.
    uint32_t limit;
    /// The included asset kinds, empty means all kinds.
    DestackRustMediaAssetKindSlice kinds;
    /// Whether hidden assets should be included.
    bool include_hidden;
} DestackRustMediaQuery;

/// One typed media descriptor passed through the Apple bridge.
typedef struct DestackRustMediaAssetDescriptor {
    /// The stable asset identifier.
    DestackRustStringRef id;
    /// The host URI for this asset.
    DestackRustStringRef uri;
    /// The asset filename payload.
    DestackRustStringRef filename;
    /// The asset MIME type payload when available.
    DestackRustStringRef mime_type;
    /// The asset class.
    DestackRustMediaAssetKind kind;
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
} DestackRustMediaAssetDescriptor;

/// One typed media-descriptor slice passed through the Apple bridge.
typedef struct DestackRustMediaAssetDescriptorSlice {
    /// The slice data pointer.
    const DestackRustMediaAssetDescriptor *data;
    /// The slice length in elements.
    uint32_t len;
} DestackRustMediaAssetDescriptorSlice;

/// One typed media page passed through the Apple bridge.
typedef struct DestackRustMediaPage {
    /// The returned assets for this page.
    DestackRustMediaAssetDescriptorSlice assets;
    /// Whether the page carries one next cursor.
    bool has_next_cursor;
    /// The opaque next-page cursor when available.
    DestackRustStringRef next_cursor;
    /// Whether more assets are available.
    bool has_more;
} DestackRustMediaPage;

/// One simplified notification request passed through the Apple bridge.
typedef struct DestackRustNotificationRequest {
    /// The stable runtime notification identifier.
    DestackRustStringRef identifier;
    /// The primary notification title.
    DestackRustStringRef title;
    /// The primary notification body text.
    DestackRustStringRef body;
} DestackRustNotificationRequest;

/// The simplified notification event kind passed through the Apple bridge.
typedef enum DestackRustNotificationEventKind {
    /// The notification was delivered.
    DESTACK_RUST_NOTIFICATION_EVENT_DELIVERED = 1,
    /// The notification was activated by the user.
    DESTACK_RUST_NOTIFICATION_EVENT_ACTIVATED = 2,
    /// The notification was dismissed.
    DESTACK_RUST_NOTIFICATION_EVENT_DISMISSED = 3
} DestackRustNotificationEventKind;

/// One simplified notification event passed through the Apple bridge.
typedef struct DestackRustNotificationEvent {
    /// The notification interaction kind.
    DestackRustNotificationEventKind kind;
    /// The event sequence number for this stream.
    uint64_t sequence;
    /// The monotonic event timestamp in nanoseconds.
    uint64_t timestamp_ns;
    /// The simplified request associated with this event.
    DestackRustNotificationRequest request;
    /// Whether the host provided one action identifier.
    bool has_action_identifier;
    /// The action identifier for interactive notifications when available.
    DestackRustStringRef action_identifier;
} DestackRustNotificationEvent;

/// The document callback type registered for one runtime session.
typedef uint32_t (*DestackRustDocumentCallback)(
    uint64_t session_handle,
    DestackRustDocumentRequest request
);

/// The permission request callback type registered for one runtime session.
typedef uint32_t (*DestackRustPermissionRequestCallback)(
    uint64_t session_handle,
    DestackRustPermissionRequest request
);

/// The permission-settings callback type registered for one runtime session.
typedef uint32_t (*DestackRustPermissionOpenSettingsCallback)(
    uint64_t session_handle
);

/// The contact-list callback type registered for one runtime session.
typedef uint32_t (*DestackRustContactListCallback)(
    uint64_t session_handle,
    DestackRustContactQuery query,
    DestackRustContactPage *output_page
);

/// The contact-search callback type registered for one runtime session.
typedef uint32_t (*DestackRustContactSearchCallback)(
    uint64_t session_handle,
    DestackRustStringRef query_text,
    DestackRustContactQuery query,
    DestackRustContactPage *output_page
);

/// The contact-read callback type registered for one runtime session.
typedef uint32_t (*DestackRustContactReadCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier,
    DestackRustContact *output_contact
);

/// The contact-create callback type registered for one runtime session.
typedef uint32_t (*DestackRustContactCreateCallback)(
    uint64_t session_handle,
    DestackRustContactDraft draft,
    DestackRustStringRef *output_identifier
);

/// The contact-update callback type registered for one runtime session.
typedef uint32_t (*DestackRustContactUpdateCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier,
    DestackRustContactDraft draft
);

/// The contact-delete callback type registered for one runtime session.
typedef uint32_t (*DestackRustContactDeleteCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier
);

/// The calendar-list callback type registered for one runtime session.
typedef uint32_t (*DestackRustCalendarListCallback)(
    uint64_t session_handle,
    DestackRustCalendarDescriptorSlice *output_calendars
);

/// The calendar event-list callback type registered for one runtime session.
typedef uint32_t (*DestackRustCalendarEventListCallback)(
    uint64_t session_handle,
    DestackRustCalendarQuery query,
    DestackRustCalendarEventSlice *output_events
);

/// The calendar event-read callback type registered for one runtime session.
typedef uint32_t (*DestackRustCalendarEventReadCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier,
    DestackRustCalendarEvent *output_event
);

/// The calendar event-create callback type registered for one runtime session.
typedef uint32_t (*DestackRustCalendarEventCreateCallback)(
    uint64_t session_handle,
    DestackRustCalendarEventDraft draft,
    DestackRustStringRef *output_identifier
);

/// The calendar event-update callback type registered for one runtime session.
typedef uint32_t (*DestackRustCalendarEventUpdateCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier,
    DestackRustCalendarEventDraft draft
);

/// The calendar event-delete callback type registered for one runtime session.
typedef uint32_t (*DestackRustCalendarEventDeleteCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier
);

/// The can-open-url callback type registered for one runtime session.
typedef uint32_t (*DestackRustIntentCanOpenUrlCallback)(
    uint64_t session_handle,
    DestackRustStringRef url,
    bool *is_supported
);

/// The open-url callback type registered for one runtime session.
typedef uint32_t (*DestackRustIntentOpenUrlCallback)(
    uint64_t session_handle,
    DestackRustStringRef url
);

/// The open-path callback type registered for one runtime session.
typedef uint32_t (*DestackRustIntentOpenPathCallback)(
    uint64_t session_handle,
    DestackRustStringRef path
);

/// The share-text callback type registered for one runtime session.
typedef uint32_t (*DestackRustIntentShareTextCallback)(
    uint64_t session_handle,
    DestackRustStringRef text,
    bool has_content_type,
    DestackRustStringRef content_type
);

/// The share-paths callback type registered for one runtime session.
typedef uint32_t (*DestackRustIntentSharePathsCallback)(
    uint64_t session_handle,
    DestackRustStringSlice paths,
    bool has_content_type,
    DestackRustStringRef content_type
);

/// The location-services-enabled callback type registered for one runtime session.
typedef uint32_t (*DestackRustLocationServicesEnabledCallback)(
    uint64_t session_handle,
    bool *is_enabled
);

/// The last-known-location callback type registered for one runtime session.
typedef uint32_t (*DestackRustLocationLastKnownCallback)(
    uint64_t session_handle,
    DestackRustLocationSample *sample
);

/// The location-watch-open callback type registered for one runtime session.
typedef uint32_t (*DestackRustLocationWatchOpenCallback)(
    uint64_t session_handle,
    DestackRustStringRef watch_id,
    DestackRustLocationWatchOptions options
);

/// The location-watch-close callback type registered for one runtime session.
typedef uint32_t (*DestackRustLocationWatchCloseCallback)(
    uint64_t session_handle,
    DestackRustStringRef watch_id
);

/// The media-list callback type registered for one runtime session.
typedef uint32_t (*DestackRustMediaListCallback)(
    uint64_t session_handle,
    DestackRustMediaQuery query,
    DestackRustMediaPage *output_page
);

/// The media-read callback type registered for one runtime session.
typedef uint32_t (*DestackRustMediaReadCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier,
    DestackRustMediaAssetDescriptor *output_descriptor
);

/// The media-import callback type registered for one runtime session.
typedef uint32_t (*DestackRustMediaImportPathCallback)(
    uint64_t session_handle,
    DestackRustStringRef path,
    int32_t kind,
    DestackRustStringRef *output_identifier
);

/// The media-delete callback type registered for one runtime session.
typedef uint32_t (*DestackRustMediaDeleteCallback)(
    uint64_t session_handle,
    DestackRustStringSlice identifiers,
    uint32_t *deleted_count
);

/// The notification post callback type registered for one runtime session.
typedef uint32_t (*DestackRustNotificationPostCallback)(
    uint64_t session_handle,
    DestackRustNotificationRequest request
);

/// The notification cancel callback type registered for one runtime session.
typedef uint32_t (*DestackRustNotificationCancelCallback)(
    uint64_t session_handle,
    DestackRustStringRef identifier
);

/// The notification cancel-all callback type registered for one runtime session.
typedef uint32_t (*DestackRustNotificationCancelAllCallback)(
    uint64_t session_handle
);

/// The document callbacks registered for one runtime session.
typedef struct IosHostDocumentCallbacks {
    DestackRustDocumentCallback pick;
} IosHostDocumentCallbacks;

/// The permission callbacks registered for one runtime session.
typedef struct IosHostPermissionCallbacks {
    DestackRustPermissionOpenSettingsCallback open_settings;
    DestackRustPermissionRequestCallback request;
} IosHostPermissionCallbacks;

/// The contact callbacks registered for one runtime session.
typedef struct IosHostContactCallbacks {
    DestackRustContactListCallback list;
    DestackRustContactSearchCallback search;
    DestackRustContactReadCallback read;
    DestackRustContactCreateCallback create;
    DestackRustContactUpdateCallback update;
    DestackRustContactDeleteCallback delete_contact;
} IosHostContactCallbacks;

/// The calendar callbacks registered for one runtime session.
typedef struct IosHostCalendarCallbacks {
    DestackRustCalendarListCallback list;
    DestackRustCalendarEventListCallback event_list;
    DestackRustCalendarEventReadCallback event_read;
    DestackRustCalendarEventCreateCallback event_create;
    DestackRustCalendarEventUpdateCallback event_update;
    DestackRustCalendarEventDeleteCallback event_delete;
} IosHostCalendarCallbacks;

/// The intent callbacks registered for one runtime session.
typedef struct IosHostIntentCallbacks {
    DestackRustIntentCanOpenUrlCallback can_open_url;
    DestackRustIntentOpenUrlCallback open_url;
    DestackRustIntentOpenPathCallback open_path;
    DestackRustIntentShareTextCallback share_text;
    DestackRustIntentSharePathsCallback share_paths;
} IosHostIntentCallbacks;

/// The location callbacks registered for one runtime session.
typedef struct IosHostLocationCallbacks {
    DestackRustLocationServicesEnabledCallback services_enabled;
    DestackRustLocationLastKnownCallback last_known;
    DestackRustLocationWatchOpenCallback watch_open;
    DestackRustLocationWatchCloseCallback watch_close;
} IosHostLocationCallbacks;

/// The media callbacks registered for one runtime session.
typedef struct IosHostMediaCallbacks {
    DestackRustMediaListCallback list;
    DestackRustMediaReadCallback read;
    DestackRustMediaImportPathCallback import_path;
    DestackRustMediaDeleteCallback delete_media;
} IosHostMediaCallbacks;

/// The notification callbacks registered for one runtime session.
typedef struct IosHostNotificationCallbacks {
    DestackRustNotificationCancelCallback cancel;
    DestackRustNotificationCancelAllCallback cancel_all;
    DestackRustNotificationPostCallback post;
} IosHostNotificationCallbacks;

/// The mobile runtime-bridge callback table for one runtime session.
typedef struct IosRuntimeBridgeBindings {
    IosHostDocumentCallbacks document;
    IosHostPermissionCallbacks permission;
    IosHostContactCallbacks contact;
    IosHostCalendarCallbacks calendar;
    IosHostIntentCallbacks intent;
    IosHostLocationCallbacks location;
    IosHostMediaCallbacks media;
    IosHostNotificationCallbacks notification;
} IosRuntimeBridgeBindings;

/// Register one mobile bridge callback table for one runtime session.
uint32_t destack_runtime_host_ios_register_runtime_bridge_bindings(
    uint64_t session_handle,
    IosRuntimeBridgeBindings callbacks
);

/// Unregister one mobile bridge callback table for one runtime session.
void destack_runtime_host_ios_unregister_runtime_bridge_bindings(
    uint64_t session_handle
);

/// Deliver one document result into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_document_result(
    uint64_t session_handle,
    uint64_t request_id,
    DestackRustDocumentDescriptorSlice documents
);

/// Deliver one notification event into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_notification_event(
    uint64_t session_handle,
    DestackRustNotificationEvent event
);

/// Deliver one intent open-url event into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_open_url(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef url
);

/// Deliver one intent open-file event into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_open_file(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef path,
    bool has_content_type,
    DestackRustStringRef content_type
);

/// Deliver one intent share-text event into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_share_text(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef text,
    bool has_content_type,
    DestackRustStringRef content_type
);

/// Deliver one intent share-files event into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_share_files(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringSlice paths,
    bool has_content_type,
    DestackRustStringRef content_type
);

/// Deliver one intent custom-action event into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_custom_action(
    uint64_t session_handle,
    bool has_source,
    DestackRustStringRef source,
    DestackRustStringRef action,
    bool has_url,
    DestackRustStringRef url,
    DestackRustStringSlice paths,
    bool has_text,
    DestackRustStringRef text,
    bool has_content_type,
    DestackRustStringRef content_type
);

/// Deliver one location sample into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_location_sample(
    uint64_t session_handle,
    DestackRustStringRef watch_id,
    DestackRustLocationSample sample
);

/// Deliver one permission result into one runtime session.
DestackRustRuntimeStatus destack_runtime_host_ios_notify_permission_result(
    uint64_t session_handle,
    bool has_request_id,
    uint64_t request_id,
    const uint8_t *permission,
    uint32_t permission_len,
    bool granted
);

#endif
