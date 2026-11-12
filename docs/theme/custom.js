// kuva docs — sidebar category collapse.
//
// mdBook's built-in expand/collapse (the `expanded` class toggled by
// toc.js) only applies to *nested* chapters, i.e. a chapter with its own
// `<ol class="section">` of sub-pages (like "kuva CLI" and its 56
// subcommands). The `# Category` part-titles introduced by the SUMMARY.md
// reorg are flat siblings in the same list with no nesting to hook into, so
// there's nothing built-in to toggle. This adds that behaviour: each
// part-title becomes a clickable header that shows/hides the chapter-items
// between it and the next part-title.
//
// Categories start collapsed, except the one containing the current page
// (so navigating to a plot page doesn't hide it), and previously-opened
// categories are remembered across page loads via localStorage so browsing
// several pages in the same category doesn't require re-opening it each time.
(function () {
  const STORAGE_KEY = "kuva-sidebar-open-categories";

  function initCategoryCollapse() {
    const sidebar = document.querySelector("mdbook-sidebar-scrollbox");
    if (!sidebar) {
      return;
    }
    const chapterList = sidebar.querySelector("ol.chapter");
    if (!chapterList || chapterList.dataset.kuvaInit === "true") {
      return;
    }
    chapterList.dataset.kuvaInit = "true";

    const groups = [];
    let currentGroup = null;

    Array.from(chapterList.children).forEach((li) => {
      if (li.classList.contains("part-title")) {
        currentGroup = { title: li.textContent.trim(), titleLi: li, items: [] };
        groups.push(currentGroup);
        li.classList.add("kuva-toggle");
      } else if (
        currentGroup &&
        li.classList.contains("chapter-item") &&
        li.querySelector("a")
      ) {
        currentGroup.items.push(li);
      }
    });

    let openState = {};
    try {
      openState = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
    } catch (e) {
      openState = {};
    }

    function setGroupOpen(group, open, persist) {
      group.items.forEach((li) => li.classList.toggle("kuva-collapsed", !open));
      group.titleLi.classList.toggle("kuva-open", open);
      if (persist) {
        openState[group.title] = open;
        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(openState));
        } catch (e) {
          // localStorage unavailable (private browsing, etc.) — collapse
          // state just won't persist across page loads, which is fine.
        }
      }
    }

    groups.forEach((group) => {
      const hasActivePage = group.items.some((li) => li.querySelector("a.active"));
      const open = hasActivePage || openState[group.title] === true;
      setGroupOpen(group, open, false);

      group.titleLi.addEventListener("click", () => {
        setGroupOpen(group, !group.titleLi.classList.contains("kuva-open"), true);
      });
    });
  }

  // ── Search UI ─────────────────────────────────────────────────────────
  //
  // There are two presentations of the same single search input/index (you
  // can't have two independent copies — searcher.js's behavior is entirely
  // ID-based, so there's exactly one #mdbook-search-wrapper element to go
  // around):
  //
  //  1. A compact, always-visible box at the top of the sidebar, above
  //     "Introduction" — fast, convenient, but cramped for reading result
  //     teasers.
  //  2. A dedicated full-width `search.html` page (see docs/src/search.md's
  //     `#kuva-fullpage-search-slot`) for reading through result stubs
  //     properly. The top-bar search icon now navigates here instead of
  //     just focusing the sidebar box.
  //
  // On every page except search.html, the real input lives in the sidebar.
  // On search.html itself, it's moved into the page's own content slot
  // instead and opened immediately, since the whole point of that page is
  // to search.
  //
  // The search index is loaded lazily by mdBook itself (a private closure
  // in searcher.js we can't call directly), on the searchbar's first
  // `focus` rather than unconditionally on page load, by simulating a click
  // on the toggle icon — that runs mdBook's real init sequence exactly once.
  // Triggering it unconditionally on every sidebar-page load was considered
  // and rejected: mdBook's own `init()` unconditionally calls
  // `searchbar.focus()` as part of that sequence, which would silently
  // steal keyboard focus into the sidebar on every single page load.
  function initSearchUI() {
    const wrapper = document.getElementById("mdbook-search-wrapper");
    const searchbar = document.getElementById("mdbook-searchbar");
    const toggleIcon = document.getElementById("mdbook-search-toggle");
    if (!wrapper || !searchbar || !toggleIcon) {
      return;
    }

    const fullPageSlot = document.getElementById("kuva-fullpage-search-slot");
    if (fullPageSlot) {
      fullPageSlot.appendChild(wrapper);
      wrapper.classList.add("kuva-fullpage-search");
      if (wrapper.classList.contains("hidden")) {
        toggleIcon.click();
      }
      searchbar.focus();
      return;
    }

    const sidebar = document.getElementById("mdbook-sidebar");
    const scrollbox = sidebar && sidebar.querySelector(".sidebar-scrollbox");
    if (!sidebar || !scrollbox) {
      return;
    }

    sidebar.insertBefore(wrapper, scrollbox);

    function syncSearchHeight() {
      sidebar.style.setProperty("--kuva-search-height", wrapper.offsetHeight + "px");
    }
    syncSearchHeight();
    if (window.ResizeObserver) {
      new ResizeObserver(syncSearchHeight).observe(wrapper);
    }

    searchbar.addEventListener(
      "focus",
      () => {
        if (wrapper.classList.contains("hidden")) {
          toggleIcon.click();
        }
      },
      { once: true }
    );

    // Redirect the top-bar icon to the full-page search experience instead
    // of its default toggle-in-place behavior. Attached on the capture
    // phase with stopImmediatePropagation so searcher.js's own bubble-phase
    // click handler (bound in its own script, load order aside) never runs
    // — capture always precedes bubble regardless of attachment order.
    // `path_to_root` is a plain (non-`window`-scoped) global `const` set by
    // mdBook's own inline bootstrap script earlier in the page — every
    // classic <script> on the page, including this one, shares that
    // top-level scope, and searcher.js itself already relies on the same
    // pattern (`window.path_to_searchindex_js || path_to_root + '...'`).
    //
    // Only redirects on a *real* click (`e.isTrusted`) — the sidebar
    // searchbar's own first-focus handler above also calls
    // `toggleIcon.click()` to piggyback on mdBook's real lazy-load/init
    // logic, and that synthetic click must fall through to searcher.js's
    // normal handler untouched, or focusing the sidebar box would redirect
    // to search.html too instead of initializing in place.
    toggleIcon.addEventListener(
      "click",
      (e) => {
        if (!e.isTrusted) {
          return;
        }
        e.preventDefault();
        e.stopImmediatePropagation();
        window.location.href = path_to_root + "search.html";
      },
      true
    );
  }

  function init() {
    initCategoryCollapse();
    initSearchUI();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
