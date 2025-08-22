#!/usr/bin/env bash
set -euo pipefail

# Deploy the Dioxus build output to the gh-pages branch using a temporary worktree.
# Usage:
#   BUILD_DIR=target/dx/ephemeral_web/release/web/public ./scripts/deploy-ghpages.sh
# or just:
#   ./scripts/deploy-ghpages.sh

BUILD_DIR=${BUILD_DIR:-target/dx/ephemeral_web/release/web/public}
TMPDIR=${TMPDIR:-.gh-pages-temp}
BRANCH=${BRANCH:-gh-pages}
REMOTE=${REMOTE:-origin}

echo "Build dir: $BUILD_DIR"
[ -d "$BUILD_DIR" ] || { echo "Build folder not found: $BUILD_DIR" >&2; exit 1; }

# Ensure repo up-to-date
git fetch "$REMOTE"

# Clean any stale temp worktree metadata and directory
echo "Pruning stale worktrees (if any)"
git worktree prune || true

# If a worktree record still references the TMPDIR, remove it first.
if git worktree list --porcelain | grep -F "worktree $TMPDIR" >/dev/null 2>&1; then
  echo "Found existing worktree record for $TMPDIR, removing it"
  git worktree remove "$TMPDIR" --force || true
fi

if [ -d "$TMPDIR" ]; then
  echo "Removing existing temp worktree directory $TMPDIR"
  rm -rf "$TMPDIR"
fi

# Create or reset gh-pages worktree. If remote branch exists, base on it; otherwise create orphan.
if git ls-remote --exit-code --heads "$REMOTE" "$BRANCH" >/dev/null 2>&1; then
  echo "Creating worktree from remote branch $REMOTE/$BRANCH"
  git worktree add -B "$BRANCH" "$TMPDIR" "$REMOTE/$BRANCH"
else
  echo "Creating new local worktree branch $BRANCH"
  git worktree add -B "$BRANCH" "$TMPDIR"
fi

# Sync files
echo "Syncing files to worktree..."
rsync -av --delete "$BUILD_DIR/" "$TMPDIR/"

# Commit & push if there are changes
cd "$TMPDIR"
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
git worktree remove "$TMPDIR" --force || rm -rf "$TMPDIR"

echo "Deploy complete."
