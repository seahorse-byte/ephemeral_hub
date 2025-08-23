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

# Safety guard: ensure the build directory looks like a valid web build.
# If index.html is missing, refuse to proceed to avoid wiping the gh-pages branch.
if [ ! -f "$BUILD_DIR/index.html" ]; then
  echo "Refusing to deploy: $BUILD_DIR does not contain index.html. Aborting to avoid wiping gh-pages." >&2
  echo "If you really want to publish an empty site, set FORCE_DEPLOY=1 in the environment." >&2
  if [ "${FORCE_DEPLOY:-0}" != "1" ]; then
    exit 1
  fi
fi

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

# Fix deployed index.html asset paths so they are relative (avoid leading slash)
# This converts occurrences like "/./assets/..." to "./assets/..." so the site
# works when published under a repo subpath (e.g. https://user.github.io/repo/).
if [ -f "$WORKTREE_TMPDIR/index.html" ]; then
  echo "Rewriting index.html asset paths to be relative"
  # Replace any '/./assets' with './assets'
  perl -0777 -pe 's{/\./assets}{./assets}g' -i.bak "$WORKTREE_TMPDIR/index.html" || true
  rm -f "$WORKTREE_TMPDIR/index.html.bak" || true
fi

# Inject a small script to normalize the pathname when hosting under a repo subpath
# This strips the repo base (default 'ephemeral_spaces') from the path so the
# Dioxus router sees routes like '/' or '/s/:id' instead of '/ephemeral_spaces/'.
REPO_BASENAME=${REPO_BASENAME:-ephemeral_spaces}
if [ -f "$WORKTREE_TMPDIR/index.html" ]; then
  echo "Injecting repo-base normalization script into index.html (repo basename: $REPO_BASENAME)"
  SCRIPT=$(cat <<EOF
<script>
  (function(){
    try {
      var repo = '$REPO_BASENAME';
      if (location.pathname.indexOf('/' + repo) === 0) {
        var newPath = location.pathname.replace('/' + repo, '') || '/';
        history.replaceState({}, document.title, newPath + location.search + location.hash);
      }
    } catch(e) {}
  })();
</script>
EOF
)

  # Insert the SCRIPT into the <head> so it runs before any scripts/imports
  awk -v ins="$SCRIPT" 'BEGIN{added=0} /<head[^>]*>/ && !added {print; print ins; added=1; next} {print}' "$WORKTREE_TMPDIR/index.html" > "$WORKTREE_TMPDIR/index.html.tmp" && mv "$WORKTREE_TMPDIR/index.html.tmp" "$WORKTREE_TMPDIR/index.html"
fi

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
