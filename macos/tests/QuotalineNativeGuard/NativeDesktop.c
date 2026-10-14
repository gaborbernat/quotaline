#include "include/NativeDesktop.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// A cached native test executable must refuse local --skip-build runs too.
__attribute__((constructor)) void requireNativeTestDesktop(void) {
  const char *desktop = getenv("QUOTALINE_TEST_DESKTOP");
#if QUOTALINE_APPLICATION_UI_TESTS
  if (desktop && strcmp(desktop, "local-verification") == 0) {
    return;
  }
#endif
  const char *actions = getenv("GITHUB_ACTIONS");
  const char *runner = getenv("RUNNER_ENVIRONMENT");
  if (!desktop || strcmp(desktop, "github-hosted") || !actions || strcmp(actions, "true") ||
      !runner || strcmp(runner, "github-hosted")) {
    fputs("Native AppKit tests require an isolated GitHub-hosted desktop. Run just macos-test locally.\n", stderr);
    exit(EXIT_FAILURE);
  }
}
