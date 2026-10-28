# Security policy

## Supported versions

Only the latest release of `klipsch-lumina` and `klipsch-lumina-cli`
receives fixes.

## Reporting a vulnerability

Please report vulnerabilities privately through
[GitHub's private reporting](https://github.com/wlof/lumina/security/advisories/new)
(Security tab → *Report a vulnerability*), not in a public issue. Include
the version, your platform and the steps to reproduce.

## Scope

In scope, for example:

- the library or the CLI panicking, hanging or misbehaving on data read
  from the speakers or the USB bus;
- the udev rule granting more access than the speakers' interfaces;
- the release pipeline, archives and installers.

Out of scope: `lumina debug send` sends any command the user gives it,
including ones that put the speakers in standby or reset them. That is its
purpose.

## Verifying a release

Release archives carry a build provenance attestation from GitHub Actions:

```sh
gh attestation verify <archive> --repo wlof/lumina
```
