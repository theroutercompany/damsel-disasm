This directory contains the small fixture corpus used by the loader, CLI, and
benchmark tests.

- `src/hello.c` builds the symbolized, stripped, and universal C fixtures.
- `src/objc-sample.m` builds the Objective-C metadata fixture.
- `build-fixtures.sh` regenerates the binaries on macOS with Xcode installed.

The checked-in binaries are meant for static parsing only. They are not invoked
by the test suite.
