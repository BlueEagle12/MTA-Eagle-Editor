# Releasing Eagle Editor

This document is the release checklist and formatting standard for Eagle
Editor. It is based on the published `v0.1.0` through `v0.1.3` GitHub
releases.

## Release identity

- Use a semantic-version Git tag: `vX.Y.Z`.
- Use the same version without the leading `v` in `Cargo.toml`:
  `X.Y.Z`.
- Title the GitHub release `vX.Y.Z`. The original `Eagle Editor v0.1.0` title
  is a historical exception; releases from `v0.1.1` onward use the tag as the
  title.
- Publish normal releases as public, non-draft, and non-prerelease releases.

## Required release assets

Every release ships these three files with exact versioned names:

```text
EagleEditor-vX.Y.Z-linux-x86_64.tar.gz
EagleEditor-vX.Y.Z-windows-x86_64.zip
SHA256SUMS.txt
```

The checksum file must include both archives and be verified from its own
directory with:

```bash
sha256sum -c SHA256SUMS.txt
```

Do not attach the unpacked release folders to GitHub; attach only the two
archives and their checksum manifest.

## Release notes

Use a short `## What's Changed` section with reader-facing bullets. Lead with
features and fixes that change editor or in-game behavior, then end with the
full comparison link:

```md
## What's Changed

- Added …
- Improved …
- Fixed …

**Full Changelog**: https://github.com/BlueEagle12/MTA-Eagle-Editor/compare/vPREVIOUS...vX.Y.Z
```

Keep bullets concrete: say what users can now do and what observable issue was
fixed. Mention platform or packaging changes when they affect installation.
Use GitHub-generated notes when no curated notes are needed; the release
workflow already generates this format for pushed tags.

## Checklist

1. Update `Cargo.toml`, `Cargo.lock`, README version references, and release
   notes for `X.Y.Z`.
2. Run the focused tests for the changed feature, then build with
   `cargo build --release --locked`.
3. Commit the release preparation, push it to `main`, and create the annotated
   `vX.Y.Z` tag on that release commit.
4. Build both portable packages locally with:

   ```bash
   ./scripts/build_releases.sh
   ```

   The output directory is `release/github-upload/`.
5. Confirm the archives contain the executable, runtime assets, notices, and
   README, then verify `SHA256SUMS.txt` from `release/github-upload/`.
6. Publish the GitHub release from the tag and attach exactly the three
   required assets. The tag-push workflow can build and publish automatically;
   a manually published release must use the same names, notes format, and
   assets.
7. Open the published release page and confirm its tag, title, target commit,
   notes, and all three downloadable assets.

## Automation

`.github/workflows/release-builds.yml` runs for `v*` tag pushes. It builds the
Linux and Windows packages, bundles the checksums, and publishes a GitHub
release with `--generate-notes --verify-tag`. Keep the asset names produced by
`scripts/package_release.py` stable so local packages and CI releases remain
interchangeable.
