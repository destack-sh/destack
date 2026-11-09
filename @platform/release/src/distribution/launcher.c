#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#if defined(__APPLE__)
#include <mach-o/dyld.h>
#endif

/** The arguments a program starts with before its own: the runtime, its four and the bundle. */
#define LAUNCH_ARGUMENTS 5

/** Find this launcher's real path, following the links a command line or a menu starts it through. */
static int resolve_self(char *resolved) {
    char executable[PATH_MAX];
#if defined(__APPLE__)
    uint32_t size = sizeof(executable);
    if (_NSGetExecutablePath(executable, &size) != 0) {
        return -1;
    }
#else
    ssize_t length = readlink("/proc/self/exe", executable, sizeof(executable) - 1);
    if (length < 0) {
        return -1;
    }
    executable[length] = '\0';
#endif
    return realpath(executable, resolved) == NULL ? -1 : 0;
}

/** Start the program this launcher is named after: the distribution's runtime running its bundle. */
int main(int argc, char **argv) {
    // find this launcher's directory and name, which name the program
    char self[PATH_MAX];
    if (resolve_self(self) != 0) {
        perror("cannot locate Destack");
        return 1;
    }
    char *slash = strrchr(self, '/');
    if (slash == NULL) {
        fputs("cannot locate the Destack directory\n", stderr);
        return 1;
    }
    *slash = '\0';
    const char *directory = self;
    const char *name = slash + 1;

    // read the resources from the macOS application's Resources, else beside the launcher
    char resources[PATH_MAX];
    const char *contents = strstr(directory, ".app/Contents/");
    int written = contents != NULL
        ? snprintf(resources, sizeof(resources), "%.*s.app/Contents/Resources", (int)(contents - directory), directory)
        : snprintf(resources, sizeof(resources), "%s", directory);
    if (written < 0 || (size_t)written >= sizeof(resources)) {
        fputs("the Destack path is too long\n", stderr);
        return 1;
    }

    // point at the runtime beside the launcher, the configuration and the program's bundle
    char runtime[PATH_MAX];
    char configuration[PATH_MAX];
    char bundle[PATH_MAX];
    int runtime_length = snprintf(runtime, sizeof(runtime), "%s/bun", directory);
    int configuration_length = snprintf(configuration, sizeof(configuration), "--config=%s/bunfig.toml", resources);
    int bundle_length = snprintf(bundle, sizeof(bundle), "%s/program/%s.js", resources, name);
    if (runtime_length < 0 || (size_t)runtime_length >= sizeof(runtime) ||
        configuration_length < 0 || (size_t)configuration_length >= sizeof(configuration) ||
        bundle_length < 0 || (size_t)bundle_length >= sizeof(bundle)) {
        fputs("the Destack path is too long\n", stderr);
        return 1;
    }

    // run the bundle on the runtime, ignoring a project's .env and Bun configuration, passing the arguments on
    char **arguments = calloc((size_t)argc + LAUNCH_ARGUMENTS, sizeof(char *));
    if (arguments == NULL) {
        perror("cannot start Destack");
        return 1;
    }
    arguments[0] = runtime;
    arguments[1] = "run";
    arguments[2] = "--no-env-file";
    arguments[3] = configuration;
    arguments[4] = bundle;
    for (int index = 1; index < argc; index += 1) {
        arguments[LAUNCH_ARGUMENTS + index - 1] = argv[index];
    }
    arguments[argc + LAUNCH_ARGUMENTS - 1] = NULL;
    execv(runtime, arguments);
    perror("cannot start Destack");
    return 1;
}
