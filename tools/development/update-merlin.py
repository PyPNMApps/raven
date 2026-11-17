#!/usr/bin/env python3
#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Select a stable MERLIN release and refresh its pinned source and legal files."""
import argparse
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
REMOTE = 'https://github.com/mgarcia01752/merlin.git'
LEGAL_FILES = {
    'LICENSE': 'LICENSE',
    'NOTICE': 'NOTICE',
    'crates/fec-codec/LICENSE-MIT': 'AFF3CT-LICENSE-MIT.txt',
    'crates/ofdm/resources/qam-table/LICENSE': 'QAM-LICENSE.txt',
    'crates/ofdm/resources/qam-table/NOTICE': 'QAM-NOTICE.txt',
}

# Validate a whole stable tag such as v0.4.0; three numeric groups sort releases.
# Pre-release suffixes, whitespace and leading-zero versions are excluded.
STABLE_TAG = re.compile(r'v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)')


def select_tag(listing, requested):
    """Resolve an explicit stable tag or the highest stable semantic version.

    Accept Git ls-remote output and a tag or 'latest'. Return tag and peeled
    commit; reject missing tags or a repository without stable releases.
    """
    references = dict(line.split()[::-1] for line in listing.splitlines() if line.strip())
    versions = {}
    for reference in references:
        tag = reference.removeprefix('refs/tags/')
        match = STABLE_TAG.fullmatch(tag)
        if match:
            versions[tag] = tuple(int(part) for part in match.groups())
    if not versions:
        raise ValueError('MERLIN has no stable vMAJOR.MINOR.PATCH tags')
    tag = max(versions, key=versions.get) if requested == 'latest' else requested
    if tag not in versions:
        raise ValueError(f'Stable MERLIN tag not found: {tag}')
    reference = f'refs/tags/{tag}'
    return tag, references.get(reference + '^{}', references[reference])


def update_dependency(manifest, revision):
    """Replace only MERLIN's Git selector, preserving unrelated TOML bytes.

    Return updated text; fail if the expected single dependency is absent or
    has multiple selectors. The selected release is pinned by immutable commit.
    """
    lines = manifest.splitlines(keepends=True)
    matches = [index for index, line in enumerate(lines) if line.startswith('merlin = {')]
    if len(matches) != 1:
        raise ValueError('Expected one inline merlin dependency')
    index = matches[0]
    if f'git = "{REMOTE}"' not in lines[index]:
        raise ValueError('Unexpected MERLIN repository')

    # Replace one quoted selector, e.g. rev = "abc", within the dependency line.
    # Group one is the selector name; values cannot contain quotes or newlines.
    selector = re.compile(r'\b(rev|tag|branch) = "[^"\n]+"')
    lines[index], count = selector.subn(f'rev = "{revision}"', lines[index])
    if count != 1:
        raise ValueError('Expected one MERLIN Git selector')
    return ''.join(lines)


def run(arguments, **kwargs):
    """Run an argument array and return stdout, propagating command failures."""
    return subprocess.run(arguments, check=True, stdout=subprocess.PIPE, **kwargs).stdout


def apply_update(root, tag, revision, legal):
    """Update the pin, runtime revision, lockfile and legal snapshots together.

    Restore original files if Cargo resolution fails. Cargo's download cache is
    retained. Callers must not run this concurrently with other dependency edits.
    """
    manifest = root / 'apps/viewer/Cargo.toml'
    runtime = root / 'apps/viewer/src/simulation/fec/mod.rs'
    runtime_text = runtime.read_text()
    marker = 'pub const MERLIN_REVISION: &str = "'
    if runtime_text.count(marker) != 1:
        raise ValueError('Expected one adapter revision constant')
    prefix, remainder = runtime_text.split(marker)
    _, suffix = remainder.split('";', 1)
    changes = {
        manifest: update_dependency(manifest.read_text(), revision).encode(),
        runtime: (prefix + marker + revision + '";' + suffix).encode(),
    }
    legal_root = root / 'LICENSES/merlin'
    changes.update({legal_root / name: content for name, content in legal.items()})
    changes[legal_root / 'SOURCE.txt'] = f'{REMOTE}\nTag: {tag}\nCommit: {revision}\n'.encode()
    backups = {path: path.read_bytes() if path.exists() else None
               for path in [*changes, root / 'Cargo.lock']}
    try:
        for path, content in changes.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        subprocess.run(['bash', str(root / 'scripts/cargo-local.sh'), 'update',
                        '--manifest-path', str(root / 'Cargo.toml'),
                        '-p', 'docsis-ofdm-fec', '--precise', revision], check=True)
    except BaseException:
        for path, content in backups.items():
            if content is None:
                path.unlink(missing_ok=True)
            else:
                path.write_bytes(content)
        raise


def main():
    """Parse release selection, verify remote evidence and apply a local update.

    Latest means highest stable semantic tag, not commit date or a prerelease.
    Dry run performs remote reads only. Never commit, push, build or run tests.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tag', nargs='?', default='latest', help='latest or vMAJOR.MINOR.PATCH')
    parser.add_argument('--dry-run', action='store_true', help='show selection without modifying files')
    arguments = parser.parse_args()
    try:
        listing = run(['git', 'ls-remote', '--tags', REMOTE], text=True)
        tag, revision = select_tag(listing, arguments.tag)
        print(f'MERLIN {tag}: {revision}', flush=True)
        if arguments.dry_run:
            return
        with tempfile.TemporaryDirectory(prefix='raven-merlin-') as directory:
            run(['git', 'init', '--bare', '--quiet', directory])
            run(['git', '-C', directory, 'fetch', '--quiet', '--depth=1', REMOTE,
                 f'refs/tags/{tag}'])
            fetched = run(['git', '-C', directory, 'rev-parse', 'FETCH_HEAD^{commit}'], text=True).strip()
            if fetched != revision:
                raise ValueError('Remote tag moved during update; retry after review')
            legal = {destination: run(['git', '-C', directory, 'show', f'{revision}:{source}'])
                     for source, destination in LEGAL_FILES.items()}
        apply_update(ROOT, tag, revision, legal)
        print('Updated pin, Cargo.lock, adapter revision and MERLIN notices.')
        print('Review the diff and run feature-enabled tests before committing.')
    except (ValueError, OSError, subprocess.CalledProcessError) as failure:
        parser.exit(1, f'MERLIN update failed: {failure}\n')


if __name__ == '__main__':
    main()
