<!DOCTYPE HTML>
<html lang="en" class="light sidebar-visible" dir="ltr">
    <head>
        <!-- Book generated using mdBook -->
        <meta charset="UTF-8">
        <title>Chapter 1 - foobar</title>


        <!-- Custom HTML head -->

        <meta name="description" content="">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <meta name="theme-color" content="#ffffff">

        <link rel="icon" href="{{ root_dir }}_css/favicon.svg">
        <link rel="shortcut icon" href="{{ root_dir }}_css/favicon.png">
        <link rel="stylesheet" href="{{ root_dir }}_css/variables.css">
        <link rel="stylesheet" href="{{ root_dir }}_css/general.css">
        <link rel="stylesheet" href="{{ root_dir }}_css/chrome.css">
        <link rel="stylesheet" href="{{ root_dir }}_css/print.css" media="print">

        <!-- Fonts -->
        <link rel="stylesheet" href="{{ root_dir }}_fontawesome/css/font-awesome.css">
        <link rel="stylesheet" href="{{ root_dir }}_fonts/fonts.css">

        <!-- Highlight.js Stylesheets -->
        <link rel="stylesheet" href="{{ root_dir }}_scripts/highlight.css">
        <link rel="stylesheet" href="{{ root_dir }}_scripts/tomorrow-night.css">
        <link rel="stylesheet" href="{{ root_dir }}_scripts/ayu-highlight.css">

        <!-- Provide site root to javascript -->
        <script>
            var path_to_root = "";
            var default_theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "navy" : "light";
        </script>
        <!-- Start loading toc.js asap -->
        <script src="toc.js"></script>
    </head>
    <body>
    <div id="body-container">
        <!-- Work around some values being stored in localStorage wrapped in quotes -->
        <script>
            try {
                var theme = localStorage.getItem('mdbook-theme');
                var sidebar = localStorage.getItem('mdbook-sidebar');

                if (theme.startsWith('"') && theme.endsWith('"')) {
                    localStorage.setItem('mdbook-theme', theme.slice(1, theme.length - 1));
                }

                if (sidebar.startsWith('"') && sidebar.endsWith('"')) {
                    localStorage.setItem('mdbook-sidebar', sidebar.slice(1, sidebar.length - 1));
                }
            } catch (e) { }
        </script>

        <!-- Set the theme before any content is loaded, prevents flash -->
        <script>
            var theme;
            try { theme = localStorage.getItem('mdbook-theme'); } catch(e) { }
            if (theme === null || theme === undefined) { theme = default_theme; }
            const html = document.documentElement;
            html.classList.remove('light')
            html.classList.add(theme);
            html.classList.add("js");
        </script>

        <input type="checkbox" id="sidebar-toggle-anchor" class="hidden">

        <!-- Hide / unhide sidebar before it is displayed -->
        <script>
            var sidebar = null;
            var sidebar_toggle = document.getElementById("sidebar-toggle-anchor");
            if (document.body.clientWidth >= 1080) {
                try { sidebar = localStorage.getItem('mdbook-sidebar'); } catch(e) { }
                sidebar = sidebar || 'visible';
            } else {
                sidebar = 'hidden';
            }
            sidebar_toggle.checked = sidebar === 'visible';
            html.classList.remove('sidebar-visible');
            html.classList.add("sidebar-" + sidebar);
        </script>

        <nav id="sidebar" class="sidebar" aria-label="Table of contents">
            <!-- populated by js -->
            <mdbook-sidebar-scrollbox class="sidebar-scrollbox">
                <ol class="chapter">
                {% for item in nav_items %}
                {% if item.menu_cmd == "OpenLevel" %}
                    <li class = "chapter-item expanded">
                        {% if item.url.size > 0 %}
                        <a href="{{ root_dir }}{{ item.url }}">{{ item.name }}</a>
                        {% else %}
                        <a href="#"><strong>{{ item.name }}</strong></a>
                        {% endif %}
                    </li>
                    <li>
                        <ol class="section">
                {% elsif item.menu_cmd == "CloseOpenLevel" %}
                        </ol>
                    <li>
                    <li class = "chapter-item expanded">
                        {% if item.url.size > 0 %}
                        <a href="{{ root_dir }}{{ item.url }}">{{ item.name }}</a>
                        {% else %}
                        <a href="#"><strong>{{ item.name }}</strong></a>
                        {% endif %}
                    </li>
                    <li>
                        <ol class="section">
                {% elsif item.menu_cmd == "CloseLevel" %}
                        </ol>
                    <li>
                {% else %}
                    <li class = "chapter-item expanded">
                        <a href="{{ root_dir }}{{ item.url }}">{{ item.name }}</a>
                    </li>
                {% endif %}
                {% endfor %}
                </ol>
            </mdbook-sidebar-scrollbox>
            <noscript>
                <iframe class="sidebar-iframe-outer" src="toc.html"></iframe>
            </noscript>
            <div id="sidebar-resize-handle" class="sidebar-resize-handle">
                <div class="sidebar-resize-indicator"></div>
            </div>
        </nav>

        <div id="page-wrapper" class="page-wrapper">

            <div class="page">
                <div id="menu-bar-hover-placeholder"></div>
                <div id="menu-bar" class="menu-bar sticky">
                    <div class="left-buttons">
                        <label id="sidebar-toggle" class="icon-button" for="sidebar-toggle-anchor" title="Toggle Table of Contents" aria-label="Toggle Table of Contents" aria-controls="sidebar">
                            <i class="fa fa-bars"></i>
                        </label>
                        <button id="theme-toggle" class="icon-button" type="button" title="Change theme" aria-label="Change theme" aria-haspopup="true" aria-expanded="false" aria-controls="theme-list">
                            <i class="fa fa-paint-brush"></i>
                        </button>
                        <ul id="theme-list" class="theme-popup" aria-label="Themes" role="menu">
                            <li role="none"><button role="menuitem" class="theme" id="light">Light</button></li>
                            <li role="none"><button role="menuitem" class="theme" id="rust">Rust</button></li>
                            <li role="none"><button role="menuitem" class="theme" id="coal">Coal</button></li>
                            <li role="none"><button role="menuitem" class="theme" id="navy">Navy</button></li>
                            <li role="none"><button role="menuitem" class="theme" id="ayu">Ayu</button></li>
                        </ul>
                        <!--
                        <button id="search-toggle" class="icon-button" type="button" title="Search. (Shortkey: s)" aria-label="Toggle Searchbar" aria-expanded="false" aria-keyshortcuts="S" aria-controls="searchbar">
                            <i class="fa fa-search"></i>
                        </button>
                        -->
                    </div>

                    <h1 class="menu-title">{{title}}</h1>

                    <div class="right-buttons">
                        <a href="print.html" title="Print this book" aria-label="Print this book">
                            <i id="print-button" class="fa fa-print"></i>
                        </a>

                    </div>
                </div>

                <div id="search-wrapper" class="hidden">
                    <form id="searchbar-outer" class="searchbar-outer">
                        <input type="search" id="searchbar" name="searchbar" placeholder="Search this book ..." aria-controls="searchresults-outer" aria-describedby="searchresults-header">
                    </form>
                    <div id="searchresults-outer" class="searchresults-outer hidden">
                        <div id="searchresults-header" class="searchresults-header"></div>
                        <ul id="searchresults">
                        </ul>
                    </div>
                </div>

                <!-- Apply ARIA attributes after the sidebar and the sidebar toggle button are added to the DOM -->
                <script>
                    document.getElementById('sidebar-toggle').setAttribute('aria-expanded', sidebar === 'visible');
                    document.getElementById('sidebar').setAttribute('aria-hidden', sidebar !== 'visible');
                    Array.from(document.querySelectorAll('#sidebar a')).forEach(function(link) {
                        link.setAttribute('tabIndex', sidebar === 'visible' ? 0 : -1);
                    });
                </script>

                <div id="content" class="content">
                    <main>
                        {{ content }}
                    </main>

                    <nav class="nav-wrapper" aria-label="Page navigation">
                        <!-- Mobile navigation buttons -->


                        <div style="clear: both"></div>
                    </nav>
                </div>
            </div>

            <nav class="nav-wide-wrapper" aria-label="Page navigation">

            </nav>

        </div>

        <script>
            window.playground_copyable = true;
        </script>

        <script src="{{ root_dir }}_scripts/elasticlunr.min.js"></script>
        <script src="{{ root_dir }}_scripts/mark.min.js"></script>
        <script src="{{ root_dir }}_scripts/searcher.js"></script>

        <script src="{{ root_dir }}_scripts/clipboard.min.js"></script>
        <script src="{{ root_dir }}_scripts/highlight.js"></script>
        <script src="{{ root_dir }}_scripts/book.js"></script>
    </div>
    </body>
</html>
