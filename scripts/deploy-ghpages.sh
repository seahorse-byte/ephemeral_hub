#!/usr/bin/env bash
set -euo pipefail

# Deploy the Dioxus build output to the gh-pages branch using a temporary worktree.
# Usage:
#   BUILD_DIR=target/dx/ephemeral_web/release/web/public ./scripts/deploy-ghpages.sh
# or just:
#   ./scripts/deploy-ghpages.sh

BUILD_DIR=${BUILD_DIR:-target/dx/ephemeral_web/release/web/public}
# Use a repo-local temp dir name to avoid clobbering the system TMPDIR on macOS
# Allow overriding with WORKTREE_TMPDIR, but default to a relative path inside repo
WORKTREE_TMPDIR=${WORKTREE_TMPDIR:-.gh-pages-temp}
BRANCH=${BRANCH:-gh-pages}
REMOTE=${REMOTE:-origin}

echo "Build dir: $BUILD_DIR"
[ -d "$BUILD_DIR" ] || { echo "Build folder not found: $BUILD_DIR" >&2; exit 1; }

# Ensure repo up-to-date
git fetch "$REMOTE"

# Clean any stale temp worktree metadata and directory
echo "Pruning stale worktrees (if any)"
git worktree prune || true

# Sanity: ensure the chosen temp dir is inside the repository root. This prevents
# accidental destructive rm -rf on system tmp (macOS sets TMPDIR) or other folders.
REPO_ROOT=$(git rev-parse --show-toplevel)
# Resolve a safe absolute path for the worktree temp dir (works for relative paths)
WORKTREE_TMPDIR_ABS=$(cd "$(dirname "$WORKTREE_TMPDIR")" >/dev/null 2>&1 && pwd)/$(basename "$WORKTREE_TMPDIR")
if [[ "$WORKTREE_TMPDIR_ABS" != "$REPO_ROOT"* ]]; then
  echo "Refusing to operate on worktree temp dir outside the repo: $WORKTREE_TMPDIR_ABS" >&2
  echo "Set WORKTREE_TMPDIR to a path inside the repository (or leave it unset to use .gh-pages-temp)." >&2
  exit 1
fi

# If a worktree record still references the temp dir, remove it first.
if git worktree list --porcelain | grep -F "worktree $WORKTREE_TMPDIR" >/dev/null 2>&1; then
  echo "Found existing worktree record for $WORKTREE_TMPDIR, removing it"
  git worktree remove "$WORKTREE_TMPDIR" --force || true
fi

if [ -d "$WORKTREE_TMPDIR" ]; then
  echo "Removing existing temp worktree directory $WORKTREE_TMPDIR"
  rm -rf "$WORKTREE_TMPDIR"
fi

# Create or reset gh-pages worktree. If remote branch exists, base on it; otherwise create orphan.
if git ls-remote --exit-code --heads "$REMOTE" "$BRANCH" >/dev/null 2>&1; then
  echo "Creating worktree from remote branch $REMOTE/$BRANCH"
  git worktree add -B "$BRANCH" "$WORKTREE_TMPDIR" "$REMOTE/$BRANCH"
else
  echo "Creating new local worktree branch $BRANCH"
  git worktree add -B "$BRANCH" "$WORKTREE_TMPDIR"
fi

# Sync files
echo "Syncing files to worktree (preserving .git)..."
# Exclude .git so we don't accidentally remove the worktree metadata and break cleanup
rsync -av --delete --exclude='.git' "$BUILD_DIR/" "$WORKTREE_TMPDIR/"

# Commit & push if there are changes
cd "$WORKTREE_TMPDIR"
if [ -n "$(git status --porcelain)" ]; then
  git add --all
  git commit -m "Deploy site: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
  git push "$REMOTE" "$BRANCH"
  echo "Pushed updates to $REMOTE/$BRANCH"
else
  echo "No changes to deploy"
fi

# Cleanup worktree
cd - >/dev/null
echo "Removing worktree $TMPDIR"
echo "Removing worktree $WORKTREE_TMPDIR"
git worktree remove "$WORKTREE_TMPDIR" --force || rm -rf "$WORKTREE_TMPDIR"

echo "Deploy complete."
