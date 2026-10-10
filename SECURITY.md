# Security policy

## Reporting a vulnerability

Please don't open a public issue for a security problem. Report it privately instead:
on the [Security tab](https://github.com/nickmorell/bifrost/security), choose **Report a vulnerability**.
Only the maintainer can see the report.

Useful things to include: the BiFrost version and operating system, what an attacker could do, and the steps
to reproduce it.

## What happens next

BiFrost is maintained by one person in their spare time, so this is best effort:

- You get a reply, usually within a week.
- A confirmed problem is fixed in the next patch release (for example 0.1.1), ahead of anything else.
- The fix is published with a GitHub security advisory, crediting you unless you'd rather not be named.

There is no bug bounty.

## Supported versions

Only the latest release gets security fixes. Please update before reporting.

## What counts

Anything that puts a BiFrost user's account, tokens, files or privacy at risk, for example:

- sign-in tokens or API keys leaving the OS credential store, or being written to disk or logs;
- BiFrost contacting a service whose integration is switched off;
- a post, message, link, image or video that makes BiFrost crash, hang, run code or open something without asking;
- a problem in the release downloads or the workflows that build them.

Problems in Bluesky itself or another AT Protocol service belong with that service.

## Checking a download

Every file on the [Releases page](https://github.com/nickmorell/bifrost/releases) comes with a signed build
provenance attestation. With the [GitHub CLI](https://cli.github.com) you can check that a download was built
by this repository's release workflow:

```sh
gh attestation verify bifrost-v0.1.1-windows-x86_64.zip --repo nickmorell/bifrost
```

`SHA256SUMS.txt` on each release lists the checksum of every file.
