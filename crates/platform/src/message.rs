//! The codes of this crate's messages (see `reroute_core::message`).

reroute_core::message_codes! {
    PLATFORM_TOOL = "platform.tool" (tool, detail);
    PLATFORM_OS = "platform.os" (detail);
    PLATFORM_UNSUPPORTED = "platform.unsupported";
    PLATFORM_ICON = "platform.icon" (detail);

    REGISTER_CONFIRM_MACOS = "register.confirm_macos";
    REGISTER_CONFIRM_WINDOWS = "register.confirm_windows";

    RELEASE_NOT_FOUND = "release.not_found";
    RELEASE_STATUS = "release.status" (status);
    RELEASE_TIMEOUT = "release.timeout";
    RELEASE_UNREACHABLE = "release.unreachable" (detail);
    RELEASE_UNREADABLE = "release.unreadable" (detail);
    RELEASE_NO_VERSION = "release.no_version";
}
