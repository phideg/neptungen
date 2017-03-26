[![Build Status](https://travis-ci.org/phideg/neptungen.svg?branch=master)](https://travis-ci.org/phideg/neptungen)

# neptungen
Yet another static Website Generator

# Why
Have you ever designed a static website for your club or for one of your relatives but you didn't want to maintain the contents for them? Well most static website generators are either dedicated to bloggers or they are too complicated to be used by non digital natives. 

The goal of neptungen is to be easy to use and minutes to set up even if you aren't an experienced web developer. 

But probably the real reason for neptungen was the desire to learn programming in Rust. So over time the code will hopefully get more idiomatic.

# Features
- Completely written in Rust
- CMS based on CommonMark
- Built-in gallery generator
- Built in FTP synchronization
- Customizable via [liquid](https://shopify.github.io/liquid/) templates

# Who uses neptungen
- [TSC Neptun Bruehl](http://tsc-neptun-bruehl.de)

# Getting started
Create a new root folder for your website
```bash
mkdir my_new_website
```
Each folders beneath that root folder represents a separate page of your website. The name of such a sub-folder will be used as a label in the navigation menu.
```bash
cd my_new_website
mkdir nav1
mkdir nav2
mkdir nav3
```

How can you add content to a page? Well, neptungen searches for markdown files and turns them into HTML which in turn is handed over to the page template via the `{{content}}` variable. Markdown files must have the *.md extension.

```bash
cd nav1
touch index.md
...
```

Open and edit the markdown files with the markdown editor of your choice.
Each folder should only contain one markdown file plus the images you reference in your markdown file.

The final step is to generate the site. Therefore `cd` into the root directory and run neptungen as follows:

```bash
cd ../../my_new_website
/path/to/your/neptungen_executable/neptungen build
``` 

By default the generated output can be found in the `_output` directory.

# Galleries
Galleries are as simple as normal pages. Create a `gallery` sub directory within any of your page directories. Copy or symlink all relevant images into it. Finally invoke the generator again as already described before.

By defailt the images will be resized to 800x600 pixels and the corresponding thumbs nails will be set to 90x90 pixels. Those settings can be specified via a separate configuration file `config.toml`. 

# config.toml
Neptungen can be tweaked with the `config.toml` file. It has to be put into the root directory of your project. [TOML](https://github.com/toml-lang/toml) aims to be a minimal configuration file format that's easy to read due to obvious semantics. Neptungen offers the following configuration options:

```toml
title = "Here you can give your home page a name"
template_dir = "_the_name_of_the_templates_directory"
output_dir = "_name_of_the_output_directory"
copy_dirs = [ "static_dir1", "static_dir2", ... , "static_dirN" ]

[gallery]
img_width = 600
img_height = 500
thumb_width = 90
thumb_height = 90

[sync_settings]
ftp_server = "my.ftpserver.com"
ftp_port = 21
ftp_user = "my_ftp_user"

```

# Customize your website
You don't want to use the built-in website theme? Just create a template directory and specify the path to that directory in your config.toml file (`template_dir = "my_template_folder"`).

Neptungen needs 2 templates:
 1. A page template named `page.liq`
 2. A gallery template named `gallery.liq`

Neptungen provides the following liquid variables:
 - __{{ title }}__ 
 - __{{ content }}__
 - __{{ root_dir }}__

 The {{root_dir}} variable contains a relative path to your web root depending on the depth of your site structure.
 The other variables are quite self explanatory. A little more complex is the `nav_items` collection. The following example template code show how you can use the collection to build a simple list based menu:

```html
<nav id="main-nav" role="navigation">
    <ul>
        <li>
            <a href="{{ root_dir }}index.html">Home</a>
        </li>
{% for item in nav_items %} 
    {% if item.menu_cmd == "OpenLevel" %}
        <li>
            <a href="#">{{ item.name }}</a>
            <ul>
    {% endif %} 
    {% if item.menu_cmd == "CloseLevel" %} 
        {% for i in (0..item.level_depth) %}
            </ul>
        </li>
        {% endfor %}
        <li>
            <a href="{{ root_dir }}{{ item.url }}">{{ item.name }}</a>
        </li>
    {% endif %} 
    {% if item.menu_cmd == "CloseOpenLevel" %} 
        {% for i in (0..item.level_depth) %}
            </ul>
        </li>
            {% endfor %}
        <li>
            <a href="#">{{ item.name }}</a>
            <ul>
    {% endif %} 
    {% if item.menu_cmd == "None" %}
        <li>
            <a href="{{ root_dir }}{{ item.url }}">{{ item.name }}</a>
        </li>
    {% endif %} 
{% endfor %}
    </ul>
</nav>
```

Please also have a look into the examples as they are always a good starting point.

# Alternatives
In case neptungen does not fulfill your requirements you might want to look into:
- [cobalt](https://github.com/cobalt-org/cobalt.rs)
- [gutenberg](https://github.com/Keats/gutenberg)