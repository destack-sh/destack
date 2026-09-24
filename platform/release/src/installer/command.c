#include <mach-o/dyld.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#ifndef DESTACK_COMMAND
#define DESTACK_COMMAND "destack"
#endif

/** Run the native CLI without modifying its embedded archive. */
int main(int argc, char **argv) {
    (void)argc;

    // resolve the bundle location even when invoked through the registered symlink
    char executable[PATH_MAX];
    char resolved[PATH_MAX];
    uint32_t size = sizeof(executable);
    if (_NSGetExecutablePath(executable, &size) != 0 || realpath(executable, resolved) == NULL) {
        perror("cannot locate Destack");
        return 1;
    }
    char *name = strrchr(resolved, '/');
    if (name == NULL) {
        fputs("cannot locate the executable directory\n", stderr);
        return 1;
    }
    *name = '\0';

    // retain separate compiled payloads while the operating system selects this launcher's slice
#if defined(__arm64__)
    const char *architecture = "arm64";
#else
    const char *architecture = "x86_64";
#endif
    int length = snprintf(executable, sizeof(executable), "%s/%s/%s", resolved, architecture, DESTACK_COMMAND);
    if (length < 0 || (size_t)length >= sizeof(executable)) {
        fputs("destack path is too long\n", stderr);
        return 1;
    }
    argv[0] = executable;
    execv(executable, argv);
    perror("cannot start Destack");
    return 1;
}
