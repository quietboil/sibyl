// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="intro.html">Introduction</a></li><li class="chapter-item expanded "><a href="using.html"><strong aria-hidden="true">1.</strong> Getting Started</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="opt_features.html"><strong aria-hidden="true">1.1.</strong> Optional Features</a></li></ol></li><li class="chapter-item expanded "><a href="example_project.html"><strong aria-hidden="true">2.</strong> Example Project</a></li><li class="chapter-item expanded affix "><li class="part-title">User Guide</li><li class="chapter-item expanded "><a href="env.html"><strong aria-hidden="true">3.</strong> Environment</a></li><li class="chapter-item expanded "><a href="conn.html"><strong aria-hidden="true">4.</strong> Connections</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="conn_own.html"><strong aria-hidden="true">4.1.</strong> Connection Per Thread</a></li><li class="chapter-item expanded "><a href="session_pool.html"><strong aria-hidden="true">4.2.</strong> Session Pool</a></li><li class="chapter-item expanded "><a href="conn_pool.html"><strong aria-hidden="true">4.3.</strong> Connection Pool</a></li></ol></li><li class="chapter-item expanded "><a href="exec.html"><strong aria-hidden="true">5.</strong> SQL Statement Execution</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="args.html"><strong aria-hidden="true">5.1.</strong> Statement Arguments</a></li><li class="chapter-item expanded "><a href="slices.html"><strong aria-hidden="true">5.2.</strong> Slices as Arguments</a></li><li class="chapter-item expanded "><a href="nulls.html"><strong aria-hidden="true">5.3.</strong> NULLs</a></li><li class="chapter-item expanded "><a href="dyn_sql.html"><strong aria-hidden="true">5.4.</strong> Dynamic SQL</a></li></ol></li><li class="chapter-item expanded "><a href="encoding.html"><strong aria-hidden="true">6.</strong> Character Sets</a></li><li class="chapter-item expanded "><a href="odt.html"><strong aria-hidden="true">7.</strong> Oracle Data Types</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="odt/varchar.html"><strong aria-hidden="true">7.1.</strong> Varchar</a></li><li class="chapter-item expanded "><a href="odt/number.html"><strong aria-hidden="true">7.2.</strong> Number</a></li><li class="chapter-item expanded "><a href="odt/raw.html"><strong aria-hidden="true">7.3.</strong> RAW</a></li><li class="chapter-item expanded "><a href="odt/date.html"><strong aria-hidden="true">7.4.</strong> Date</a></li><li class="chapter-item expanded "><a href="odt/timestamp.html"><strong aria-hidden="true">7.5.</strong> Timestamp</a></li><li class="chapter-item expanded "><a href="odt/interval.html"><strong aria-hidden="true">7.6.</strong> Interval</a></li><li class="chapter-item expanded "><a href="odt/rowid.html"><strong aria-hidden="true">7.7.</strong> Row ID</a></li><li class="chapter-item expanded "><a href="odt/cursor.html"><strong aria-hidden="true">7.8.</strong> Cursor</a></li><li class="chapter-item expanded "><a href="odt/lobs.html"><strong aria-hidden="true">7.9.</strong> LOB</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Notes</li><li class="chapter-item expanded "><a href="issues.html"><strong aria-hidden="true">8.</strong> Known Issues</a></li><li class="chapter-item expanded "><a href="limits.html"><strong aria-hidden="true">9.</strong> Limitations</a></li><li class="chapter-item expanded "><a href="testing.html"><strong aria-hidden="true">10.</strong> Testing</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
