# Security policy

## Reporting a vulnerability

Please report vulnerabilities **privately**, through
[GitHub's private vulnerability reporting](https://github.com/avanserv/spacify/security/advisories/new).
Do not open a public issue for one.

Say what you found, how to reproduce it, and what it lets someone do. You will get an
acknowledgement, and the fix and its disclosure are coordinated with you through the advisory.

## Supported versions

The project is pre-1.0: only the latest release receives fixes.

## What is in scope

spacify rewrites the files it is given and nothing else. It makes no network requests and runs no
code from the files it reads. A behaviour that breaks one of these is a vulnerability:

- A file outside the paths it was given, or outside the walk of a directory it was given, being
  written.
- A change to a file other than adding or removing blank lines between statements.
